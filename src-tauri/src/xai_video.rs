use crate::models::VideoGenerationRequest;
use reqwest::{redirect, Client, Proxy, StatusCode, Url};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

const DEFAULT_ENDPOINT: &str = "https://api.x.ai/v1";
const MAX_JSON_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_VIDEO_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum XaiVideoError {
    #[error("xAI API key is missing.")]
    MissingApiKey,
    #[error("xAI video request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("xAI returned an error ({status}): {body}")]
    Api { status: StatusCode, body: String },
    #[error("xAI returned an invalid video response: {0}")]
    InvalidResponse(String),
    #[error("xAI video request failed: {0}")]
    GenerationFailed(String),
    #[error("xAI returned an unsafe video URL.")]
    UnsafeVideoUrl,
    #[error("xAI returned a video larger than 512 MB.")]
    VideoTooLarge,
    #[error("xAI returned data that is not an MP4 video.")]
    InvalidVideoData,
}

#[derive(Debug, Clone)]
pub struct XaiVideoClient {
    api_key: String,
    base_url: String,
    client: Client,
    download_client: Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XaiVideoStatus {
    Pending,
    Done { url: String, duration: Option<u8> },
    Failed(String),
    Expired,
}

#[derive(Debug, Clone)]
pub struct XaiVideoPollResponse {
    pub status: XaiVideoStatus,
    pub metadata: Value,
}

impl XaiVideoClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, XaiVideoError> {
        if api_key.trim().is_empty() {
            return Err(XaiVideoError::MissingApiKey);
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
                    if is_xai_video_url(attempt.url()) {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }));

        if let Some(proxy_url) = proxy_url.filter(|value| !value.trim().is_empty()) {
            let proxy = Proxy::all(proxy_url.trim())?;
            client_builder = client_builder.proxy(proxy.clone());
            download_builder = download_builder.proxy(proxy);
        }

        Ok(Self {
            api_key,
            base_url,
            client: client_builder.build()?,
            download_client: download_builder.build()?,
        })
    }

    pub async fn start(&self, request: &VideoGenerationRequest) -> Result<String, XaiVideoError> {
        let response = self
            .client
            .post(format!("{}/videos/generations", self.base_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&build_start_body(request))
            .send()
            .await?;
        let (status, body) = response_json(response).await?;
        if !status.is_success() {
            return Err(XaiVideoError::Api {
                status,
                body: sanitized_body(&body),
            });
        }
        parse_start_response(&body)
    }

    pub async fn poll(&self, request_id: &str) -> Result<XaiVideoPollResponse, XaiVideoError> {
        validate_request_id(request_id)?;
        let response = self
            .client
            .get(format!("{}/videos/{request_id}", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let (status, body) = response_json(response).await?;
        if !status.is_success() {
            return Err(XaiVideoError::Api {
                status,
                body: sanitized_body(&body),
            });
        }
        parse_poll_response(body)
    }

    pub async fn download_to(
        &self,
        video_url: &str,
        output_path: &Path,
    ) -> Result<u64, XaiVideoError> {
        let url = Url::parse(video_url).map_err(|_| XaiVideoError::UnsafeVideoUrl)?;
        if !is_xai_video_url(&url) {
            return Err(XaiVideoError::UnsafeVideoUrl);
        }

        let mut response = self.download_client.get(url).send().await?;
        if !is_xai_video_url(response.url()) {
            return Err(XaiVideoError::UnsafeVideoUrl);
        }
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map(|value| sanitized_text(&value))
                .unwrap_or_else(|_| "Video download failed.".to_string());
            return Err(XaiVideoError::Api { status, body });
        }
        if response
            .content_length()
            .map(|length| length > MAX_VIDEO_BYTES as u64)
            .unwrap_or(false)
        {
            return Err(XaiVideoError::VideoTooLarge);
        }

        let mut file = tokio::fs::File::create(output_path)
            .await
            .map_err(|error| {
                XaiVideoError::InvalidResponse(format!("failed to create video output: {error}"))
            })?;
        let mut total_bytes = 0usize;
        let mut signature = Vec::with_capacity(12);
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(error) => {
                    let _ = tokio::fs::remove_file(output_path).await;
                    return Err(XaiVideoError::Request(error));
                }
            };
            total_bytes = total_bytes
                .checked_add(chunk.len())
                .ok_or(XaiVideoError::VideoTooLarge)?;
            if total_bytes > MAX_VIDEO_BYTES {
                let _ = tokio::fs::remove_file(output_path).await;
                return Err(XaiVideoError::VideoTooLarge);
            }
            if signature.len() < 12 {
                let needed = 12usize.saturating_sub(signature.len());
                signature.extend_from_slice(&chunk[..chunk.len().min(needed)]);
            }
            if let Err(error) = file.write_all(&chunk).await {
                let _ = tokio::fs::remove_file(output_path).await;
                return Err(XaiVideoError::InvalidResponse(format!(
                    "failed to write video output: {error}"
                )));
            }
        }
        if let Err(error) = file.flush().await {
            let _ = tokio::fs::remove_file(output_path).await;
            return Err(XaiVideoError::InvalidResponse(format!(
                "failed to flush video output: {error}"
            )));
        }
        drop(file);
        if !is_mp4(&signature) {
            let _ = tokio::fs::remove_file(output_path).await;
            return Err(XaiVideoError::InvalidVideoData);
        }
        Ok(total_bytes as u64)
    }
}

fn build_start_body(request: &VideoGenerationRequest) -> Value {
    let mut body = json!({
        "model": request.model,
        "prompt": request.prompt.trim(),
        "duration": request.options.duration,
        "aspect_ratio": request.options.aspect_ratio,
        "resolution": request.options.resolution,
    });
    if let Some(image) = &request.starting_image {
        body["image"] = json!({
            "url": format!(
                "data:{};base64,{}",
                supported_input_mime(&image.mime_type),
                image.data.trim()
            )
        });
    }
    if let Some(reference_images) = request
        .reference_images
        .as_ref()
        .filter(|images| !images.is_empty())
    {
        body["reference_images"] = Value::Array(
            reference_images
                .iter()
                .map(|image| {
                    json!({
                        "url": format!(
                            "data:{};base64,{}",
                            supported_input_mime(&image.mime_type),
                            image.data.trim()
                        )
                    })
                })
                .collect(),
        );
    }
    body
}

fn supported_input_mime(mime_type: &str) -> &str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        _ => "image/png",
    }
}

fn parse_start_response(value: &Value) -> Result<String, XaiVideoError> {
    let request_id = value
        .get("request_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| XaiVideoError::InvalidResponse("missing request_id".to_string()))?;
    validate_request_id(request_id)?;
    Ok(request_id.to_string())
}

fn parse_poll_response(value: Value) -> Result<XaiVideoPollResponse, XaiVideoError> {
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status.to_ascii_lowercase())
        .ok_or_else(|| XaiVideoError::InvalidResponse("missing status".to_string()))?;

    let status = match status.as_str() {
        "pending" | "processing" | "running" => XaiVideoStatus::Pending,
        "done" => {
            let video = value
                .get("video")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    XaiVideoError::InvalidResponse("missing completed video".to_string())
                })?;
            if video.get("respect_moderation").and_then(Value::as_bool) == Some(false) {
                return Err(XaiVideoError::GenerationFailed(
                    "Video was filtered by xAI moderation.".to_string(),
                ));
            }
            let url = video
                .get("url")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|url| !url.is_empty())
                .ok_or_else(|| XaiVideoError::InvalidResponse("missing video URL".to_string()))?;
            let parsed_url = Url::parse(url).map_err(|_| XaiVideoError::UnsafeVideoUrl)?;
            if !is_xai_video_url(&parsed_url) {
                return Err(XaiVideoError::UnsafeVideoUrl);
            }
            let duration = video
                .get("duration")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok());
            XaiVideoStatus::Done {
                url: url.to_string(),
                duration,
            }
        }
        "failed" => XaiVideoStatus::Failed(response_error_message(&value)),
        "expired" => XaiVideoStatus::Expired,
        other => {
            return Err(XaiVideoError::InvalidResponse(format!(
                "unsupported status `{other}`"
            )))
        }
    };

    Ok(XaiVideoPollResponse {
        status,
        metadata: sanitized_metadata(&value),
    })
}

async fn response_json(response: reqwest::Response) -> Result<(StatusCode, Value), XaiVideoError> {
    let status = response.status();
    if response
        .content_length()
        .map(|length| length > MAX_JSON_RESPONSE_BYTES as u64)
        .unwrap_or(false)
    {
        return Err(XaiVideoError::InvalidResponse(
            "response exceeded 1 MB".to_string(),
        ));
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_JSON_RESPONSE_BYTES {
        return Err(XaiVideoError::InvalidResponse(
            "response exceeded 1 MB".to_string(),
        ));
    }
    let value = serde_json::from_slice(&bytes).map_err(|_| {
        XaiVideoError::InvalidResponse(format!(
            "expected JSON, received `{}`",
            sanitized_text(&String::from_utf8_lossy(&bytes))
        ))
    })?;
    Ok((status, value))
}

fn validate_api_base_url(base_url: &str) -> Result<(), XaiVideoError> {
    let url = Url::parse(base_url)
        .map_err(|_| XaiVideoError::InvalidResponse("invalid xAI base URL".to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(XaiVideoError::InvalidResponse(
            "invalid xAI base URL".to_string(),
        ));
    }
    Ok(())
}

fn validate_request_id(request_id: &str) -> Result<(), XaiVideoError> {
    if request_id.is_empty()
        || request_id.len() > 128
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(XaiVideoError::InvalidResponse(
            "invalid request_id".to_string(),
        ));
    }
    Ok(())
}

fn is_xai_video_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .map(|host| host == "x.ai" || host.ends_with(".x.ai"))
            .unwrap_or(false)
}

fn is_mp4(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.get(4..8) == Some(b"ftyp")
}

fn response_error_message(value: &Value) -> String {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(sanitized_text)
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| "xAI video generation failed.".to_string())
}

fn sanitized_body(value: &Value) -> String {
    sanitized_text(&sanitized_metadata(value).to_string())
}

fn sanitized_metadata(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter_map(|(key, value)| {
                    if ["authorization", "api_key", "prompt", "url"]
                        .contains(&key.to_ascii_lowercase().as_str())
                    {
                        None
                    } else {
                        Some((key.clone(), sanitized_metadata(value)))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(sanitized_metadata).collect()),
        _ => value.clone(),
    }
}

fn sanitized_text(value: &str) -> String {
    const MAX_CHARS: usize = 2_000;
    let normalized = value.replace(['\r', '\n'], " ");
    if normalized.chars().count() <= MAX_CHARS {
        normalized
    } else {
        format!(
            "{}...",
            normalized.chars().take(MAX_CHARS).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{VideoGenerationOptions, VideoInputMode, VideoProvider};

    #[test]
    fn start_payload_maps_documented_video_options() {
        let request = request();
        let body = build_start_body(&request);
        assert_eq!(body["model"], "grok-imagine-video");
        assert_eq!(body["duration"], 5);
        assert_eq!(body["aspect_ratio"], "16:9");
        assert_eq!(body["resolution"], "480p");
        assert!(body.get("image").is_none());
    }

    #[test]
    fn start_payload_maps_one_starting_image_as_a_data_uri() {
        let mut request = request();
        request.input_mode = VideoInputMode::Image;
        request.starting_image = Some(crate::models::ReferenceImageInput {
            name: "starting.webp".to_string(),
            mime_type: "image/webp".to_string(),
            data: "YWJj".to_string(),
            asset_id: None,
        });

        let body = build_start_body(&request);

        assert_eq!(body["image"]["url"], "data:image/webp;base64,YWJj");
        assert!(body.get("reference_images").is_none());
    }

    #[test]
    fn start_payload_maps_reference_images_as_data_uris() {
        let mut request = request();
        request.input_mode = VideoInputMode::Reference;
        request.reference_images = Some(vec![
            crate::models::ReferenceImageInput {
                name: "first.png".to_string(),
                mime_type: "image/png".to_string(),
                data: "Zmlyc3Q=".to_string(),
                asset_id: None,
            },
            crate::models::ReferenceImageInput {
                name: "second.jpg".to_string(),
                mime_type: "image/jpeg".to_string(),
                data: "c2Vjb25k".to_string(),
                asset_id: None,
            },
        ]);

        let body = build_start_body(&request);

        assert!(body.get("image").is_none());
        assert_eq!(
            body["reference_images"],
            json!([
                { "url": "data:image/png;base64,Zmlyc3Q=" },
                { "url": "data:image/jpeg;base64,c2Vjb25k" }
            ])
        );
    }

    #[test]
    fn parses_pending_and_completed_poll_responses() {
        let pending = parse_poll_response(json!({ "status": "pending", "progress": 25 })).unwrap();
        assert_eq!(pending.status, XaiVideoStatus::Pending);

        let done = parse_poll_response(json!({
            "status": "done",
            "video": {
                "url": "https://vidgen.x.ai/generated/video.mp4",
                "duration": 5,
                "respect_moderation": true
            }
        }))
        .unwrap();
        assert_eq!(
            done.status,
            XaiVideoStatus::Done {
                url: "https://vidgen.x.ai/generated/video.mp4".to_string(),
                duration: Some(5)
            }
        );
    }

    #[test]
    fn rejects_non_xai_video_urls_and_filtered_results() {
        let unsafe_url = parse_poll_response(json!({
            "status": "done",
            "video": { "url": "http://127.0.0.1/private.mp4" }
        }))
        .unwrap_err();
        assert!(matches!(unsafe_url, XaiVideoError::UnsafeVideoUrl));

        let filtered = parse_poll_response(json!({
            "status": "done",
            "video": {
                "url": "https://vidgen.x.ai/generated/video.mp4",
                "respect_moderation": false
            }
        }))
        .unwrap_err();
        assert!(filtered.to_string().contains("moderation"));
    }

    #[test]
    fn poll_metadata_does_not_retain_temporary_url() {
        let done = parse_poll_response(json!({
            "status": "done",
            "video": {
                "url": "https://vidgen.x.ai/generated/video.mp4",
                "duration": 5
            }
        }))
        .unwrap();
        assert!(done.metadata.pointer("/video/url").is_none());
        assert_eq!(done.metadata.pointer("/video/duration"), Some(&json!(5)));
    }

    #[test]
    fn mp4_validation_checks_file_type_box() {
        assert!(is_mp4(b"\0\0\0\x18ftypisom"));
        assert!(!is_mp4(b"not an mp4 file"));
    }

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            task_id: None,
            provider: VideoProvider::GrokImagine,
            model: "grok-imagine-video".to_string(),
            prompt: "  A calm lake at sunrise  ".to_string(),
            prompt_snapshot: None,
            input_mode: VideoInputMode::Text,
            starting_image: None,
            ending_image: None,
            reference_images: None,
            options: VideoGenerationOptions {
                duration: 5,
                aspect_ratio: "16:9".to_string(),
                resolution: "480p".to_string(),
                generate_audio: None,
            },
        }
    }
}
