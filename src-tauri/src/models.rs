use crate::gemini_models::max_reference_images;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub settings: AppSettings,
    pub current_prompt: String,
    pub batches: Vec<GenerationBatch>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: AppSettings::default(),
            current_prompt: "Create a cinematic portrait with realistic lighting.".to_string(),
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
    #[serde(default)]
    pub optional_base_url: Option<String>,
    #[serde(default)]
    pub openai_base_url: Option<String>,
    #[serde(default)]
    pub xai_base_url: Option<String>,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
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

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_provider: "nano-banana".to_string(),
            default_model: "gemini-3-pro-image-preview".to_string(),
            output_directory: default_output_directory(),
            output_template: "{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}"
                .to_string(),
            optional_base_url: None,
            openai_base_url: None,
            xai_base_url: None,
            proxy_url: None,
            timeout_seconds: 180,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
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
}

pub const SUPPORTED_MODELS: [&str; 3] = [
    "gemini-3-pro-image-preview",
    "gemini-3.1-flash-image-preview",
    "gemini-2.5-flash-image",
];

pub const OPENAI_IMAGE_MODELS: [&str; 1] = ["gpt-image-2"];

pub const XAI_IMAGE_MODELS: [&str; 1] = ["grok-imagine-image"];

fn supported_models(provider: &str) -> &'static [&'static str] {
    match provider {
        "nano-banana" => &SUPPORTED_MODELS,
        "gpt-image" => &OPENAI_IMAGE_MODELS,
        "grok-imagine" => &XAI_IMAGE_MODELS,
        _ => &[],
    }
}

fn max_reference_images_for_provider(provider: &str, model: &str) -> usize {
    match provider {
        "gpt-image" => 16,
        "grok-imagine" => 5,
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
pub struct GenerationBatch {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub prompt_snapshot: String,
    pub status: GenerationStatus,
    pub images: Vec<OutputImage>,
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
}
