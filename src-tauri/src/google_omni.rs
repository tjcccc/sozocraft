use crate::{google_veo::GoogleVeoClient, models::VideoGenerationRequest};
use reqwest::{Client, Proxy, Url};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};

const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub struct GoogleOmniClient {
    api_key: String,
    base_url: String,
    client: Client,
    downloader: GoogleVeoClient,
}

pub enum OmniStatus {
    Pending,
    Done(String),
    Failed,
}

impl GoogleOmniClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout: u64,
    ) -> Result<Self, String> {
        let downloader = GoogleVeoClient::new(
            api_key.clone(),
            base_url.clone(),
            proxy_url.clone(),
            timeout,
        )?;
        // URI delivery is synchronous; generation can take several minutes.
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(timeout.max(900)))
            .redirect(reqwest::redirect::Policy::none());
        if let Some(proxy) = proxy_url.filter(|value| !value.trim().is_empty()) {
            builder = builder.proxy(Proxy::all(proxy.trim()).map_err(|error| error.to_string())?);
        }
        Ok(Self {
            api_key,
            base_url: base_url
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string())
                .trim_end_matches('/')
                .to_string(),
            client: builder.build().map_err(|error| error.to_string())?,
            downloader,
        })
    }

    pub async fn start(&self, request: &VideoGenerationRequest) -> Result<String, String> {
        let response = self
            .client
            .post(format!("{}/interactions", self.base_url))
            .header("x-goog-api-key", &self.api_key)
            .json(&build_body(request))
            .send()
            .await
            .map_err(|_| "Gemini Omni generation request failed.".to_string())?;
        let body = response_json(response).await?;
        parse_file_id(&body)
    }

    pub async fn poll(&self, file_id: &str) -> Result<OmniStatus, String> {
        validate_file_id(file_id)?;
        let response = self
            .client
            .get(format!("{}/files/{file_id}", self.base_url))
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await
            .map_err(|_| "Gemini Omni file status request failed.".to_string())?;
        let body = response_json(response).await?;
        match body.get("state").and_then(Value::as_str) {
            Some("ACTIVE") => Ok(OmniStatus::Done(format!(
                "{DEFAULT_ENDPOINT}/files/{file_id}:download?alt=media"
            ))),
            Some("PROCESSING") => Ok(OmniStatus::Pending),
            Some("FAILED") => Ok(OmniStatus::Failed),
            _ => Err("Gemini Omni returned an unknown file status.".to_string()),
        }
    }

    pub async fn download_to(&self, url: &str, path: &Path) -> Result<u64, String> {
        // Reuse the Google URL allowlist, bounded streaming, and MP4 validation.
        self.downloader.download_to(url, path).await
    }
}

fn build_body(request: &VideoGenerationRequest) -> Value {
    let mut input = Vec::new();
    let mut roles = Vec::new();
    for (image, role) in request
        .starting_image
        .iter()
        .map(|image| (image, "FIRST_FRAME"))
        .chain(
            request
                .ending_image
                .iter()
                .map(|image| (image, "LAST_FRAME")),
        )
    {
        input.push(
            json!({"type": "image", "mime_type": image.mime_type, "data": image.data.trim()}),
        );
        roles.push(format!("<{role}>@Image{}", input.len()));
    }
    let sources = if roles.is_empty() {
        String::new()
    } else {
        format!("[# Sources {}] ", roles.join(" "))
    };
    roles.clear();
    for (index, image) in request
        .reference_images
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        input.push(
            json!({"type": "image", "mime_type": image.mime_type, "data": image.data.trim()}),
        );
        roles.push(format!("<IMAGE_REF_{index}>@Image{}", input.len()));
    }
    let references = if roles.is_empty() {
        String::new()
    } else {
        format!("[# References {}] ", roles.join(" "))
    };
    input.push(
        json!({"type": "text", "text": format!("{sources}{references}{}", request.prompt.trim())}),
    );
    json!({
        "model": request.model,
        "input": input,
        "background": false, "store": false, "stream": false,
        "response_format": {
            "type": "video", "delivery": "uri",
            "aspect_ratio": request.options.aspect_ratio,
            "resolution": request.options.resolution,
            "duration": format!("{}s", request.options.duration),
        }
    })
}

async fn response_json(mut response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Gemini Omni response could not be read.")?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(
                "Gemini Omni response exceeded 2 MB; URI delivery is required.".to_string(),
            );
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        // Never log echoed media, prompts, or credentials from an API error body.
        return Err(format!(
            "Gemini Omni returned HTTP {status}. Check API access and generation settings."
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_| "Gemini Omni returned invalid JSON.".to_string())
}

fn parse_file_id(body: &Value) -> Result<String, String> {
    if body.get("status").and_then(Value::as_str) != Some("completed") {
        return Err("Gemini Omni did not complete video generation.".to_string());
    }
    let video = body
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|step| step.get("type").and_then(Value::as_str) == Some("model_output"))
        .flat_map(|step| {
            step.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|content| content.get("type").and_then(Value::as_str) == Some("video"))
        .ok_or("Gemini Omni completed without a video.")?;
    let uri = video
        .get("uri")
        .and_then(Value::as_str)
        .ok_or("Gemini Omni returned no video URI.")?;
    let url = Url::parse(uri).map_err(|_| "Gemini Omni returned an invalid video URI.")?;
    if url.scheme() != "https"
        || url.host_str() != Some("generativelanguage.googleapis.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Gemini Omni returned an unsafe video URI.".to_string());
    }
    let file_id = url
        .path()
        .strip_prefix("/v1beta/files/")
        .ok_or("Gemini Omni returned an invalid file path.")?
        .trim_end_matches(":download");
    validate_file_id(file_id)?;
    Ok(file_id.to_string())
}

fn validate_file_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 256
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("Gemini Omni returned an invalid file ID.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_uri_delivery_with_explicit_frame_roles_and_duration() {
        use crate::models::{
            ReferenceImageInput, VideoGenerationOptions, VideoInputMode, VideoProvider,
        };
        let image = ReferenceImageInput {
            name: "frame.png".to_string(),
            mime_type: "image/png".to_string(),
            data: " YWJj ".to_string(),
            asset_id: None,
        };
        let mut request = VideoGenerationRequest {
            task_id: None,
            provider: VideoProvider::GoogleVeo,
            model: "gemini-omni-1.1-flash".to_string(),
            prompt: "Animate".to_string(),
            prompt_snapshot: None,
            input_mode: VideoInputMode::Frames,
            starting_image: Some(image.clone()),
            ending_image: Some(image.clone()),
            reference_images: None,
            options: VideoGenerationOptions {
                duration: 5,
                aspect_ratio: "9:16".to_string(),
                resolution: "4k".to_string(),
                generate_audio: None,
            },
        };
        let body = build_body(&request);
        assert_eq!(
            body["response_format"],
            json!({"type":"video","delivery":"uri","duration":"5s","aspect_ratio":"9:16","resolution":"4k"})
        );
        assert_eq!(body["input"][0]["data"], "YWJj");
        assert_eq!(
            body["input"][2]["text"],
            "[# Sources <FIRST_FRAME>@Image1 <LAST_FRAME>@Image2] Animate"
        );
        assert_eq!(body["store"], false);
        request.starting_image = None;
        request.ending_image = None;
        request.reference_images = Some(vec![image]);
        assert_eq!(
            build_body(&request)["input"][1]["text"],
            "[# References <IMAGE_REF_0>@Image1] Animate"
        );
    }

    #[test]
    fn parses_only_model_video_and_rejects_unsafe_file_uris() {
        let body = |uri: &str| {
            json!({"status":"completed", "steps":[
            {"type":"user_input","content":[{"type":"video","uri":"https://evil.invalid/input"}]},
            {"type":"model_output","content":[{"type":"video","uri":uri}]}]})
        };
        assert_eq!(
            parse_file_id(&body(
                "https://generativelanguage.googleapis.com/v1beta/files/abc-123:download?alt=media"
            ))
            .unwrap(),
            "abc-123"
        );
        for uri in [
            "https://evil.invalid/v1beta/files/abc",
            "https://generativelanguage.googleapis.com/v1beta/files/a/b",
            "http://generativelanguage.googleapis.com/v1beta/files/abc",
            "https://generativelanguage.googleapis.com/v1beta/files/%2e%2e",
        ] {
            assert!(parse_file_id(&body(uri)).is_err());
        }
        assert!(parse_file_id(&json!({"status":"completed","steps":[]})).is_err());
    }
}
