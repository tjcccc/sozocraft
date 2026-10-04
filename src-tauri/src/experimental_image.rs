//! Experimental image mode: trial models from other providers, all routed
//! through OpenRouter's Images API (`/api/v1/images`). A model that proves useful can later be
//! promoted to its own image mode.
//!
//! This module owns the experimental model capability table, so desktop and
//! CLI validation stay identical.

use crate::{
    models::{AppSettings, GenerationRequest, ReferenceImageInput},
    openai_image,
};
use reqwest::{redirect, Client, Proxy, Url};
use serde_json::{json, Value};
use std::time::Duration;

pub const PROVIDER_ID: &str = "experimental";
pub const PLATFORM: &str = "openrouter";
const OPENROUTER_API_BASE: &str = "https://openrouter.ai/api/v1";
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

pub struct ExperimentalImageModel {
    pub id: &'static str,
    /// Provider segment used in output filenames.
    pub filename_provider: &'static str,
    /// Model segment used in output filenames.
    pub filename_model: &'static str,
    /// Aspect ratio choices and the `size` sent for each; `None` lets the
    /// model pick its default shape.
    pub aspect_ratios: &'static [(&'static str, Option<&'static str>)],
    pub max_reference_images: usize,
    /// OpenRouter provider slug that receives `moderation: "low"`, matching
    /// the app's GPT Image policy; `None` leaves moderation at the default.
    pub low_moderation_provider: Option<&'static str>,
}

impl ExperimentalImageModel {
    pub fn supports_aspect_ratio(&self, ratio: &str) -> bool {
        self.aspect_ratios.iter().any(|(value, _)| *value == ratio)
    }

    fn size_for_aspect_ratio(&self, ratio: &str) -> Option<&'static str> {
        self.aspect_ratios
            .iter()
            .find(|(value, _)| *value == ratio)
            .and_then(|(_, size)| *size)
    }
}

pub const MODELS: &[ExperimentalImageModel] = &[ExperimentalImageModel {
    id: "meta/muse-image",
    filename_provider: "meta",
    filename_model: "muse-image",
    // Muse treats `aspect_ratio` as a loose hint (it snapped 16:9 to 3:2 in
    // live tests), but honors a `WxH` size as the target shape at its own
    // resolution (1920x1080 returned 2048x1152).
    aspect_ratios: &[
        ("auto", None),
        ("1:1", Some("1024x1024")),
        ("2:3", Some("1024x1536")),
        ("3:2", Some("1536x1024")),
        ("3:4", Some("1152x1536")),
        ("4:3", Some("1536x1152")),
        ("9:16", Some("1080x1920")),
        ("16:9", Some("1920x1080")),
        ("21:9", Some("2016x864")),
        ("9:21", Some("864x2016")),
    ],
    max_reference_images: 10,
    // Meta's native API accepts `auto` and `low`. OpenRouter currently drops
    // this option for Muse (an invalid value still returned 200), so it has no
    // effect today; it is kept to match the GPT Image policy if forwarding lands.
    low_moderation_provider: Some("meta"),
}];

pub const DEFAULT_MODEL: &str = "meta/muse-image";

pub fn model(id: &str) -> Option<&'static ExperimentalImageModel> {
    MODELS.iter().find(|model| model.id == id)
}

pub fn model_ids() -> impl Iterator<Item = &'static str> {
    MODELS.iter().map(|model| model.id)
}

/// Rejects options the selected experimental model does not accept. Shared by
/// the desktop command and the CLI.
pub fn validate_request(request: &GenerationRequest) -> Result<(), String> {
    let Some(model) = model(&request.model) else {
        return Err(format!(
            "Unsupported experimental image model: {}",
            request.model
        ));
    };
    let options = &request.options;
    if let Some(ratio) = options
        .aspect_ratio
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        if !model.supports_aspect_ratio(ratio) {
            return Err(format!(
                "Unsupported aspect ratio for {}: {ratio}",
                model.id
            ));
        }
    }
    if options
        .image_size
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Err(format!(
            "{} does not accept an image size option.",
            model.id
        ));
    }
    if options
        .quality
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Err(format!("{} does not accept a quality option.", model.id));
    }
    if options
        .thinking_level
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Err(format!(
            "{} does not accept a thinking level option.",
            model.id
        ));
    }
    Ok(())
}

pub struct ExperimentalImageResponse {
    pub images: Vec<Vec<u8>>,
    pub metadata: Value,
}

/// Generates one image. The OpenRouter endpoint comes from Rust-owned
/// settings only; a renderer-supplied base URL is never used for this key.
pub async fn generate(
    request: &GenerationRequest,
    settings: &AppSettings,
    api_key: &str,
) -> Result<ExperimentalImageResponse, String> {
    validate_request(request)?;
    let url = images_url(settings.openrouter_base_url.as_deref())?;
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(
            settings.openrouter_timeout_seconds.max(10),
        ))
        .redirect(redirect::Policy::none())
        .http1_only()
        .pool_max_idle_per_host(0);
    if let Some(proxy_url) = settings.openrouter_proxy_url() {
        builder =
            builder.proxy(Proxy::all(proxy_url).map_err(|_| "Invalid proxy URL.".to_string())?);
    }
    let client = builder
        .build()
        .map_err(|error| format!("Could not create OpenRouter client: {error}"))?;

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&build_request_body(request))
        .send()
        .await
        .map_err(describe_request_error)?;
    let status = response.status();
    let body = response.bytes().await.map_err(describe_request_error)?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err("OpenRouter response exceeded the 64 MiB limit.".to_string());
    }
    let text = String::from_utf8_lossy(&body);
    if !status.is_success() {
        return Err(format!(
            "OpenRouter returned an error ({status}): {}",
            text.chars().take(2000).collect::<String>()
        ));
    }
    let json: Value = serde_json::from_str(&text)
        .map_err(|_| "OpenRouter returned a response that is not JSON.".to_string())?;
    let parsed = openai_image::parse_response(json).map_err(|error| error.to_string())?;
    Ok(ExperimentalImageResponse {
        images: parsed.images.into_iter().map(|image| image.bytes).collect(),
        metadata: parsed.metadata,
    })
}

fn images_url(base_url: Option<&str>) -> Result<Url, String> {
    let base = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(OPENROUTER_API_BASE)
        .trim_end_matches('/')
        .trim_end_matches("/chat/completions")
        .trim_end_matches("/images");
    let url = Url::parse(&format!("{base}/images"))
        .map_err(|_| "OpenRouter base URL is not a valid URL.".to_string())?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    // Plain http is allowed only for local gateways and test mocks.
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        return Err("OpenRouter base URL must use https (http only for localhost).".to_string());
    }
    Ok(url)
}

fn build_request_body(request: &GenerationRequest) -> Value {
    let mut body = json!({
        "model": request.model,
        "prompt": request.prompt.trim(),
        "n": 1,
        "output_format": "png",
    });
    let size = request
        .options
        .aspect_ratio
        .as_deref()
        .zip(model(&request.model))
        .and_then(|(ratio, model)| model.size_for_aspect_ratio(ratio));
    if let Some(size) = size {
        body["size"] = json!(size);
    }
    if let Some(provider) = model(&request.model).and_then(|model| model.low_moderation_provider) {
        body["provider"] = json!({ "options": { provider: { "moderation": "low" } } });
    }
    let references = request.reference_images.as_deref().unwrap_or_default();
    if !references.is_empty() {
        body["input_references"] = Value::Array(references.iter().map(reference_part).collect());
    }
    body
}

fn reference_part(image: &ReferenceImageInput) -> Value {
    let mime_type = match image.mime_type.as_str() {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        _ => "image/png",
    };
    json!({
        "type": "image_url",
        "image_url": { "url": format!("data:{mime_type};base64,{}", image.data.trim()) },
    })
}

fn describe_request_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        return "OpenRouter request timed out. Muse-style reasoning models can take minutes; \
                increase the OpenRouter timeout in Settings > Platforms."
            .to_string();
    }
    // Strip the URL so query details never reach logs.
    format!("OpenRouter request failed: {}", error.without_url())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GenerationOptions;

    fn request() -> GenerationRequest {
        GenerationRequest {
            task_id: None,
            provider: PROVIDER_ID.to_string(),
            model: DEFAULT_MODEL.to_string(),
            prompt: "  A paper lantern festival  ".to_string(),
            prompt_snapshot: None,
            batch_count: 1,
            reference_images: None,
            output_template: "{id}.{extension}".to_string(),
            base_url: None,
            options: GenerationOptions {
                aspect_ratio: Some("16:9".to_string()),
                image_size: None,
                temperature: None,
                top_p: None,
                thinking_level: None,
                quality: None,
                unlimited: None,
            },
        }
    }

    #[test]
    fn builds_images_api_request_with_aspect_ratio() {
        let body = build_request_body(&request());
        assert_eq!(body["model"], "meta/muse-image");
        assert_eq!(body["prompt"], "A paper lantern festival");
        assert_eq!(body["n"], 1);
        assert_eq!(body["output_format"], "png");
        assert_eq!(body["provider"]["options"]["meta"]["moderation"], "low");
        assert_eq!(body["size"], "1920x1080");
        assert!(body.get("aspect_ratio").is_none());

        let mut auto = request();
        auto.options.aspect_ratio = Some("auto".to_string());
        assert!(build_request_body(&auto).get("size").is_none());
        assert!(body.get("input_references").is_none());
    }

    #[test]
    fn sends_references_as_data_url_parts() {
        let mut value = request();
        value.options.aspect_ratio = None;
        value.reference_images = Some(vec![ReferenceImageInput {
            name: "ref.jpg".to_string(),
            mime_type: "image/jpg".to_string(),
            data: "QUJD".to_string(),
            asset_id: None,
        }]);
        let body = build_request_body(&value);
        let references = body["input_references"].as_array().unwrap();
        assert_eq!(references[0]["type"], "image_url");
        assert_eq!(
            references[0]["image_url"]["url"],
            "data:image/jpeg;base64,QUJD"
        );
        assert!(body.get("size").is_none());
    }

    #[test]
    fn validation_rejects_unsupported_options() {
        assert!(validate_request(&request()).is_ok());

        let mut ratio = request();
        ratio.options.aspect_ratio = Some("5:4".to_string());
        assert!(validate_request(&ratio).is_err());

        let mut size = request();
        size.options.image_size = Some("2K".to_string());
        assert!(validate_request(&size).is_err());

        let mut quality = request();
        quality.options.quality = Some("high".to_string());
        assert!(validate_request(&quality).is_err());

        let mut model = request();
        model.model = "openai/gpt-image-2".to_string();
        assert!(validate_request(&model).is_err());
    }

    #[test]
    fn endpoint_requires_https_and_defaults_to_openrouter() {
        assert_eq!(
            images_url(None).unwrap().as_str(),
            "https://openrouter.ai/api/v1/images"
        );
        assert_eq!(
            images_url(Some("https://gateway.example/api/v1/"))
                .unwrap()
                .as_str(),
            "https://gateway.example/api/v1/images"
        );
        assert_eq!(
            images_url(Some("https://openrouter.ai/api/v1/chat/completions"))
                .unwrap()
                .as_str(),
            "https://openrouter.ai/api/v1/images"
        );
        assert!(images_url(Some("http://openrouter.ai/api/v1")).is_err());
        assert!(images_url(Some("http://127.0.0.1:8080/api/v1")).is_ok());
        assert!(images_url(Some("ftp://openrouter.ai/api/v1")).is_err());
        assert!(images_url(Some("not a url")).is_err());
    }
}
