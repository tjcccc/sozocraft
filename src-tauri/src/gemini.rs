use crate::models::GenerationRequest;
use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Proxy, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, thiserror::Error)]
pub enum GeminiError {
    #[error("Gemini API key is missing.")]
    MissingApiKey,
    #[error("Gemini request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Gemini returned an error ({status}): {body}")]
    Api { status: StatusCode, body: String },
    #[error("Gemini returned image data that could not be decoded.")]
    InvalidImageData,
}

#[derive(Debug, Clone)]
pub struct GeminiClient {
    api_key: String,
    base_url: String,
    client: Client,
}

#[derive(Debug, Clone)]
pub struct GeminiGeneratedImage {
    pub bytes: Vec<u8>,
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct GeminiResponse {
    pub images: Vec<GeminiGeneratedImage>,
    pub metadata: Value,
}

impl GeminiClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, GeminiError> {
        if api_key.trim().is_empty() {
            return Err(GeminiError::MissingApiKey);
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

        let client = client_builder.build()?;

        Ok(Self {
            api_key,
            base_url,
            client,
        })
    }

    pub async fn generate(
        &self,
        request: &GenerationRequest,
    ) -> Result<GeminiResponse, GeminiError> {
        let url = format!("{}/{}:generateContent", self.base_url, request.model);
        let body = build_request_body(request);
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(GeminiError::Api { status, body: text });
        }

        let json: Value = serde_json::from_str(&text).map_err(|_| GeminiError::InvalidImageData)?;
        parse_response(json)
    }
}

fn build_request_body(request: &GenerationRequest) -> Value {
    let mut parts = vec![json!({ "text": request.prompt.trim() })];

    if let Some(reference_images) = &request.reference_images {
        for path in reference_images {
            if path.trim().is_empty() {
                continue;
            }
            if let Ok(bytes) = std::fs::read(path) {
                parts.push(json!({
                    "inline_data": {
                        "mimeType": guess_mime(path),
                        "data": general_purpose::STANDARD.encode(bytes)
                    }
                }));
            }
        }
    }

    let mut generation_config = json!({
        "responseModalities": ["TEXT", "IMAGE"]
    });

    if let Some(temp) = request.options.temperature {
        if temp >= 0.0 {
            generation_config["temperature"] = json!(temp);
        }
    }
    if let Some(top_p) = request.options.top_p {
        generation_config["topP"] = json!(top_p);
    }
    if let Some(seed) = request.options.seed {
        if seed >= 0 {
            generation_config["seed"] = json!(seed);
        }
    }

    let mut image_config = serde_json::Map::new();
    if let Some(aspect_ratio) = &request.options.aspect_ratio {
        if aspect_ratio != "Auto" {
            image_config.insert("aspectRatio".to_string(), json!(aspect_ratio));
        }
    }

    if request.model != "gemini-2.5-flash-image" {
        if let Some(image_size) = &request.options.image_size {
            image_config.insert("imageSize".to_string(), json!(image_size));
        }
    }

    if !image_config.is_empty() {
        generation_config["imageConfig"] = Value::Object(image_config);
    }

    if request.model == "gemini-3.1-flash-image-preview" {
        if let Some(thinking_level) = &request.options.thinking_level {
            generation_config["thinkingConfig"] = json!({
                "thinkingLevel": thinking_level,
                "includeThoughts": false
            });
        }
    }

    json!({
        "contents": [{
            "role": "user",
            "parts": parts
        }],
        "generationConfig": generation_config
    })
}

fn parse_response(json: Value) -> Result<GeminiResponse, GeminiError> {
    let mut images = Vec::new();
    if let Some(candidates) = json.get("candidates").and_then(|value| value.as_array()) {
        for candidate in candidates {
            if let Some(parts) = candidate
                .get("content")
                .and_then(|content| content.get("parts"))
                .and_then(|value| value.as_array())
            {
                for part in parts {
                    let inline = part.get("inlineData").or_else(|| part.get("inline_data"));
                    let Some(inline) = inline else {
                        continue;
                    };
                    let Some(data) = inline.get("data").and_then(|value| value.as_str()) else {
                        continue;
                    };
                    let bytes = general_purpose::STANDARD
                        .decode(data)
                        .map_err(|_| GeminiError::InvalidImageData)?;
                    let mime = inline
                        .get("mimeType")
                        .or_else(|| inline.get("mime_type"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("image/png");
                    images.push(GeminiGeneratedImage {
                        bytes,
                        extension: extension_for_mime(mime).to_string(),
                    });
                }
            }
        }
    }

    let metadata = json!({
        "usageMetadata": json.get("usageMetadata").cloned().unwrap_or(Value::Null),
        "modelVersion": json.get("modelVersion").cloned().unwrap_or(Value::Null)
    });

    Ok(GeminiResponse { images, metadata })
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    }
}

fn guess_mime(path: &str) -> &'static str {
    match std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_image_response() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "inlineData": {
                            "mimeType": "image/png",
                            "data": general_purpose::STANDARD.encode([1, 2, 3])
                        }
                    }]
                }
            }]
        });

        let parsed = parse_response(response).unwrap();
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0].bytes, vec![1, 2, 3]);
        assert_eq!(parsed.images[0].extension, "png");
    }
}
