use crate::models::{VideoGenerationRequest, VideoInputMode};
use reqwest::{redirect, Client, Proxy, StatusCode, Url};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta";
const MAX_JSON_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_VIDEO_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct GoogleVeoClient {
    api_key: String,
    base_url: String,
    client: Client,
    download_client: Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleVeoStatus {
    Pending,
    Done { url: String },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct GoogleVeoPollResponse {
    pub status: GoogleVeoStatus,
    pub metadata: Value,
}

impl GoogleVeoClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Gemini API key is missing.".to_string());
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
                    if is_google_video_url(attempt.url()) {
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
            .post(format!(
                "{}/models/{}:predictLongRunning",
                self.base_url, request.model
            ))
            .header("x-goog-api-key", &self.api_key)
            .json(&build_start_body(request))
            .send()
            .await
            .map_err(|error| format!("Google Veo request failed: {error}"))?;
        let (status, body) = response_json(response).await?;
        if !status.is_success() {
            return Err(format!(
                "Google Veo returned an error ({status}): {}",
                sanitized_text(&sanitized_metadata(&body).to_string())
            ));
        }
        parse_start_response(&body)
    }

    pub async fn poll(&self, operation_name: &str) -> Result<GoogleVeoPollResponse, String> {
        validate_operation_name(operation_name)?;
        let response = self
            .client
            .get(format!("{}/{}", self.base_url, operation_name))
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await
            .map_err(|error| format!("Google Veo poll failed: {error}"))?;
        let (status, body) = response_json(response).await?;
        if !status.is_success() {
            return Err(format!(
                "Google Veo returned an error ({status}): {}",
                sanitized_text(&sanitized_metadata(&body).to_string())
            ));
        }
        parse_poll_response(body)
    }

    pub async fn download_to(&self, video_url: &str, output_path: &Path) -> Result<u64, String> {
        let url = Url::parse(video_url).map_err(|_| "Google Veo returned an unsafe video URL.")?;
        if !is_google_video_url(&url) {
            return Err("Google Veo returned an unsafe video URL.".to_string());
        }
        let mut response = self
            .download_client
            .get(url)
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await
            .map_err(|error| format!("Google Veo video download failed: {error}"))?;
        if !is_google_video_url(response.url()) {
            return Err("Google Veo returned an unsafe video URL.".to_string());
        }
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Google Veo video download failed ({status}): {}",
                sanitized_text(&body)
            ));
        }
        write_mp4_response(&mut response, output_path).await
    }
}

fn build_start_body(request: &VideoGenerationRequest) -> Value {
    let mut instance = json!({ "prompt": request.prompt.trim() });
    if let Some(image) = &request.starting_image {
        instance["image"] = encoded_image(image);
    }
    if let Some(image) = &request.ending_image {
        instance["lastFrame"] = encoded_image(image);
    }
    if let Some(images) = request
        .reference_images
        .as_ref()
        .filter(|items| !items.is_empty())
    {
        instance["referenceImages"] = Value::Array(
            images
                .iter()
                .map(|image| json!({ "image": encoded_image(image), "referenceType": "asset" }))
                .collect(),
        );
    }
    json!({
        "instances": [instance],
        "parameters": {
            "aspectRatio": request.options.aspect_ratio,
            "durationSeconds": request.options.duration,
            "personGeneration": person_generation(request.input_mode),
            "resolution": request.options.resolution,
        }
    })
}

fn person_generation(input_mode: VideoInputMode) -> &'static str {
    match input_mode {
        VideoInputMode::Text => "allow_all",
        VideoInputMode::Image | VideoInputMode::Frames | VideoInputMode::Reference => "allow_adult",
    }
}

fn encoded_image(image: &crate::models::ReferenceImageInput) -> Value {
    json!({
        "mimeType": supported_input_mime(&image.mime_type),
        "bytesBase64Encoded": image.data.trim(),
    })
}

fn supported_input_mime(mime_type: &str) -> &str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        _ => "image/png",
    }
}

fn parse_start_response(value: &Value) -> Result<String, String> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Google Veo returned an invalid operation without a name.".to_string())?;
    validate_operation_name(name)?;
    Ok(name.to_string())
}

fn parse_poll_response(value: Value) -> Result<GoogleVeoPollResponse, String> {
    let metadata = sanitized_metadata(&value);
    if value.get("done").and_then(Value::as_bool) != Some(true) {
        return Ok(GoogleVeoPollResponse {
            status: GoogleVeoStatus::Pending,
            metadata,
        });
    }
    if let Some(error) = value.get("error") {
        return Ok(GoogleVeoPollResponse {
            status: GoogleVeoStatus::Failed(response_error_message(error)),
            metadata,
        });
    }
    if let Some(message) = filtered_video_message(&value) {
        return Ok(GoogleVeoPollResponse {
            status: GoogleVeoStatus::Failed(message),
            metadata,
        });
    }
    let uri = value
        .pointer("/response/generateVideoResponse/generatedSamples/0/video/uri")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|uri| !uri.is_empty())
        .ok_or_else(|| "Google Veo completed without a video URI.".to_string())?;
    let url = Url::parse(uri).map_err(|_| "Google Veo returned an unsafe video URL.")?;
    if !is_google_video_url(&url) {
        return Err("Google Veo returned an unsafe video URL.".to_string());
    }
    Ok(GoogleVeoPollResponse {
        status: GoogleVeoStatus::Done {
            url: uri.to_string(),
        },
        metadata,
    })
}

fn filtered_video_message(value: &Value) -> Option<String> {
    let response = value.pointer("/response/generateVideoResponse")?;
    let filtered_count = response
        .get("raiMediaFilteredCount")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let reasons = response
        .get("raiMediaFilteredReasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(sanitized_text)
        .filter(|reason| !reason.trim().is_empty())
        .collect::<Vec<_>>();
    if filtered_count == 0 && reasons.is_empty() {
        return None;
    }
    if reasons.is_empty() {
        return Some(
            "Google Veo filtered the generated video under its Responsible AI policies."
                .to_string(),
        );
    }
    Some(sanitized_text(&format!(
        "Google Veo filtered the generated video: {}",
        reasons.join(" ")
    )))
}

async fn response_json(response: reqwest::Response) -> Result<(StatusCode, Value), String> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|value| value > MAX_JSON_RESPONSE_BYTES as u64)
    {
        return Err("Google Veo response exceeded 1 MB.".to_string());
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() > MAX_JSON_RESPONSE_BYTES {
        return Err("Google Veo response exceeded 1 MB.".to_string());
    }
    let value = serde_json::from_slice(&bytes)
        .map_err(|_| "Google Veo returned a non-JSON response.".to_string())?;
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
        return Err("Google Veo returned a video larger than 1 GB.".to_string());
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
                return Err(format!("Google Veo video download failed: {error}"));
            }
        };
        total = total
            .checked_add(chunk.len())
            .ok_or("Google Veo video is too large.")?;
        if total > MAX_VIDEO_BYTES {
            let _ = tokio::fs::remove_file(output_path).await;
            return Err("Google Veo returned a video larger than 1 GB.".to_string());
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
        return Err("Google Veo returned data that is not an MP4 video.".to_string());
    }
    Ok(total as u64)
}

fn validate_api_base_url(value: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| "Invalid Google Gemini base URL.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Invalid Google Gemini base URL.".to_string());
    }
    Ok(())
}

fn validate_operation_name(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 512
        || value.starts_with('/')
        || value.contains(['?', '#', '\\'])
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("Google Veo returned an invalid operation name.".to_string());
    }
    Ok(())
}

fn is_google_video_url(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    url.host_str().is_some_and(|host| {
        host == "googleapis.com"
            || host.ends_with(".googleapis.com")
            || host == "googleusercontent.com"
            || host.ends_with(".googleusercontent.com")
    })
}

fn response_error_message(value: &Value) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .map(sanitized_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Google Veo video generation failed.".to_string())
}

fn sanitized_metadata(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter_map(|(key, value)| {
                    if [
                        "data",
                        "bytesbase64encoded",
                        "prompt",
                        "uri",
                        "x-goog-api-key",
                    ]
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
            provider: VideoProvider::GoogleVeo,
            model: "veo-3.1-generate-preview".to_string(),
            prompt: "A paper boat crosses a moonlit pond".to_string(),
            prompt_snapshot: None,
            input_mode: VideoInputMode::Text,
            starting_image: None,
            ending_image: None,
            reference_images: None,
            options: VideoGenerationOptions {
                duration: 8,
                aspect_ratio: "16:9".to_string(),
                resolution: "720p".to_string(),
                generate_audio: None,
            },
        }
    }

    #[test]
    fn builds_sdk_compatible_reference_image_shape() {
        let mut value = request();
        value.input_mode = VideoInputMode::Reference;
        value.reference_images = Some(vec![ReferenceImageInput {
            name: "boat.png".to_string(),
            mime_type: "image/png".to_string(),
            data: "abc".to_string(),
            asset_id: None,
        }]);
        let body = build_start_body(&value);
        assert_eq!(
            body.pointer("/instances/0/referenceImages/0/referenceType"),
            Some(&json!("asset"))
        );
        assert_eq!(
            body.pointer("/instances/0/referenceImages/0/image/mimeType"),
            Some(&json!("image/png"))
        );
        assert_eq!(
            body.pointer("/instances/0/referenceImages/0/image/bytesBase64Encoded"),
            Some(&json!("abc"))
        );
        assert!(body
            .pointer("/instances/0/referenceImages/0/image/inlineData")
            .is_none());
        assert_eq!(body.pointer("/parameters/durationSeconds"), Some(&json!(8)));
        assert_eq!(
            body.pointer("/parameters/personGeneration"),
            Some(&json!("allow_adult"))
        );
    }

    #[test]
    fn uses_documented_person_generation_for_each_input_mode() {
        for (input_mode, expected) in [
            (VideoInputMode::Text, "allow_all"),
            (VideoInputMode::Image, "allow_adult"),
            (VideoInputMode::Frames, "allow_adult"),
            (VideoInputMode::Reference, "allow_adult"),
        ] {
            let mut value = request();
            value.input_mode = input_mode;
            assert_eq!(
                build_start_body(&value).pointer("/parameters/personGeneration"),
                Some(&json!(expected))
            );
        }
    }

    #[test]
    fn builds_sdk_compatible_starting_image_shape() {
        let mut value = request();
        value.input_mode = VideoInputMode::Image;
        value.starting_image = Some(ReferenceImageInput {
            name: "boat.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            data: " abc ".to_string(),
            asset_id: None,
        });

        let body = build_start_body(&value);

        assert_eq!(
            body.pointer("/instances/0/image"),
            Some(&json!({
                "mimeType": "image/jpeg",
                "bytesBase64Encoded": "abc",
            }))
        );
        assert!(body.pointer("/instances/0/image/inlineData").is_none());
    }

    #[test]
    fn builds_sdk_compatible_start_and_end_frames() {
        let mut value = request();
        value.input_mode = VideoInputMode::Frames;
        value.starting_image = Some(ReferenceImageInput {
            name: "start.png".to_string(),
            mime_type: "image/png".to_string(),
            data: "start".to_string(),
            asset_id: None,
        });
        value.ending_image = Some(ReferenceImageInput {
            name: "end.png".to_string(),
            mime_type: "image/png".to_string(),
            data: "end".to_string(),
            asset_id: None,
        });

        let body = build_start_body(&value);

        assert_eq!(
            body.pointer("/instances/0/image/bytesBase64Encoded"),
            Some(&json!("start"))
        );
        assert_eq!(
            body.pointer("/instances/0/lastFrame/bytesBase64Encoded"),
            Some(&json!("end"))
        );
    }

    #[test]
    fn strips_encoded_images_from_metadata() {
        let metadata = sanitized_metadata(&json!({
            "bytesBase64Encoded": "secret image bytes",
            "nested": { "bytesBase64Encoded": "more bytes", "state": "RUNNING" }
        }));

        assert_eq!(metadata, json!({ "nested": { "state": "RUNNING" } }));
    }

    #[test]
    fn parses_operation_and_completed_video() {
        assert_eq!(
            parse_start_response(&json!({ "name": "models/veo/operations/abc" })).unwrap(),
            "models/veo/operations/abc"
        );
        let poll = parse_poll_response(json!({
            "done": true,
            "response": { "generateVideoResponse": { "generatedSamples": [{
                "video": { "uri": "https://generativelanguage.googleapis.com/v1beta/files/abc:download" }
            }] } }
        })).unwrap();
        assert!(matches!(poll.status, GoogleVeoStatus::Done { .. }));
        assert!(poll
            .metadata
            .pointer("/response/generateVideoResponse/generatedSamples/0/video/uri")
            .is_none());
    }

    #[test]
    fn reports_provider_reason_when_generated_video_is_filtered() {
        let poll = parse_poll_response(json!({
            "done": true,
            "response": { "generateVideoResponse": {
                "generatedSamples": [],
                "raiMediaFilteredCount": 1,
                "raiMediaFilteredReasons": [
                    "The input image resembles a public figure.\nPlease use another image."
                ]
            } }
        }))
        .unwrap();

        assert_eq!(
            poll.status,
            GoogleVeoStatus::Failed(
                "Google Veo filtered the generated video: The input image resembles a public figure. Please use another image."
                    .to_string()
            )
        );
        assert_eq!(
            poll.metadata
                .pointer("/response/generateVideoResponse/raiMediaFilteredCount"),
            Some(&json!(1))
        );
    }

    #[test]
    fn reports_generic_filter_message_when_provider_omits_reason() {
        let poll = parse_poll_response(json!({
            "done": true,
            "response": { "generateVideoResponse": {
                "raiMediaFilteredCount": 1
            } }
        }))
        .unwrap();

        assert_eq!(
            poll.status,
            GoogleVeoStatus::Failed(
                "Google Veo filtered the generated video under its Responsible AI policies."
                    .to_string()
            )
        );
    }
}
