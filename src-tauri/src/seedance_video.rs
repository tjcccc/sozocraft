use crate::models::VideoGenerationRequest;
use reqwest::{redirect, Client, Proxy, StatusCode, Url};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

const DEFAULT_ENDPOINT: &str = "https://ark.cn-beijing.volces.com/api/v3";
const MAX_JSON_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_VIDEO_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SeedanceVideoClient {
    api_key: String,
    base_url: String,
    client: Client,
    download_client: Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedanceVideoStatus {
    Pending,
    Done { url: String, duration: Option<u8> },
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct SeedanceVideoPollResponse {
    pub status: SeedanceVideoStatus,
    pub metadata: Value,
}

impl SeedanceVideoClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Volcengine Ark API key is missing.".to_string());
        }
        let base_url = base_url
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string())
            .trim_end_matches('/')
            .to_string();
        validate_api_base_url(&base_url)?;
        let timeout = Duration::from_secs(timeout_seconds.max(10));
        let mut client_builder = Client::builder().timeout(timeout);
        let mut download_builder =
            Client::builder()
                .timeout(timeout)
                .redirect(redirect::Policy::custom(|attempt| {
                    if is_volcengine_video_url(attempt.url()) {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }));
        if let Some(proxy_url) = proxy_url.filter(|value| !value.trim().is_empty()) {
            let proxy = Proxy::all(proxy_url.trim()).map_err(|error| error.to_string())?;
            client_builder = client_builder.proxy(proxy.clone());
            download_builder = download_builder.proxy(proxy);
        }
        Ok(Self {
            api_key,
            base_url,
            client: client_builder.build().map_err(|error| error.to_string())?,
            download_client: download_builder
                .build()
                .map_err(|error| error.to_string())?,
        })
    }

    pub async fn start(&self, request: &VideoGenerationRequest) -> Result<String, String> {
        let response = self
            .client
            .post(format!("{}/contents/generations/tasks", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&build_start_body(request))
            .send()
            .await
            .map_err(|error| format!("Volcengine Seedance request failed: {error}"))?;
        let (status, body) = response_json(response).await?;
        if !status.is_success() {
            return Err(format!(
                "Volcengine Ark returned an error ({status}): {}",
                sanitized_text(&sanitized_metadata(&body).to_string())
            ));
        }
        parse_start_response(&body)
    }

    pub async fn poll(&self, task_id: &str) -> Result<SeedanceVideoPollResponse, String> {
        validate_task_id(task_id)?;
        let response = self
            .client
            .get(format!(
                "{}/contents/generations/tasks/{task_id}",
                self.base_url
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|error| format!("Volcengine Seedance poll failed: {error}"))?;
        let (status, body) = response_json(response).await?;
        if !status.is_success() {
            return Err(format!(
                "Volcengine Ark returned an error ({status}): {}",
                sanitized_text(&sanitized_metadata(&body).to_string())
            ));
        }
        parse_poll_response(body)
    }

    pub async fn download_to(&self, video_url: &str, output_path: &Path) -> Result<u64, String> {
        let url = Url::parse(video_url)
            .map_err(|_| "Volcengine Ark returned an unsafe video URL.".to_string())?;
        if !is_volcengine_video_url(&url) {
            return Err("Volcengine Ark returned an unsafe video URL.".to_string());
        }
        let mut response = self
            .download_client
            .get(url)
            .send()
            .await
            .map_err(|error| format!("Seedance video download failed: {error}"))?;
        if !is_volcengine_video_url(response.url()) {
            return Err("Volcengine Ark returned an unsafe video URL.".to_string());
        }
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Seedance video download failed ({status}): {}",
                sanitized_text(&body)
            ));
        }
        write_mp4_response(&mut response, output_path).await
    }
}

fn build_start_body(request: &VideoGenerationRequest) -> Value {
    let mut content = vec![json!({ "type": "text", "text": request.prompt.trim() })];
    if let Some(image) = &request.starting_image {
        content.push(image_content(image, "first_frame"));
    }
    if let Some(image) = &request.ending_image {
        content.push(image_content(image, "last_frame"));
    }
    if let Some(images) = request.reference_images.as_ref() {
        content.extend(
            images
                .iter()
                .map(|image| image_content(image, "reference_image")),
        );
    }
    json!({
        "model": request.model,
        "content": content,
        "duration": request.options.duration,
        "ratio": request.options.aspect_ratio,
        "resolution": request.options.resolution,
        "generate_audio": request.options.generate_audio.unwrap_or(true),
        "watermark": false,
    })
}

fn image_content(image: &crate::models::ReferenceImageInput, role: &str) -> Value {
    if let Some(asset_id) = image.asset_id.as_deref() {
        return json!({
            "type": "image_url",
            "image_url": { "url": format!("asset://{asset_id}") },
            "role": role,
        });
    }
    json!({
        "type": "image_url",
        "image_url": {
            "url": format!(
                "data:{};base64,{}",
                supported_input_mime(&image.mime_type),
                image.data.trim()
            )
        },
        "role": role,
    })
}

fn supported_input_mime(mime_type: &str) -> &str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        _ => "image/png",
    }
}

fn parse_start_response(value: &Value) -> Result<String, String> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Volcengine Ark returned a video task without an id.".to_string())?;
    validate_task_id(id)?;
    Ok(id.to_string())
}

fn parse_poll_response(value: Value) -> Result<SeedanceVideoPollResponse, String> {
    let metadata = sanitized_metadata(&value);
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| "Volcengine Ark returned a video task without a status.".to_string())?;
    let status = match status.as_str() {
        "queued" | "running" => SeedanceVideoStatus::Pending,
        "succeeded" => {
            let url = value
                .pointer("/content/video_url")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|url| !url.is_empty())
                .ok_or_else(|| "Seedance completed without a video URL.".to_string())?;
            let parsed = Url::parse(url)
                .map_err(|_| "Volcengine Ark returned an unsafe video URL.".to_string())?;
            if !is_volcengine_video_url(&parsed) {
                return Err("Volcengine Ark returned an unsafe video URL.".to_string());
            }
            let duration = value
                .get("duration")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok());
            SeedanceVideoStatus::Done {
                url: url.to_string(),
                duration,
            }
        }
        "failed" => SeedanceVideoStatus::Failed(response_error_message(&value)),
        "cancelled" => SeedanceVideoStatus::Cancelled,
        other => {
            return Err(format!(
                "Volcengine Ark returned unsupported status `{other}`."
            ))
        }
    };
    Ok(SeedanceVideoPollResponse { status, metadata })
}

async fn response_json(response: reqwest::Response) -> Result<(StatusCode, Value), String> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|value| value > MAX_JSON_RESPONSE_BYTES as u64)
    {
        return Err("Volcengine Ark response exceeded 1 MB.".to_string());
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() > MAX_JSON_RESPONSE_BYTES {
        return Err("Volcengine Ark response exceeded 1 MB.".to_string());
    }
    let value = serde_json::from_slice(&bytes)
        .map_err(|_| "Volcengine Ark returned a non-JSON response.".to_string())?;
    Ok((status, value))
}

async fn write_mp4_response(
    response: &mut reqwest::Response,
    output_path: &Path,
) -> Result<u64, String> {
    if response
        .content_length()
        .is_some_and(|value| value > MAX_VIDEO_BYTES as u64)
    {
        return Err("Seedance returned a video larger than 1 GB.".to_string());
    }
    let mut file = tokio::fs::File::create(output_path)
        .await
        .map_err(|error| format!("Failed to create video output: {error}"))?;
    let mut total = 0usize;
    let mut signature = Vec::with_capacity(12);
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => {
                let _ = tokio::fs::remove_file(output_path).await;
                return Err(format!("Seedance video download failed: {error}"));
            }
        };
        total = total
            .checked_add(chunk.len())
            .ok_or("Seedance video is too large.")?;
        if total > MAX_VIDEO_BYTES {
            let _ = tokio::fs::remove_file(output_path).await;
            return Err("Seedance returned a video larger than 1 GB.".to_string());
        }
        if signature.len() < 12 {
            signature.extend_from_slice(&chunk[..chunk.len().min(12 - signature.len())]);
        }
        if let Err(error) = file.write_all(&chunk).await {
            let _ = tokio::fs::remove_file(output_path).await;
            return Err(format!("Failed to write video output: {error}"));
        }
    }
    if let Err(error) = file.flush().await {
        let _ = tokio::fs::remove_file(output_path).await;
        return Err(format!("Failed to flush video output: {error}"));
    }
    if signature.len() < 8 || signature.get(4..8) != Some(b"ftyp") {
        let _ = tokio::fs::remove_file(output_path).await;
        return Err("Seedance returned data that is not an MP4 video.".to_string());
    }
    Ok(total as u64)
}

fn validate_api_base_url(value: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| "Invalid Volcengine Ark base URL.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Invalid Volcengine Ark base URL.".to_string());
    }
    Ok(())
}

fn validate_task_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Volcengine Ark returned an invalid video task id.".to_string());
    }
    Ok(())
}

fn is_volcengine_video_url(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    url.host_str().is_some_and(|host| {
        host == "volces.com"
            || host.ends_with(".volces.com")
            || host == "volcengine.com"
            || host.ends_with(".volcengine.com")
    })
}

fn response_error_message(value: &Value) -> String {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(sanitized_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Seedance video generation failed.".to_string())
}

fn sanitized_metadata(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter_map(|(key, value)| {
                    if ["authorization", "prompt", "text", "url", "video_url"]
                        .contains(&key.to_ascii_lowercase().as_str())
                    {
                        None
                    } else {
                        Some((key.clone(), sanitized_metadata(value)))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(sanitized_metadata).collect()),
        _ => value.clone(),
    }
}

fn sanitized_text(value: &str) -> String {
    value
        .replace(['\r', '\n'], " ")
        .chars()
        .take(2_000)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ReferenceImageInput, VideoGenerationOptions, VideoInputMode, VideoProvider,
    };

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            task_id: None,
            provider: VideoProvider::Seedance,
            model: "doubao-seedance-2-0-260128".to_string(),
            prompt: "A dancer crosses a rain-soaked stage".to_string(),
            prompt_snapshot: None,
            input_mode: VideoInputMode::Image,
            starting_image: Some(ReferenceImageInput {
                name: "frame.png".to_string(),
                mime_type: "image/png".to_string(),
                data: "abc".to_string(),
                asset_id: None,
            }),
            ending_image: None,
            reference_images: None,
            options: VideoGenerationOptions {
                duration: 15,
                aspect_ratio: "21:9".to_string(),
                resolution: "1080p".to_string(),
                generate_audio: Some(true),
            },
        }
    }

    #[test]
    fn builds_seedance_content_roles_and_options() {
        let body = build_start_body(&request());
        assert_eq!(body.pointer("/content/1/role"), Some(&json!("first_frame")));
        assert_eq!(body.pointer("/generate_audio"), Some(&json!(true)));
        assert_eq!(body.pointer("/watermark"), Some(&json!(false)));
    }

    #[test]
    fn builds_seedance_start_and_end_frame_roles() {
        let mut value = request();
        value.input_mode = VideoInputMode::Frames;
        value.ending_image = Some(ReferenceImageInput {
            name: "ending.png".to_string(),
            mime_type: "image/png".to_string(),
            data: "def".to_string(),
            asset_id: None,
        });

        let body = build_start_body(&value);

        assert_eq!(body.pointer("/content/1/role"), Some(&json!("first_frame")));
        assert_eq!(body.pointer("/content/2/role"), Some(&json!("last_frame")));
    }

    #[test]
    fn sends_ark_asset_reference_without_inline_data() {
        let mut value = request();
        value.input_mode = VideoInputMode::Reference;
        value.starting_image = None;
        value.reference_images = Some(vec![ReferenceImageInput {
            name: "portrait".to_string(),
            mime_type: "image/ark-asset".to_string(),
            data: String::new(),
            asset_id: Some("asset-20260801-example".to_string()),
        }]);

        let body = build_start_body(&value);

        assert_eq!(
            body.pointer("/content/1/image_url/url"),
            Some(&json!("asset://asset-20260801-example"))
        );
        assert_eq!(
            body.pointer("/content/1/role"),
            Some(&json!("reference_image"))
        );
    }

    #[test]
    fn parses_task_lifecycle() {
        assert_eq!(
            parse_start_response(&json!({ "id": "cgt-123" })).unwrap(),
            "cgt-123"
        );
        let poll = parse_poll_response(json!({
            "status": "succeeded",
            "duration": 15,
            "content": { "video_url": "https://example.tos-cn-beijing.volces.com/video.mp4" }
        }))
        .unwrap();
        assert!(matches!(
            poll.status,
            SeedanceVideoStatus::Done {
                duration: Some(15),
                ..
            }
        ));
        assert!(poll.metadata.pointer("/content/video_url").is_none());
    }
}
