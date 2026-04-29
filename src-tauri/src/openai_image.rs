use crate::models::GenerationRequest;
use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Proxy, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1";

#[derive(Debug, thiserror::Error)]
pub enum OpenAiImageError {
    #[error("OpenAI API key is missing.")]
    MissingApiKey,
    #[error("OpenAI image request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("OpenAI returned an error ({status}): {body}")]
    Api { status: StatusCode, body: String },
    #[error("OpenAI returned image data that could not be decoded.")]
    InvalidImageData,
}

#[derive(Debug, Clone)]
pub struct OpenAiImageClient {
    api_key: String,
    base_url: String,
    client: Client,
}

#[derive(Debug, Clone)]
pub struct OpenAiGeneratedImage {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct OpenAiImageResponse {
    pub images: Vec<OpenAiGeneratedImage>,
    pub metadata: Value,
}

impl OpenAiImageClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, OpenAiImageError> {
        if api_key.trim().is_empty() {
            return Err(OpenAiImageError::MissingApiKey);
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
    ) -> Result<OpenAiImageResponse, OpenAiImageError> {
        let url = format!("{}/images/generations", self.base_url);
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
            return Err(OpenAiImageError::Api { status, body: text });
        }

        let json: Value =
            serde_json::from_str(&text).map_err(|_| OpenAiImageError::InvalidImageData)?;
        parse_response(json)
    }
}

fn build_request_body(request: &GenerationRequest) -> Value {
    let mut body = json!({
        "model": request.model,
        "prompt": request.prompt.trim(),
        "n": request.batch_count,
        "output_format": "png"
    });

    if let Some(size) = request.options.image_size.as_deref() {
        if ["auto", "1024x1024", "1536x1024", "1024x1536"].contains(&size) {
            body["size"] = json!(size);
        }
    }
    if let Some(quality) = request.options.quality.as_deref() {
        if ["auto", "low", "medium", "high"].contains(&quality) {
            body["quality"] = json!(quality);
        }
    }

    body
}

fn parse_response(json: Value) -> Result<OpenAiImageResponse, OpenAiImageError> {
    let mut images = Vec::new();
    if let Some(items) = json.get("data").and_then(Value::as_array) {
        for item in items {
            let Some(data) = item.get("b64_json").and_then(Value::as_str) else {
                continue;
            };
            let bytes = decode_image_payload(data)?;
            images.push(OpenAiGeneratedImage { bytes });
        }
    }

    let metadata = json!({
        "created": json.get("created").cloned().unwrap_or(Value::Null),
        "usage": json.get("usage").cloned().unwrap_or(Value::Null),
        "dataCount": images.len()
    });

    Ok(OpenAiImageResponse { images, metadata })
}

fn decode_image_payload(data: &str) -> Result<Vec<u8>, OpenAiImageError> {
    let payload = data
        .trim()
        .strip_prefix("data:")
        .and_then(|value| value.split_once(',').map(|(_, encoded)| encoded))
        .unwrap_or_else(|| data.trim());

    general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| OpenAiImageError::InvalidImageData)
}

#[cfg(test)]
mod tests {
    use super::{build_request_body, decode_image_payload, parse_response};
    use crate::models::{GenerationOptions, GenerationRequest};
    use base64::{engine::general_purpose, Engine as _};
    use serde_json::json;

    #[test]
    fn generation_payload_uses_gpt_image_2_options() {
        let body = build_request_body(&request());

        assert_eq!(body["model"], "gpt-image-2");
        assert_eq!(body["n"], 3);
        assert_eq!(body["output_format"], "png");
        assert_eq!(body["size"], "1536x1024");
        assert_eq!(body["quality"], "high");
    }

    #[test]
    fn parser_accepts_raw_base64() {
        let encoded = general_purpose::STANDARD.encode([1, 2, 3, 4]);
        let response = parse_response(json!({
            "data": [{ "b64_json": encoded }]
        }))
        .unwrap();

        assert_eq!(response.images[0].bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn parser_accepts_data_url_base64() {
        let encoded = general_purpose::STANDARD.encode([5, 6, 7, 8]);
        let bytes = decode_image_payload(&format!("data:image/png;base64,{encoded}")).unwrap();

        assert_eq!(bytes, vec![5, 6, 7, 8]);
    }

    fn request() -> GenerationRequest {
        GenerationRequest {
            provider: "gpt-image".to_string(),
            model: "gpt-image-2".to_string(),
            prompt: "Render a test image".to_string(),
            prompt_snapshot: None,
            batch_count: 3,
            reference_images: None,
            output_template: "{id}.{extension}".to_string(),
            options: GenerationOptions {
                aspect_ratio: Some("3:2".to_string()),
                image_size: Some("1536x1024".to_string()),
                temperature: None,
                top_p: None,
                thinking_level: None,
                quality: Some("high".to_string()),
            },
            base_url: None,
        }
    }
}
