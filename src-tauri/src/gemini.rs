use crate::{
    gemini_models::{
        GEMINI_2_5_FLASH_ASPECT_RATIOS, GEMINI_3_FLASH_ASPECT_RATIOS, GEMINI_3_FLASH_IMAGE_SIZES,
        GEMINI_3_FLASH_THINKING_LEVELS, GEMINI_3_PRO_ASPECT_RATIOS, GEMINI_3_PRO_IMAGE_SIZES,
    },
    models::GenerationRequest,
};
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
        for image in reference_images {
            if image.data.trim().is_empty() {
                continue;
            }
            parts.push(json!({
                "inline_data": {
                    "mimeType": supported_input_mime(&image.mime_type),
                    "data": image.data
                }
            }));
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
    let mut image_config = serde_json::Map::new();
    if let Some(aspect_ratio) = supported_aspect_ratio(request) {
        image_config.insert("aspectRatio".to_string(), json!(aspect_ratio));
    }

    if let Some(image_size) = supported_image_size(request) {
        image_config.insert("imageSize".to_string(), json!(image_size));
    }

    if !image_config.is_empty() {
        generation_config["imageConfig"] = Value::Object(image_config);
    }

    if let Some(thinking_level) = supported_thinking_level(request) {
        generation_config["thinkingConfig"] = json!({
            "thinkingLevel": thinking_level,
            "includeThoughts": false
        });
    }

    json!({
        "safetySettings": default_safety_settings(),
        "contents": [{
            "role": "user",
            "parts": parts
        }],
        "generationConfig": generation_config
    })
}

fn default_safety_settings() -> Value {
    json!([
        {
            "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
            "threshold": "BLOCK_NONE"
        }
    ])
}

fn supported_aspect_ratio(request: &GenerationRequest) -> Option<&str> {
    let aspect_ratio = request.options.aspect_ratio.as_deref()?;
    if aspect_ratio == "Auto" {
        return None;
    }

    let supported = match request.model.as_str() {
        "gemini-3-pro-image-preview" => GEMINI_3_PRO_ASPECT_RATIOS.as_slice(),
        "gemini-3.1-flash-image-preview" => GEMINI_3_FLASH_ASPECT_RATIOS.as_slice(),
        "gemini-2.5-flash-image" => GEMINI_2_5_FLASH_ASPECT_RATIOS.as_slice(),
        _ => return None,
    };

    supported.contains(&aspect_ratio).then_some(aspect_ratio)
}

fn supported_image_size(request: &GenerationRequest) -> Option<&str> {
    let image_size = request.options.image_size.as_deref()?;
    let supported = match request.model.as_str() {
        "gemini-3-pro-image-preview" => GEMINI_3_PRO_IMAGE_SIZES.as_slice(),
        "gemini-3.1-flash-image-preview" => GEMINI_3_FLASH_IMAGE_SIZES.as_slice(),
        _ => return None,
    };

    supported.contains(&image_size).then_some(image_size)
}

fn supported_thinking_level(request: &GenerationRequest) -> Option<&str> {
    if request.model != "gemini-3.1-flash-image-preview" {
        return None;
    }

    let thinking_level = request.options.thinking_level.as_deref()?;
    GEMINI_3_FLASH_THINKING_LEVELS
        .contains(&thinking_level)
        .then_some(thinking_level)
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
                    images.push(GeminiGeneratedImage { bytes });
                }
            }
        }
    }

    let metadata = json!({
        "usageMetadata": json.get("usageMetadata").cloned().unwrap_or(Value::Null),
        "modelVersion": json.get("modelVersion").cloned().unwrap_or(Value::Null),
        "promptFeedback": json.get("promptFeedback").cloned().unwrap_or(Value::Null),
        "candidates": summarize_candidates(&json)
    });

    Ok(GeminiResponse { images, metadata })
}

pub fn no_image_data_error(metadata: &Value) -> String {
    let mut details = vec!["Gemini returned no image data.".to_string()];

    if let Some(prompt_feedback) = metadata.get("promptFeedback") {
        if let Some(block_reason) = prompt_feedback.get("blockReason").and_then(Value::as_str) {
            details.push(format!("Prompt block reason: {block_reason}."));
        }
        if let Some(safety) = prompt_feedback.get("safetyRatings") {
            if let Some(summary) = safety_ratings_summary(safety) {
                details.push(format!("Prompt safety ratings: {summary}."));
            }
        }
    }

    if let Some(candidates) = metadata.get("candidates").and_then(Value::as_array) {
        if candidates.is_empty() {
            details.push("Response contained no candidates.".to_string());
        }

        for (index, candidate) in candidates.iter().enumerate() {
            let label = format!("Candidate {}", index + 1);
            if let Some(finish_reason) = candidate.get("finishReason").and_then(Value::as_str) {
                details.push(format!("{label} finish reason: {finish_reason}."));
            }
            if let Some(finish_message) = candidate.get("finishMessage").and_then(Value::as_str) {
                details.push(format!("{label} finish message: {finish_message}."));
            }
            if let Some(safety) = candidate.get("safetyRatings") {
                if let Some(summary) = safety_ratings_summary(safety) {
                    details.push(format!("{label} safety ratings: {summary}."));
                }
            }
            if let Some(text) = candidate
                .get("textParts")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str)
            {
                details.push(format!("{label} text response: {}.", truncate(text, 500)));
            }
        }
    }

    details.join("\n")
}

fn summarize_candidates(json: &Value) -> Value {
    let Some(candidates) = json.get("candidates").and_then(Value::as_array) else {
        return Value::Null;
    };

    Value::Array(
        candidates
            .iter()
            .map(|candidate| {
                let text_parts = candidate
                    .get("content")
                    .and_then(|content| content.get("parts"))
                    .and_then(Value::as_array)
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|part| part.get("text").and_then(Value::as_str))
                            .map(|text| Value::String(truncate(text, 1000)))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                json!({
                    "finishReason": candidate.get("finishReason").cloned().unwrap_or(Value::Null),
                    "finishMessage": candidate.get("finishMessage").cloned().unwrap_or(Value::Null),
                    "safetyRatings": candidate.get("safetyRatings").cloned().unwrap_or(Value::Null),
                    "textParts": text_parts
                })
            })
            .collect(),
    )
}

fn safety_ratings_summary(value: &Value) -> Option<String> {
    let ratings = value.as_array()?;
    let items = ratings
        .iter()
        .filter_map(|rating| {
            let category = rating.get("category").and_then(Value::as_str)?;
            let probability = rating
                .get("probability")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN");
            let blocked = rating
                .get("blocked")
                .and_then(Value::as_bool)
                .map(|value| if value { ", blocked" } else { "" })
                .unwrap_or("");
            Some(format!("{category}={probability}{blocked}"))
        })
        .collect::<Vec<_>>();

    (!items.is_empty()).then(|| items.join(", "))
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn supported_input_mime(mime_type: &str) -> &'static str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GenerationOptions, ReferenceImageInput};

    #[test]
    fn parses_inline_image_response() {
        let response = json!({
            "candidates": [{
                "finishReason": "STOP",
                "safetyRatings": [{
                    "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                    "probability": "NEGLIGIBLE"
                }],
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
        assert_eq!(parsed.metadata["candidates"][0]["finishReason"], "STOP");
        assert_eq!(
            parsed.metadata["candidates"][0]["safetyRatings"][0]["category"],
            "HARM_CATEGORY_SEXUALLY_EXPLICIT"
        );
    }

    #[test]
    fn payload_includes_adjustable_safety_settings() {
        let body = build_request_body(&request_for_model("gemini-3.1-flash-image-preview"));
        let safety = body["safetySettings"].as_array().unwrap();

        assert!(safety.iter().any(|setting| {
            setting["category"] == "HARM_CATEGORY_SEXUALLY_EXPLICIT"
                && setting["threshold"] == "BLOCK_NONE"
        }));
        assert_eq!(safety.len(), 1);
    }

    #[test]
    fn no_image_data_error_includes_safety_diagnostics() {
        let metadata = json!({
            "promptFeedback": {
                "blockReason": "SAFETY",
                "safetyRatings": [{
                    "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                    "probability": "HIGH",
                    "blocked": true
                }]
            },
            "candidates": [{
                "finishReason": "SAFETY",
                "finishMessage": "Safety filters blocked the candidate.",
                "safetyRatings": [{
                    "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                    "probability": "LOW"
                }],
                "textParts": ["I cannot provide that image."]
            }]
        });

        let message = no_image_data_error(&metadata);

        assert!(message.contains("Prompt block reason: SAFETY."));
        assert!(message.contains("HARM_CATEGORY_SEXUALLY_EXPLICIT=HIGH, blocked"));
        assert!(message.contains("Candidate 1 finish reason: SAFETY."));
        assert!(message.contains("I cannot provide that image."));
    }

    #[test]
    fn flash_3_1_payload_allows_extended_size_and_thinking_options() {
        let body = build_request_body(&request_for_model("gemini-3.1-flash-image-preview"));
        let config = body.get("generationConfig").unwrap();

        assert_eq!(config["imageConfig"]["aspectRatio"], "1:8");
        assert_eq!(config["imageConfig"]["imageSize"], "512");
        assert_eq!(config["thinkingConfig"]["thinkingLevel"], "high");
    }

    #[test]
    fn payload_includes_reference_images_as_inline_data() {
        let mut request = request_for_model("gemini-3.1-flash-image-preview");
        request.reference_images = Some(vec![ReferenceImageInput {
            name: "ref.png".to_string(),
            mime_type: "image/png".to_string(),
            data: "abc123".to_string(),
        }]);

        let body = build_request_body(&request);
        let parts = body["contents"][0]["parts"].as_array().unwrap();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1]["inline_data"]["mimeType"], "image/png");
        assert_eq!(parts[1]["inline_data"]["data"], "abc123");
    }

    #[test]
    fn pro_payload_rejects_512_and_model_managed_thinking() {
        let body = build_request_body(&request_for_model("gemini-3-pro-image-preview"));
        let config = body.get("generationConfig").unwrap();

        assert_eq!(config["imageConfig"]["aspectRatio"], "21:9");
        assert_eq!(config["imageConfig"].get("imageSize"), None);
        assert_eq!(config.get("thinkingConfig"), None);
    }

    #[test]
    fn flash_2_5_payload_omits_image_size_and_thinking() {
        let body = build_request_body(&request_for_model("gemini-2.5-flash-image"));
        let config = body.get("generationConfig").unwrap();

        assert_eq!(config["imageConfig"]["aspectRatio"], "21:9");
        assert_eq!(config["imageConfig"].get("imageSize"), None);
        assert_eq!(config.get("thinkingConfig"), None);
    }

    fn request_for_model(model: &str) -> GenerationRequest {
        GenerationRequest {
            provider: "nano-banana".to_string(),
            model: model.to_string(),
            prompt: "Render a test image".to_string(),
            prompt_snapshot: None,
            batch_count: 1,
            reference_images: None,
            output_template: "{id}.{extension}".to_string(),
            options: GenerationOptions {
                aspect_ratio: Some(match model {
                    "gemini-3.1-flash-image-preview" => "1:8".to_string(),
                    _ => "21:9".to_string(),
                }),
                image_size: Some("512".to_string()),
                temperature: Some(-1.0),
                top_p: Some(0.95),
                thinking_level: Some("high".to_string()),
                quality: None,
            },
            base_url: None,
        }
    }
}
