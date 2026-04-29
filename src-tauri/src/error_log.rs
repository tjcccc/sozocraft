use crate::{local_config, models::GenerationRequest};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationErrorLog<'a> {
    pub timestamp: DateTime<Utc>,
    pub batch_id: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub attempt: u32,
    pub kind: &'a str,
    pub message: &'a str,
    pub base_url: Option<&'a str>,
    pub proxy_url: Option<&'a str>,
    pub timeout_seconds: u64,
    pub reference_image_count: usize,
    pub response_metadata: Option<Value>,
}

pub fn error_log_path() -> PathBuf {
    local_config::config_dir().join("error.log")
}

pub fn append_generation_error(event: &GenerationErrorLog<'_>) -> io::Result<()> {
    let path = error_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(event)?;
    writeln!(file, "{line}")
}

pub fn log_generation_error(event: &GenerationErrorLog<'_>) {
    if let Err(err) = append_generation_error(event) {
        eprintln!("Failed to write SozoCraft error log: {err}");
    }
}

pub fn reference_image_count(request: &GenerationRequest) -> usize {
    request
        .reference_images
        .as_ref()
        .map(Vec::len)
        .unwrap_or_default()
}

pub fn no_image_metadata(metadata: &Value) -> Value {
    json!({
        "usageMetadata": metadata.get("usageMetadata").cloned().unwrap_or(Value::Null),
        "modelVersion": metadata.get("modelVersion").cloned().unwrap_or(Value::Null),
        "promptFeedback": metadata.get("promptFeedback").cloned().unwrap_or(Value::Null),
        "candidates": metadata.get("candidates").cloned().unwrap_or(Value::Null),
    })
}
