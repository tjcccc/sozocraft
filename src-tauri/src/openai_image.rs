use crate::models::{GenerationRequest, ReferenceImageInput};
use base64::{engine::general_purpose, Engine as _};
use reqwest::{multipart, Client, Proxy, RequestBuilder, StatusCode};
use serde_json::{json, Value};
use std::error::Error;
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1";
const OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, thiserror::Error)]
pub enum OpenAiImageError {
    #[error("OpenAI API key is missing.")]
    MissingApiKey,
    #[error("OpenAI image request failed: {0}")]
    Request(String),
    #[error("OpenAI returned an error ({status}): {body}")]
    Api { status: StatusCode, body: String },
    #[error("OpenAI returned image data that could not be decoded.")]
    InvalidImageData,
}

impl From<reqwest::Error> for OpenAiImageError {
    fn from(err: reqwest::Error) -> Self {
        Self::Request(describe_reqwest_error(&err))
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiImageClient {
    api_key: String,
    platform: OpenAiApiPlatform,
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
        platform: String,
        base_url: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, OpenAiImageError> {
        if api_key.trim().is_empty() {
            return Err(OpenAiImageError::MissingApiKey);
        }

        let platform = OpenAiApiPlatform::from_config(&platform);
        let base_url = base_url
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| platform.default_endpoint().to_string())
            .trim_end_matches('/')
            .to_string();

        let mut client_builder =
            Client::builder().timeout(Duration::from_secs(timeout_seconds.max(10)));
        if platform == OpenAiApiPlatform::OpenRouter {
            client_builder = client_builder.http1_only().pool_max_idle_per_host(0);
        }

        if let Some(proxy_url) = proxy_url.filter(|value| !value.trim().is_empty()) {
            client_builder = client_builder.proxy(Proxy::all(proxy_url.trim())?);
        }

        Ok(Self {
            api_key,
            platform,
            base_url,
            client: client_builder.build()?,
        })
    }

    pub async fn generate(
        &self,
        request: &GenerationRequest,
    ) -> Result<OpenAiImageResponse, OpenAiImageError> {
        let endpoint = resolve_endpoint(
            self.platform,
            &self.base_url,
            &request.model,
            has_reference_images(request),
        );
        let builder = match endpoint {
            OpenAiEndpoint::Images { url } => self
                .client
                .post(url)
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/json")
                .json(&build_image_api_request_body(request)),
            OpenAiEndpoint::ImageEdits { url } => {
                apply_multipart_form(self.client.post(url).bearer_auth(&self.api_key), request)?
            }
            OpenAiEndpoint::OpenRouterChat { url, model } => self
                .client
                .post(url)
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/json")
                .json(&build_openrouter_chat_request_body(request, &model)),
        };
        let response = builder.send().await?;

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum OpenAiEndpoint {
    Images { url: String },
    ImageEdits { url: String },
    OpenRouterChat { url: String, model: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenAiApiPlatform {
    OpenAi,
    OpenRouter,
}

impl OpenAiApiPlatform {
    fn from_config(value: &str) -> Self {
        match value.trim() {
            "openrouter" => Self::OpenRouter,
            _ => Self::OpenAi,
        }
    }

    fn default_endpoint(self) -> &'static str {
        match self {
            Self::OpenAi => DEFAULT_ENDPOINT,
            Self::OpenRouter => OPENROUTER_ENDPOINT,
        }
    }
}

fn resolve_endpoint(
    platform: OpenAiApiPlatform,
    base_url: &str,
    request_model: &str,
    has_references: bool,
) -> OpenAiEndpoint {
    if platform == OpenAiApiPlatform::OpenRouter {
        let api_base = base_url
            .trim_end_matches("/chat/completions")
            .trim_end_matches("/api/v1")
            .trim_end_matches('/');
        return OpenAiEndpoint::OpenRouterChat {
            url: format!("{api_base}/api/v1/chat/completions"),
            model: request_model.to_string(),
        };
    }

    if let Some(model) = openrouter_model_from_page_url(base_url) {
        return OpenAiEndpoint::OpenRouterChat {
            url: format!("{OPENROUTER_ENDPOINT}/chat/completions"),
            model,
        };
    }

    if base_url.contains("openrouter.ai") {
        let api_base = base_url
            .trim_end_matches("/chat/completions")
            .trim_end_matches("/api/v1")
            .trim_end_matches('/');
        return OpenAiEndpoint::OpenRouterChat {
            url: format!("{api_base}/api/v1/chat/completions"),
            model: request_model.to_string(),
        };
    }

    let suffix = if has_references {
        "images/edits"
    } else {
        "images/generations"
    };
    if has_references {
        OpenAiEndpoint::ImageEdits {
            url: format!("{}/{suffix}", base_url.trim_end_matches('/')),
        }
    } else {
        OpenAiEndpoint::Images {
            url: format!("{}/{suffix}", base_url.trim_end_matches('/')),
        }
    }
}

fn openrouter_model_from_page_url(base_url: &str) -> Option<String> {
    let marker = "openrouter.ai/";
    let path = base_url.split_once(marker)?.1.trim_matches('/');
    let mut segments = path.split('/');
    let provider = segments.next()?;
    let model = segments.next()?;
    let suffix = segments.next();

    if provider.is_empty() || model.is_empty() || suffix != Some("api") {
        return None;
    }

    Some(format!("{provider}/{model}"))
}

fn build_image_api_request_body(request: &GenerationRequest) -> Value {
    let mut body = json!({
        "model": request.model,
        "prompt": request.prompt.trim(),
        "n": request.batch_count,
        "output_format": "png"
    });

    if let Some(size) = request
        .options
        .image_size
        .as_deref()
        .and_then(normalize_openai_image_size)
    {
        body["size"] = json!(size);
    }
    if let Some(quality) = request.options.quality.as_deref() {
        if ["auto", "low", "medium", "high"].contains(&quality) {
            body["quality"] = json!(quality);
        }
    }

    body
}

fn build_openrouter_chat_request_body(request: &GenerationRequest, model: &str) -> Value {
    let content = build_openrouter_message_content(request);
    let mut body = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": content
            }
        ],
        "modalities": ["image", "text"],
        "stream": false
    });
    if let Some(image_config) = build_openrouter_image_config(request) {
        body["image_config"] = image_config;
    }
    body
}

fn build_openrouter_message_content(request: &GenerationRequest) -> Value {
    let references = reference_images(request);
    if references.is_empty() {
        return json!(request.prompt.trim());
    }

    let mut content = vec![json!({
        "type": "text",
        "text": request.prompt.trim()
    })];
    for image in references {
        content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", normalize_mime_type(&image.mime_type), image.data.trim())
            }
        }));
    }

    Value::Array(content)
}

fn apply_multipart_form(
    builder: RequestBuilder,
    request: &GenerationRequest,
) -> Result<RequestBuilder, OpenAiImageError> {
    let mut form = multipart::Form::new()
        .text("model", request.model.clone())
        .text("prompt", request.prompt.trim().to_string())
        .text("n", request.batch_count.to_string())
        .text("output_format", "png");

    if let Some(size) = request
        .options
        .image_size
        .as_deref()
        .and_then(normalize_openai_image_size)
    {
        form = form.text("size", size.to_string());
    }
    if let Some(quality) = request.options.quality.as_deref() {
        if ["auto", "low", "medium", "high"].contains(&quality) {
            form = form.text("quality", quality.to_string());
        }
    }

    for (index, image) in reference_images(request).iter().enumerate() {
        let bytes = decode_image_payload(&image.data)?;
        let part = multipart::Part::bytes(bytes)
            .file_name(reference_image_filename(image, index))
            .mime_str(normalize_mime_type(&image.mime_type))
            .map_err(|_| OpenAiImageError::InvalidImageData)?;
        form = form.part("image[]", part);
    }

    Ok(builder.multipart(form))
}

fn reference_images(request: &GenerationRequest) -> &[ReferenceImageInput] {
    request.reference_images.as_deref().unwrap_or(&[])
}

fn has_reference_images(request: &GenerationRequest) -> bool {
    !reference_images(request).is_empty()
}

fn build_openrouter_image_config(request: &GenerationRequest) -> Option<Value> {
    let size = request
        .options
        .image_size
        .as_deref()
        .and_then(parse_openrouter_image_config)?;
    Some(json!({
        "aspect_ratio": size.aspect_ratio,
        "image_size": size.image_size,
    }))
}

struct OpenRouterImageConfig<'a> {
    aspect_ratio: &'a str,
    image_size: &'a str,
}

fn parse_openrouter_image_config(size: &str) -> Option<OpenRouterImageConfig<'static>> {
    let size = size.trim();
    if size == "auto" {
        return None;
    }
    let (width, height) = parse_image_size(size)?;
    let aspect_ratio = openrouter_aspect_ratio(width, height)?;
    let image_size = match width.max(height) {
        0..=1536 => "1K",
        1537..=2560 => "2K",
        _ => "4K",
    };
    Some(OpenRouterImageConfig {
        aspect_ratio,
        image_size,
    })
}

fn openrouter_aspect_ratio(width: u64, height: u64) -> Option<&'static str> {
    let gcd = gcd(width, height);
    let ratio = (width / gcd, height / gcd);
    match ratio {
        (1, 1) => Some("1:1"),
        (2, 3) => Some("2:3"),
        (3, 2) => Some("3:2"),
        (3, 4) => Some("3:4"),
        (4, 3) => Some("4:3"),
        (4, 5) => Some("4:5"),
        (5, 4) => Some("5:4"),
        (9, 16) => Some("9:16"),
        (16, 9) => Some("16:9"),
        (21, 9) | (7, 3) => Some("21:9"),
        _ => None,
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

fn normalize_openai_image_size(size: &str) -> Option<&str> {
    let size = size.trim();
    if size == "auto" {
        return Some(size);
    }
    let (width, height) = parse_image_size(size)?;
    if is_valid_gpt_image_2_size(width, height) {
        Some(size)
    } else {
        None
    }
}

fn parse_image_size(size: &str) -> Option<(u64, u64)> {
    let (width, height) = size.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn is_valid_gpt_image_2_size(width: u64, height: u64) -> bool {
    if width == 0 || height == 0 {
        return false;
    }
    if width > 3840 || height > 3840 {
        return false;
    }
    if width % 16 != 0 || height % 16 != 0 {
        return false;
    }
    let long_edge = width.max(height);
    let short_edge = width.min(height);
    if long_edge > short_edge * 3 {
        return false;
    }
    let pixels = width * height;
    (655_360..=8_294_400).contains(&pixels)
}

fn normalize_mime_type(mime_type: &str) -> &str {
    match mime_type {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        _ => "image/png",
    }
}

fn reference_image_filename(image: &ReferenceImageInput, index: usize) -> String {
    let extension = match normalize_mime_type(&image.mime_type) {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    };
    let fallback = format!("reference-{}.{}", index + 1, extension);
    let name = image.name.trim();
    if name.is_empty() {
        fallback
    } else {
        name.to_string()
    }
}

fn parse_response(json: Value) -> Result<OpenAiImageResponse, OpenAiImageError> {
    let mut images = Vec::new();
    let mut response_format = "unknown";

    if let Some(items) = json.get("data").and_then(Value::as_array) {
        for item in items {
            if let Some(data) = item.get("b64_json").and_then(Value::as_str) {
                let bytes = decode_image_payload(data)?;
                images.push(OpenAiGeneratedImage { bytes });
                response_format = "image_api_data_b64_json";
            } else if let Some(data) = item.get("url").and_then(Value::as_str) {
                let bytes = decode_image_payload(data)?;
                images.push(OpenAiGeneratedImage { bytes });
                response_format = "image_api_data_url";
            }
        }
    }

    if let Some(items) = json.get("output").and_then(Value::as_array) {
        for item in items {
            if item.get("type").and_then(Value::as_str) != Some("image_generation_call") {
                continue;
            }
            let Some(data) = item.get("result").and_then(Value::as_str) else {
                continue;
            };
            let bytes = decode_image_payload(data)?;
            images.push(OpenAiGeneratedImage { bytes });
            response_format = "responses_image_generation_call";
        }
    }

    if let Some(choices) = json.get("choices").and_then(Value::as_array) {
        for choice in choices {
            let Some(message) = choice.get("message") else {
                continue;
            };
            let Some(message_images) = message.get("images").and_then(Value::as_array) else {
                continue;
            };
            for image in message_images {
                let data = image
                    .get("image_url")
                    .or_else(|| image.get("imageUrl"))
                    .and_then(|image_url| image_url.get("url"))
                    .and_then(Value::as_str);
                let Some(data) = data else {
                    continue;
                };
                let bytes = decode_image_payload(data)?;
                images.push(OpenAiGeneratedImage { bytes });
                response_format = "openrouter_chat_message_images";
            }
        }
    }

    let metadata = json!({
        "created": json.get("created").cloned().unwrap_or(Value::Null),
        "createdAt": json.get("created_at").cloned().unwrap_or(Value::Null),
        "id": json.get("id").cloned().unwrap_or(Value::Null),
        "model": json.get("model").cloned().unwrap_or(Value::Null),
        "usage": json.get("usage").cloned().unwrap_or(Value::Null),
        "responseFormat": response_format,
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

fn describe_reqwest_error(err: &reqwest::Error) -> String {
    let mut message = err.to_string();
    let mut source = err.source();

    while let Some(cause) = source {
        let detail = cause.to_string();
        if !detail.is_empty() && !message.contains(&detail) {
            message.push_str(": ");
            message.push_str(&detail);
        }
        source = cause.source();
    }

    if err.is_timeout() && !message.contains("Increase the OpenAI timeout") {
        message.push_str(
            ". Increase the OpenAI timeout in Settings if the provider is still generating.",
        );
    }

    message
}

#[cfg(test)]
mod tests {
    use super::{
        build_image_api_request_body, build_openrouter_chat_request_body, decode_image_payload,
        is_valid_gpt_image_2_size, parse_response, resolve_endpoint, OpenAiApiPlatform,
        OpenAiEndpoint,
    };
    use crate::models::{GenerationOptions, GenerationRequest, ReferenceImageInput};
    use base64::{engine::general_purpose, Engine as _};
    use serde_json::json;

    #[test]
    fn generation_payload_uses_gpt_image_2_options() {
        let body = build_image_api_request_body(&request());

        assert_eq!(body["model"], "gpt-image-2");
        assert_eq!(body["n"], 3);
        assert_eq!(body["output_format"], "png");
        assert_eq!(body["size"], "3840x2160");
        assert_eq!(body["quality"], "high");
    }

    #[test]
    fn gpt_image_2_size_validation_uses_documented_constraints() {
        assert!(is_valid_gpt_image_2_size(1024, 1024));
        assert!(is_valid_gpt_image_2_size(2048, 2048));
        assert!(is_valid_gpt_image_2_size(3840, 2160));
        assert!(is_valid_gpt_image_2_size(2160, 3840));
        assert!(is_valid_gpt_image_2_size(3840, 1280));

        assert!(!is_valid_gpt_image_2_size(3841, 2160));
        assert!(!is_valid_gpt_image_2_size(1025, 1024));
        assert!(!is_valid_gpt_image_2_size(3840, 1264));
        assert!(!is_valid_gpt_image_2_size(3840, 3840));
        assert!(!is_valid_gpt_image_2_size(512, 512));
    }

    #[test]
    fn openrouter_payload_uses_modalities() {
        let body = build_openrouter_chat_request_body(&request(), "openai/gpt-5.4-image-2");

        assert_eq!(body["model"], "openai/gpt-5.4-image-2");
        assert_eq!(body["messages"][0]["content"], "Render a test image");
        assert_eq!(body["modalities"], json!(["image", "text"]));
        assert_eq!(body["image_config"]["aspect_ratio"], "16:9");
        assert_eq!(body["image_config"]["image_size"], "4K");
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn openrouter_payload_maps_openai_size_to_image_config() {
        let body = build_openrouter_chat_request_body(
            &request_with_size("1024x1536"),
            "openai/gpt-5.4-image-2",
        );

        assert_eq!(body["image_config"]["aspect_ratio"], "2:3");
        assert_eq!(body["image_config"]["image_size"], "1K");
    }

    #[test]
    fn openrouter_model_page_url_routes_to_chat_completions() {
        let endpoint = resolve_endpoint(
            OpenAiApiPlatform::OpenAi,
            "https://openrouter.ai/openai/gpt-5.4-image-2/api",
            "gpt-image-2",
            false,
        );

        assert_eq!(
            endpoint,
            OpenAiEndpoint::OpenRouterChat {
                url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
                model: "openai/gpt-5.4-image-2".to_string(),
            }
        );
    }

    #[test]
    fn openrouter_platform_routes_api_base_to_chat_completions() {
        let endpoint = resolve_endpoint(
            OpenAiApiPlatform::OpenRouter,
            "https://openrouter.ai/api/v1",
            "openai/gpt-5.4-image-2",
            false,
        );

        assert_eq!(
            endpoint,
            OpenAiEndpoint::OpenRouterChat {
                url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
                model: "openai/gpt-5.4-image-2".to_string(),
            }
        );
    }

    #[test]
    fn openai_references_route_to_image_edits() {
        let endpoint = resolve_endpoint(
            OpenAiApiPlatform::OpenAi,
            "https://api.openai.com/v1",
            "gpt-image-2",
            true,
        );

        assert_eq!(
            endpoint,
            OpenAiEndpoint::ImageEdits {
                url: "https://api.openai.com/v1/images/edits".to_string(),
            }
        );
    }

    #[test]
    fn openrouter_payload_accepts_reference_images() {
        let body = build_openrouter_chat_request_body(
            &request_with_references(Some(vec![reference_image("image/png", "abc")])),
            "openai/gpt-5.4-image-2",
        );

        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(body["messages"][0]["content"][1]["type"], "image_url");
        assert_eq!(
            body["messages"][0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,abc"
        );
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
    fn parser_accepts_responses_image_generation_call() {
        let encoded = general_purpose::STANDARD.encode([9, 10, 11, 12]);
        let response = parse_response(json!({
            "output": [
                {
                    "type": "image_generation_call",
                    "result": encoded
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.images[0].bytes, vec![9, 10, 11, 12]);
        assert_eq!(
            response.metadata["responseFormat"],
            "responses_image_generation_call"
        );
    }

    #[test]
    fn parser_accepts_openrouter_chat_message_images() {
        let encoded = general_purpose::STANDARD.encode([13, 14, 15, 16]);
        let response = parse_response(json!({
            "choices": [
                {
                    "message": {
                        "images": [
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:image/png;base64,{encoded}")
                                }
                            }
                        ]
                    }
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.images[0].bytes, vec![13, 14, 15, 16]);
        assert_eq!(
            response.metadata["responseFormat"],
            "openrouter_chat_message_images"
        );
    }

    #[test]
    fn parser_accepts_data_url_base64() {
        let encoded = general_purpose::STANDARD.encode([5, 6, 7, 8]);
        let bytes = decode_image_payload(&format!("data:image/png;base64,{encoded}")).unwrap();

        assert_eq!(bytes, vec![5, 6, 7, 8]);
    }

    fn request() -> GenerationRequest {
        request_with_references(None)
    }

    fn request_with_references(
        reference_images: Option<Vec<ReferenceImageInput>>,
    ) -> GenerationRequest {
        request_with_references_and_size(reference_images, "3840x2160")
    }

    fn request_with_size(image_size: &str) -> GenerationRequest {
        request_with_references_and_size(None, image_size)
    }

    fn request_with_references_and_size(
        reference_images: Option<Vec<ReferenceImageInput>>,
        image_size: &str,
    ) -> GenerationRequest {
        GenerationRequest {
            task_id: None,
            provider: "gpt-image".to_string(),
            model: "gpt-image-2".to_string(),
            prompt: "Render a test image".to_string(),
            prompt_snapshot: None,
            batch_count: 3,
            reference_images,
            output_template: "{id}.{extension}".to_string(),
            options: GenerationOptions {
                aspect_ratio: Some("3:2".to_string()),
                image_size: Some(image_size.to_string()),
                temperature: None,
                top_p: None,
                thinking_level: None,
                quality: Some("high".to_string()),
                unlimited: None,
            },
            base_url: None,
        }
    }

    fn reference_image(mime_type: &str, data: &str) -> ReferenceImageInput {
        ReferenceImageInput {
            name: "reference.png".to_string(),
            mime_type: mime_type.to_string(),
            data: data.to_string(),
            asset_id: None,
        }
    }
}
