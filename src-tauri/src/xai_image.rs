use crate::models::GenerationRequest;
use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Proxy, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.x.ai/v1";

#[derive(Debug, thiserror::Error)]
pub enum XaiImageError {
    #[error("xAI API key is missing.")]
    MissingApiKey,
    #[error("xAI image request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("xAI returned an error ({status}): {body}")]
    Api { status: StatusCode, body: String },
    #[error("xAI returned image data that could not be decoded.")]
    InvalidImageData,
}

#[derive(Debug, Clone)]
pub struct XaiImageClient {
    api_key: String,
    base_url: String,
    client: Client,
}

#[derive(Debug, Clone)]
pub struct XaiGeneratedImage {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct XaiImageResponse {
    pub images: Vec<XaiGeneratedImage>,
    pub metadata: Value,
}

impl XaiImageClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, XaiImageError> {
        if api_key.trim().is_empty() {
            return Err(XaiImageError::MissingApiKey);
        }

        let base_url = base_url
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string())
            .trim_end_matches('/')
            .to_string();

        let mut client_builder =
            Client::builder().timeout(Duration::from_secs(timeout_seconds.max(10)));

        if let Some(proxy_url) = proxy_url.filter(|value| !value.trim().is_empty()) {
            client_builder = client_builder.proxy(Proxy::all(proxy_url.trim())?);
        }

        Ok(Self {
            api_key,
            base_url,
            client: client_builder.build()?,
        })
    }

    pub async fn generate(
        &self,
        request: &GenerationRequest,
    ) -> Result<XaiImageResponse, XaiImageError> {
        let has_reference_images = request
            .reference_images
            .as_ref()
            .map(|items| !items.is_empty())
            .unwrap_or(false);
        let endpoint = if has_reference_images {
            "images/edits"
        } else {
            "images/generations"
        };
        let url = format!("{}/{}", self.base_url, endpoint);
        let body = build_request_body(request);
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(XaiImageError::Api { status, body: text });
        }

        let json: Value =
            serde_json::from_str(&text).map_err(|_| XaiImageError::InvalidImageData)?;
        parse_response(json)
    }
}

fn build_request_body(request: &GenerationRequest) -> Value {
    let mut body = json!({
        "model": request.model,
        "prompt": request.prompt.trim(),
        "n": request.batch_count,
        "response_format": "b64_json"
    });

    if let Some(aspect_ratio) = request.options.aspect_ratio.as_deref() {
        if aspect_ratio != "auto" {
            body["aspect_ratio"] = json!(aspect_ratio);
        }
    }
    if let Some(resolution) = request.options.image_size.as_deref() {
        match resolution {
            "1k" | "2k" => body["resolution"] = json!(resolution),
            _ => {}
        }
    }
    if let Some(quality) = request.options.quality.as_deref() {
        if ["low", "medium", "high"].contains(&quality) {
            body["quality"] = json!(quality);
        }
    }
    if let Some(reference_images) = &request.reference_images {
        let images = reference_images
            .iter()
            .filter(|image| !image.data.trim().is_empty())
            .map(|image| {
                json!({
                    "type": "image_url",
                    "url": format!(
                        "data:{};base64,{}",
                        supported_input_mime(&image.mime_type),
                        image.data
                    )
                })
            })
            .collect::<Vec<_>>();

        match images.len() {
            0 => {}
            1 => body["image"] = images[0].clone(),
            _ => body["images"] = Value::Array(images),
        }
    }

    body
}

fn supported_input_mime(mime: &str) -> &str {
    match mime {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::build_request_body;
    use crate::models::{GenerationOptions, GenerationRequest, ReferenceImageInput};

    #[test]
    fn generation_payload_uses_xai_options() {
        let body = build_request_body(&request(None));

        assert_eq!(body["model"], "grok-imagine-image");
        assert_eq!(body["n"], 4);
        assert_eq!(body["response_format"], "b64_json");
        assert_eq!(body["aspect_ratio"], "9:19.5");
        assert_eq!(body["resolution"], "2k");
        assert_eq!(body["quality"], "high");
        assert!(body.get("image").is_none());
        assert!(body.get("images").is_none());
    }

    #[test]
    fn edit_payload_uses_single_image_object() {
        let body = build_request_body(&request(Some(vec![reference_image("image/png", "abc")])));

        assert_eq!(body["image"]["type"], "image_url");
        assert_eq!(body["image"]["url"], "data:image/png;base64,abc");
        assert!(body.get("images").is_none());
    }

    #[test]
    fn edit_payload_uses_images_array_for_multiple_inputs() {
        let body = build_request_body(&request(Some(vec![
            reference_image("image/jpeg", "abc"),
            reference_image("image/webp", "def"),
        ])));

        let images = body["images"].as_array().unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0]["url"], "data:image/jpeg;base64,abc");
        assert_eq!(images[1]["url"], "data:image/webp;base64,def");
        assert!(body.get("image").is_none());
    }

    fn request(reference_images: Option<Vec<ReferenceImageInput>>) -> GenerationRequest {
        GenerationRequest {
            task_id: None,
            provider: "grok-imagine".to_string(),
            model: "grok-imagine-image".to_string(),
            prompt: "Render a test image".to_string(),
            prompt_snapshot: None,
            batch_count: 4,
            reference_images,
            output_template: "{id}.{extension}".to_string(),
            options: GenerationOptions {
                aspect_ratio: Some("9:19.5".to_string()),
                image_size: Some("2k".to_string()),
                temperature: None,
                top_p: None,
                thinking_level: None,
                quality: Some("high".to_string()),
            },
            base_url: None,
        }
    }

    fn reference_image(mime_type: &str, data: &str) -> ReferenceImageInput {
        ReferenceImageInput {
            name: "input.png".to_string(),
            mime_type: mime_type.to_string(),
            data: data.to_string(),
        }
    }
}

fn parse_response(json: Value) -> Result<XaiImageResponse, XaiImageError> {
    let mut images = Vec::new();
    if let Some(items) = json.get("data").and_then(Value::as_array) {
        for item in items {
            let Some(data) = item.get("b64_json").and_then(Value::as_str) else {
                continue;
            };
            let bytes = general_purpose::STANDARD
                .decode(data)
                .map_err(|_| XaiImageError::InvalidImageData)?;
            images.push(XaiGeneratedImage { bytes });
        }
    }

    let metadata = json!({
        "model": json.get("model").cloned().unwrap_or(Value::Null),
        "dataCount": images.len()
    });

    Ok(XaiImageResponse { images, metadata })
}
