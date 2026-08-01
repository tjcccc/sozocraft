use crate::gemini_models::max_reference_images;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub settings: AppSettings,
    pub current_prompt: String,
    #[serde(default)]
    pub current_prompt_id: Option<String>,
    pub batches: Vec<GenerationBatch>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GenerationMediaType {
    #[default]
    Image,
    Video,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: AppSettings::default(),
            current_prompt: "Create a cinematic portrait with realistic lighting.".to_string(),
            current_prompt_id: None,
            batches: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_provider")]
    pub default_provider: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_output_directory")]
    pub output_directory: String,
    #[serde(default = "default_output_template")]
    pub output_template: String,
    #[serde(default = "default_prompt_directory")]
    pub prompt_directory: String,
    #[serde(default = "default_prompt_dsl_enabled")]
    pub prompt_dsl_enabled: bool,
    #[serde(default)]
    pub prompt_editor_only: bool,
    #[serde(default = "default_prompt_preview_placement")]
    pub prompt_preview_placement: String,
    #[serde(default = "default_nano_banana_api_platform")]
    pub nano_banana_api_platform: String,
    #[serde(default = "default_proxy_enabled")]
    pub gemini_proxy_enabled: bool,
    #[serde(default = "default_openai_api_platform")]
    pub openai_api_platform: String,
    #[serde(default = "default_proxy_enabled")]
    pub openai_proxy_enabled: bool,
    #[serde(default = "default_grok_api_platform")]
    pub grok_api_platform: String,
    #[serde(default = "default_proxy_enabled")]
    pub xai_proxy_enabled: bool,
    #[serde(default = "default_seedance_api_platform")]
    pub seedance_api_platform: String,
    #[serde(default = "default_seedance_video_model")]
    pub seedance_default_model: String,
    #[serde(default)]
    pub ark_proxy_enabled: bool,
    #[serde(default)]
    pub higgsfield_cli_path: Option<String>,
    #[serde(default)]
    pub optional_base_url: Option<String>,
    #[serde(default)]
    pub openai_base_url: Option<String>,
    #[serde(default)]
    pub openrouter_base_url: Option<String>,
    #[serde(default)]
    pub xai_base_url: Option<String>,
    #[serde(default)]
    pub ark_base_url: Option<String>,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_timeout_seconds")]
    pub gemini_timeout_seconds: u64,
    #[serde(default = "default_timeout_seconds")]
    pub openai_timeout_seconds: u64,
    #[serde(default = "default_timeout_seconds")]
    pub xai_timeout_seconds: u64,
    #[serde(default = "default_timeout_seconds")]
    pub ark_timeout_seconds: u64,
}

fn default_provider() -> String {
    "nano-banana".to_string()
}

fn default_model() -> String {
    "gemini-3-pro-image-preview".to_string()
}

fn default_output_template() -> String {
    "{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}".to_string()
}

fn default_timeout_seconds() -> u64 {
    180
}

fn default_output_directory() -> String {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("SozoCraft")
        .to_string_lossy()
        .to_string()
}

fn default_prompt_directory() -> String {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sozocraft")
        .join("prompts")
        .to_string_lossy()
        .to_string()
}

fn default_prompt_dsl_enabled() -> bool {
    true
}

fn default_prompt_preview_placement() -> String {
    "bottom".to_string()
}

fn default_proxy_enabled() -> bool {
    true
}

fn default_nano_banana_api_platform() -> String {
    "gemini".to_string()
}

fn default_openai_api_platform() -> String {
    "openai".to_string()
}

fn default_grok_api_platform() -> String {
    "xai".to_string()
}

fn default_seedance_api_platform() -> String {
    "ark".to_string()
}

fn default_seedance_video_model() -> String {
    "doubao-seedance-2-0-260128".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_provider: "nano-banana".to_string(),
            default_model: "gemini-3-pro-image-preview".to_string(),
            output_directory: default_output_directory(),
            output_template: "{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}"
                .to_string(),
            prompt_directory: default_prompt_directory(),
            prompt_dsl_enabled: true,
            prompt_editor_only: false,
            prompt_preview_placement: default_prompt_preview_placement(),
            nano_banana_api_platform: default_nano_banana_api_platform(),
            gemini_proxy_enabled: true,
            openai_api_platform: default_openai_api_platform(),
            openai_proxy_enabled: true,
            grok_api_platform: default_grok_api_platform(),
            xai_proxy_enabled: true,
            seedance_api_platform: default_seedance_api_platform(),
            seedance_default_model: default_seedance_video_model(),
            ark_proxy_enabled: false,
            higgsfield_cli_path: None,
            optional_base_url: None,
            openai_base_url: None,
            openrouter_base_url: None,
            xai_base_url: None,
            ark_base_url: None,
            proxy_url: None,
            timeout_seconds: 180,
            gemini_timeout_seconds: 180,
            openai_timeout_seconds: 180,
            xai_timeout_seconds: 180,
            ark_timeout_seconds: 180,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
    #[serde(default)]
    pub task_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub prompt_snapshot: Option<String>,
    pub batch_count: u32,
    pub reference_images: Option<Vec<ReferenceImageInput>>,
    pub output_template: String,
    pub options: GenerationOptions,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoGenerationRequest {
    #[serde(default)]
    pub task_id: Option<String>,
    pub provider: VideoProvider,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub prompt_snapshot: Option<String>,
    #[serde(default)]
    pub input_mode: VideoInputMode,
    #[serde(default)]
    pub starting_image: Option<ReferenceImageInput>,
    #[serde(default)]
    pub ending_image: Option<ReferenceImageInput>,
    #[serde(default)]
    pub reference_images: Option<Vec<ReferenceImageInput>>,
    pub options: VideoGenerationOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoProvider {
    Seedance,
    GrokImagine,
    GoogleVeo,
}

impl VideoProvider {
    pub fn id(self) -> &'static str {
        match self {
            Self::Seedance => "seedance",
            Self::GrokImagine => "grok-imagine",
            Self::GoogleVeo => "google-veo",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoInputMode {
    #[default]
    Text,
    Image,
    Frames,
    Reference,
}

impl VideoGenerationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(task_id) = self.task_id.as_deref() {
            if uuid::Uuid::parse_str(task_id).is_err() {
                return Err("Video task id must be a UUID.".to_string());
            }
        }
        let supported_models: &[&str] = match self.provider {
            VideoProvider::Seedance => &[
                "doubao-seedance-2-0-260128",
                "doubao-seedance-2-0-fast-260128",
                "doubao-seedance-2-0-mini-260615",
            ],
            VideoProvider::GrokImagine => &["grok-imagine-video"],
            VideoProvider::GoogleVeo => &["veo-3.1-generate-preview"],
        };
        if !supported_models.contains(&self.model.as_str()) {
            return Err(format!(
                "Unsupported {} video model: {}",
                self.provider.id(),
                self.model
            ));
        }
        let prompt = self.prompt.trim();
        if prompt.is_empty() {
            return Err("Enter a prompt before running video generation.".to_string());
        }
        if prompt.len() > 50_000 {
            return Err("Video prompt is too large.".to_string());
        }
        if self
            .prompt_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.len() > 2 * 1024 * 1024)
        {
            return Err("Video prompt snapshot is too large.".to_string());
        }
        self.options.validate(self.provider, &self.model)?;

        let reference_images = self.reference_images.as_deref().unwrap_or_default();
        match self.input_mode {
            VideoInputMode::Text => {
                if self.starting_image.is_some()
                    || self.ending_image.is_some()
                    || !reference_images.is_empty()
                {
                    return Err("Text-to-video does not accept input images.".to_string());
                }
            }
            VideoInputMode::Image => {
                let Some(image) = &self.starting_image else {
                    return Err("Image-to-video requires one starting image.".to_string());
                };
                if !reference_images.is_empty() {
                    return Err(
                        "A starting image cannot be combined with reference images.".to_string()
                    );
                }
                if self.ending_image.is_some() {
                    return Err("Starting-frame video cannot include an ending image.".to_string());
                }
                validate_video_input_image(image, "starting", self.provider)?;
            }
            VideoInputMode::Frames => {
                if self.provider == VideoProvider::GrokImagine {
                    return Err(
                        "Grok Imagine does not support start-and-end-frame video.".to_string()
                    );
                }
                let Some(starting_image) = &self.starting_image else {
                    return Err(
                        "Start-and-end-frame video requires one starting image.".to_string()
                    );
                };
                let Some(ending_image) = &self.ending_image else {
                    return Err("Start-and-end-frame video requires one ending image.".to_string());
                };
                if !reference_images.is_empty() {
                    return Err(
                        "Start-and-end-frame video cannot be combined with reference images."
                            .to_string(),
                    );
                }
                validate_video_input_image(starting_image, "starting", self.provider)?;
                validate_video_input_image(ending_image, "ending", self.provider)?;
            }
            VideoInputMode::Reference => {
                if self.starting_image.is_some() || self.ending_image.is_some() {
                    return Err(
                        "Reference-to-video cannot be combined with frame images.".to_string()
                    );
                }
                let max_images = max_video_reference_images(self.provider);
                if reference_images.is_empty() || reference_images.len() > max_images {
                    return Err(format!(
                        "{} reference-to-video requires between 1 and {max_images} reference images.",
                        self.provider.id()
                    ));
                }
                if self.provider == VideoProvider::GrokImagine
                    && self.options.duration > MAX_GROK_VIDEO_REFERENCE_DURATION
                {
                    return Err(format!(
                        "Grok Imagine reference-to-video duration must be between 1 and {MAX_GROK_VIDEO_REFERENCE_DURATION} seconds."
                    ));
                }
                if self.provider == VideoProvider::GoogleVeo && self.options.duration != 8 {
                    return Err(
                        "Google Veo reference-to-video requires an 8-second duration.".to_string(),
                    );
                }
                for image in reference_images {
                    validate_video_input_image(image, "reference", self.provider)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoGenerationOptions {
    pub duration: u8,
    pub aspect_ratio: String,
    pub resolution: String,
    #[serde(default)]
    pub generate_audio: Option<bool>,
}

impl VideoGenerationOptions {
    fn validate(&self, provider: VideoProvider, model: &str) -> Result<(), String> {
        let (durations, aspect_ratios, resolutions): (&[u8], &[&str], &[&str]) = match provider {
            VideoProvider::Seedance => {
                let resolutions: &[&str] = match model {
                    "doubao-seedance-2-0-260128" => &["480p", "720p", "1080p"],
                    "doubao-seedance-2-0-fast-260128" | "doubao-seedance-2-0-mini-260615" => {
                        &["480p", "720p"]
                    }
                    _ => &[],
                };
                (
                    &[4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                    &["16:9", "9:16", "4:3", "3:4", "1:1", "21:9"],
                    resolutions,
                )
            }
            VideoProvider::GrokImagine => (
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                &["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"],
                &["480p", "720p"],
            ),
            VideoProvider::GoogleVeo => (&[4, 6, 8], &["16:9", "9:16"], &["720p", "1080p", "4k"]),
        };
        if !durations.contains(&self.duration) {
            return Err(format!(
                "Unsupported {} video duration: {} seconds.",
                provider.id(),
                self.duration
            ));
        }
        if !aspect_ratios.contains(&self.aspect_ratio.as_str()) {
            return Err(format!(
                "Unsupported {} video aspect ratio: {}",
                provider.id(),
                self.aspect_ratio
            ));
        }
        if !resolutions.contains(&self.resolution.as_str()) {
            return Err(format!(
                "Unsupported {} video resolution: {}",
                provider.id(),
                self.resolution
            ));
        }
        if provider == VideoProvider::GoogleVeo && self.resolution != "720p" && self.duration != 8 {
            return Err(
                "Google Veo 1080p and 4K generation requires an 8-second duration.".to_string(),
            );
        }
        if provider != VideoProvider::Seedance && self.generate_audio.is_some() {
            return Err(format!(
                "{} does not accept an audio-generation option.",
                provider.id()
            ));
        }
        Ok(())
    }
}

impl GenerationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if !["nano-banana", "gpt-image", "grok-imagine"].contains(&self.provider.as_str()) {
            return Err(format!("Unsupported image provider: {}", self.provider));
        }
        if self.prompt.trim().is_empty()
            && self
                .reference_images
                .as_ref()
                .map(|items| items.is_empty())
                .unwrap_or(true)
        {
            return Err("Enter a prompt before running generation.".to_string());
        }
        if !(1..=8).contains(&self.batch_count) {
            return Err("Batch count must be between 1 and 8.".to_string());
        }
        if !supported_models(&self.provider).contains(&self.model.as_str()) {
            return Err(format!(
                "Unsupported {} image model: {}",
                self.provider, self.model
            ));
        }
        if !["nano-banana", "gpt-image", "grok-imagine"].contains(&self.provider.as_str())
            && self
                .reference_images
                .as_ref()
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        {
            return Err(
                "Reference images are currently implemented for image providers only.".to_string(),
            );
        }
        if self
            .reference_images
            .as_ref()
            .map(|items| {
                items.len() > max_reference_images_for_provider(&self.provider, &self.model)
            })
            .unwrap_or(false)
        {
            return Err(format!(
                "{} supports at most {} reference images.",
                self.model,
                max_reference_images_for_provider(&self.provider, &self.model)
            ));
        }
        if self.output_template.trim().is_empty() {
            return Err("Output filename template cannot be empty.".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceImageInput {
    pub name: String,
    pub mime_type: String,
    pub data: String,
    #[serde(default)]
    pub asset_id: Option<String>,
}

const MAX_VIDEO_INPUT_IMAGE_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_GROK_VIDEO_REFERENCE_DURATION: u8 = 10;

fn max_video_reference_images(provider: VideoProvider) -> usize {
    match provider {
        VideoProvider::Seedance => 9,
        VideoProvider::GrokImagine => 7,
        VideoProvider::GoogleVeo => 3,
    }
}

fn validate_video_input_image(
    image: &ReferenceImageInput,
    label: &str,
    provider: VideoProvider,
) -> Result<(), String> {
    if let Some(asset_id) = image.asset_id.as_deref() {
        if provider != VideoProvider::Seedance || label != "reference" {
            return Err("Ark assets can only be used as Seedance reference images.".to_string());
        }
        validate_ark_asset_id(asset_id)?;
        if !image.data.trim().is_empty() {
            return Err("Ark asset references cannot include inline image data.".to_string());
        }
        return Ok(());
    }
    if image.name.trim().is_empty() || image.name.len() > 255 {
        return Err(format!("Video {label} image must have a valid filename."));
    }
    let mime_type = image.mime_type.trim().to_ascii_lowercase();
    if !["image/png", "image/jpeg", "image/jpg", "image/webp"].contains(&mime_type.as_str()) {
        return Err(format!("Video {label} image must be PNG, JPEG, or WebP."));
    }
    if provider == VideoProvider::GoogleVeo && mime_type == "image/webp" {
        return Err("Google Veo input images must be PNG or JPEG.".to_string());
    }
    let encoded = image.data.trim();
    if encoded.is_empty() || encoded.len() > MAX_VIDEO_INPUT_IMAGE_BYTES.saturating_mul(4) / 3 + 4 {
        return Err(format!("Video {label} image is empty or too large."));
    }
    let bytes = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| format!("Video {label} image contains invalid base64 data."))?;
    if bytes.is_empty() || bytes.len() > MAX_VIDEO_INPUT_IMAGE_BYTES {
        return Err(format!("Video {label} image is empty or too large."));
    }
    let signature_matches = match mime_type.as_str() {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" | "image/jpg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/webp" => {
            bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
        }
        _ => false,
    };
    if !signature_matches {
        return Err(format!(
            "Video {label} image data does not match its file type."
        ));
    }
    Ok(())
}

pub(crate) fn validate_ark_asset_id(id: &str) -> Result<(), String> {
    if !id.starts_with("asset-")
        || id.len() > 128
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("Invalid Volcengine Ark asset id.".to_string());
    }
    Ok(())
}

pub const HIGGSFIELD_NANO_BANANA_MODELS: [&str; 3] =
    ["nano_banana_2", "nano_banana_flash", "nano_banana"];

pub const OPENAI_IMAGE_MODELS: [&str; 3] = ["gpt-image-2", "openai/gpt-5.4-image-2", "gpt_image_2"];

pub const XAI_IMAGE_MODELS: [&str; 3] = [
    "grok-imagine-image-quality",
    "grok-imagine-image",
    "grok_image",
];

fn supported_models(provider: &str) -> &'static [&'static str] {
    match provider {
        "nano-banana" => &NANO_BANANA_MODELS,
        "gpt-image" => &OPENAI_IMAGE_MODELS,
        "grok-imagine" => &XAI_IMAGE_MODELS,
        _ => &[],
    }
}

pub const NANO_BANANA_MODELS: [&str; 6] = [
    "gemini-3-pro-image-preview",
    "gemini-3.1-flash-image-preview",
    "gemini-2.5-flash-image",
    "nano_banana_2",
    "nano_banana_flash",
    "nano_banana",
];

fn max_reference_images_for_provider(provider: &str, model: &str) -> usize {
    match provider {
        "gpt-image" => 16,
        "grok-imagine" => 5,
        "nano-banana" if HIGGSFIELD_NANO_BANANA_MODELS.contains(&model) => 8,
        _ => max_reference_images(model),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationOptions {
    pub aspect_ratio: Option<String>,
    pub image_size: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub thinking_level: Option<String>,
    pub quality: Option<String>,
    pub unlimited: Option<bool>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            aspect_ratio: Some("16:9".to_string()),
            image_size: Some("1K".to_string()),
            temperature: Some(0.5),
            top_p: Some(0.95),
            thinking_level: Some("MINIMAL".to_string()),
            quality: None,
            unlimited: Some(false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputImage {
    pub id: String,
    pub batch_id: String,
    pub provider: String,
    pub model: String,
    pub path: String,
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub prompt_snapshot: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputVideo {
    pub id: String,
    pub batch_id: String,
    pub provider: String,
    pub model: String,
    pub path: String,
    pub filename: String,
    pub metadata_path: String,
    pub created_at: DateTime<Utc>,
    pub prompt_snapshot: String,
    pub duration: u8,
    pub aspect_ratio: String,
    pub resolution: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationBatch {
    pub id: String,
    #[serde(default)]
    pub media_type: GenerationMediaType,
    pub provider: String,
    pub model: String,
    pub prompt_snapshot: String,
    pub status: GenerationStatus,
    #[serde(default)]
    pub images: Vec<OutputImage>,
    #[serde(default)]
    pub videos: Vec<OutputVideo>,
    #[serde(default)]
    pub provider_request_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[cfg(test)]
mod video_tests {
    use super::{
        GenerationBatch, GenerationMediaType, ReferenceImageInput, VideoGenerationOptions,
        VideoGenerationRequest, VideoInputMode, VideoProvider,
    };
    use base64::{engine::general_purpose, Engine as _};
    use serde_json::json;

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            task_id: Some(uuid::Uuid::new_v4().to_string()),
            provider: VideoProvider::GrokImagine,
            model: "grok-imagine-video".to_string(),
            prompt: "A calm lake at sunrise".to_string(),
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

    #[test]
    fn video_request_accepts_documented_text_to_video_options() {
        assert!(request().validate().is_ok());
    }

    #[test]
    fn video_request_rejects_unsupported_model_and_options() {
        let mut value = request();
        value.model = "grok-imagine-video-1.5".to_string();
        assert!(value
            .validate()
            .unwrap_err()
            .contains("Unsupported grok-imagine video model"));

        let mut value = request();
        value.options.duration = 16;
        assert!(value.validate().unwrap_err().contains("duration"));

        let mut value = request();
        value.options.resolution = "1080p".to_string();
        assert!(value.validate().unwrap_err().contains("resolution"));
    }

    #[test]
    fn video_request_rejects_oversized_prompt_snapshot() {
        let mut value = request();
        value.prompt_snapshot = Some("x".repeat(2 * 1024 * 1024 + 1));

        assert!(value
            .validate()
            .unwrap_err()
            .contains("prompt snapshot is too large"));
    }

    #[test]
    fn video_request_accepts_one_valid_starting_image() {
        let mut value = request();
        value.input_mode = VideoInputMode::Image;
        value.starting_image = Some(valid_image("starting.png"));

        assert!(value.validate().is_ok());
    }

    #[test]
    fn video_request_rejects_invalid_starting_image_data() {
        let mut value = request();
        value.input_mode = VideoInputMode::Image;
        value.starting_image = Some(ReferenceImageInput {
            name: "starting.png".to_string(),
            mime_type: "image/png".to_string(),
            data: "not-base64".to_string(),
            asset_id: None,
        });

        assert!(value.validate().unwrap_err().contains("invalid base64"));
    }

    #[test]
    fn video_request_accepts_seven_references_at_ten_seconds() {
        let mut value = request();
        value.input_mode = VideoInputMode::Reference;
        value.options.duration = 10;
        value.reference_images = Some(
            (0..7)
                .map(|index| valid_image(&format!("reference-{index}.png")))
                .collect(),
        );

        assert!(value.validate().is_ok());
    }

    #[test]
    fn video_request_enforces_reference_count_and_duration_limits() {
        let mut value = request();
        value.input_mode = VideoInputMode::Reference;
        value.reference_images = Some(
            (0..8)
                .map(|index| valid_image(&format!("reference-{index}.png")))
                .collect(),
        );
        assert!(value.validate().unwrap_err().contains("between 1 and 7"));

        value.reference_images = Some(vec![valid_image("reference.png")]);
        value.options.duration = 11;
        assert!(value.validate().unwrap_err().contains("between 1 and 10"));
    }

    #[test]
    fn video_request_rejects_mixed_starting_and_reference_images() {
        let mut value = request();
        value.input_mode = VideoInputMode::Reference;
        value.starting_image = Some(valid_image("starting.png"));
        value.reference_images = Some(vec![valid_image("reference.png")]);

        assert!(value.validate().unwrap_err().contains("cannot be combined"));
    }

    #[test]
    fn seedance_accepts_nine_references_and_audio_at_fifteen_seconds() {
        let mut value = request();
        value.provider = VideoProvider::Seedance;
        value.model = "doubao-seedance-2-0-260128".to_string();
        value.input_mode = VideoInputMode::Reference;
        value.reference_images = Some(
            (0..9)
                .map(|index| valid_image(&format!("reference-{index}.png")))
                .collect(),
        );
        value.options = VideoGenerationOptions {
            duration: 15,
            aspect_ratio: "21:9".to_string(),
            resolution: "1080p".to_string(),
            generate_audio: Some(true),
        };

        assert!(value.validate().is_ok());
    }

    #[test]
    fn seedance_fast_and_mini_accept_documented_limits() {
        for model in [
            "doubao-seedance-2-0-fast-260128",
            "doubao-seedance-2-0-mini-260615",
        ] {
            let mut value = request();
            value.provider = VideoProvider::Seedance;
            value.model = model.to_string();
            value.options = VideoGenerationOptions {
                duration: 4,
                aspect_ratio: "16:9".to_string(),
                resolution: "720p".to_string(),
                generate_audio: Some(true),
            };
            assert!(value.validate().is_ok(), "{model} should accept 4s at 720p");

            value.options.resolution = "1080p".to_string();
            assert!(value.validate().unwrap_err().contains("resolution"));
        }
    }

    #[test]
    fn seedance_accepts_valid_ark_assets_only_as_references() {
        let asset = ReferenceImageInput {
            name: "portrait".to_string(),
            mime_type: "image/ark-asset".to_string(),
            data: String::new(),
            asset_id: Some("asset-20260801-example".to_string()),
        };
        let mut value = request();
        value.provider = VideoProvider::Seedance;
        value.model = "doubao-seedance-2-0-260128".to_string();
        value.input_mode = VideoInputMode::Reference;
        value.reference_images = Some(vec![asset.clone()]);
        value.options.generate_audio = Some(true);
        assert!(value.validate().is_ok());

        value.input_mode = VideoInputMode::Image;
        value.reference_images = None;
        value.starting_image = Some(asset);
        assert!(value
            .validate()
            .unwrap_err()
            .contains("only be used as Seedance reference"));

        value.input_mode = VideoInputMode::Reference;
        value.starting_image = None;
        value.reference_images = Some(vec![ReferenceImageInput {
            name: "unsafe".to_string(),
            mime_type: "image/ark-asset".to_string(),
            data: String::new(),
            asset_id: Some("asset-../../secret".to_string()),
        }]);
        assert!(value.validate().unwrap_err().contains("Invalid"));
    }

    #[test]
    fn seedance_and_veo_accept_start_and_end_frames_but_grok_rejects_them() {
        let mut value = request();
        value.provider = VideoProvider::Seedance;
        value.model = "doubao-seedance-2-0-260128".to_string();
        value.input_mode = VideoInputMode::Frames;
        value.starting_image = Some(valid_image("start.png"));
        value.ending_image = Some(valid_image("end.png"));
        value.options.generate_audio = Some(true);
        assert!(value.validate().is_ok());

        value.provider = VideoProvider::GoogleVeo;
        value.model = "veo-3.1-generate-preview".to_string();
        value.options.duration = 8;
        value.options.resolution = "720p".to_string();
        value.options.generate_audio = None;
        assert!(value.validate().is_ok());

        value.provider = VideoProvider::GrokImagine;
        value.model = "grok-imagine-video".to_string();
        value.options.duration = 5;
        value.options.resolution = "480p".to_string();
        assert!(value.validate().unwrap_err().contains("does not support"));
    }

    #[test]
    fn veo_enforces_reference_and_high_resolution_duration_rules() {
        let mut value = request();
        value.provider = VideoProvider::GoogleVeo;
        value.model = "veo-3.1-generate-preview".to_string();
        value.options = VideoGenerationOptions {
            duration: 6,
            aspect_ratio: "9:16".to_string(),
            resolution: "1080p".to_string(),
            generate_audio: None,
        };
        assert!(value
            .validate()
            .unwrap_err()
            .contains("requires an 8-second"));

        value.options.duration = 8;
        value.input_mode = VideoInputMode::Reference;
        value.reference_images = Some(
            (0..4)
                .map(|index| valid_image(&format!("reference-{index}.png")))
                .collect(),
        );
        assert!(value.validate().unwrap_err().contains("between 1 and 3"));
    }

    fn valid_image(name: &str) -> ReferenceImageInput {
        ReferenceImageInput {
            name: name.to_string(),
            mime_type: "image/png".to_string(),
            data: general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nsource"),
            asset_id: None,
        }
    }

    #[test]
    fn legacy_image_batch_defaults_to_image_media_type() {
        let batch: GenerationBatch = serde_json::from_value(json!({
            "id": "legacy-batch",
            "provider": "nano-banana",
            "model": "gemini-2.5-flash-image",
            "promptSnapshot": "prompt",
            "status": "completed",
            "images": [],
            "createdAt": "2026-05-01T00:00:00Z",
            "completedAt": "2026-05-01T00:00:01Z",
            "error": null
        }))
        .unwrap();

        assert_eq!(batch.media_type, GenerationMediaType::Image);
        assert!(batch.videos.is_empty());
        assert!(batch.provider_request_id.is_none());
    }
}
