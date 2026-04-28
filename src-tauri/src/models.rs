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
    pub default_model: String,
    pub output_directory: String,
    pub output_template: String,
    pub optional_base_url: Option<String>,
    pub proxy_url: Option<String>,
    pub timeout_seconds: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        let output_directory = dirs::picture_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("SozoCraft")
            .to_string_lossy()
            .to_string();

        Self {
            default_model: "gemini-3-pro-image-preview".to_string(),
            output_directory,
            output_template: "{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}"
                .to_string(),
            optional_base_url: None,
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
    pub batch_count: u32,
    pub reference_images: Option<Vec<String>>,
    pub output_template: String,
    pub options: GenerationOptions,
    pub base_url: Option<String>,
}

impl GenerationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.provider != "nano-banana" {
            return Err("Only the nano-banana provider is implemented in v0.1.0.".to_string());
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
        if !SUPPORTED_MODELS.contains(&self.model.as_str()) {
            return Err(format!("Unsupported Gemini image model: {}", self.model));
        }
        if self.output_template.trim().is_empty() {
            return Err("Output filename template cannot be empty.".to_string());
        }
        Ok(())
    }
}

pub const SUPPORTED_MODELS: [&str; 3] = [
    "gemini-3-pro-image-preview",
    "gemini-3.1-flash-image-preview",
    "gemini-2.5-flash-image",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationOptions {
    pub aspect_ratio: Option<String>,
    pub image_size: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub seed: Option<i64>,
    pub thinking_level: Option<String>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            aspect_ratio: Some("16:9".to_string()),
            image_size: Some("1K".to_string()),
            temperature: Some(0.5),
            top_p: Some(0.95),
            seed: None,
            thinking_level: Some("MINIMAL".to_string()),
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
