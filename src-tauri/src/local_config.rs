use crate::models::AppSettings;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LocalConfig {
    #[serde(default)]
    gemini: GeminiConfig,
    #[serde(default)]
    output: OutputConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GeminiConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    proxy_url: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OutputConfig {
    #[serde(default)]
    directory: Option<String>,
    #[serde(default)]
    template: Option<String>,
}

pub fn load_settings(defaults: AppSettings) -> AppSettings {
    let Ok(config) = load_config() else {
        return defaults;
    };

    AppSettings {
        default_model: config
            .gemini
            .default_model
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.default_model),
        output_directory: config
            .output
            .directory
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.output_directory),
        output_template: config
            .output
            .template
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.output_template),
        optional_base_url: config
            .gemini
            .base_url
            .filter(|value| !value.trim().is_empty())
            .or(defaults.optional_base_url),
        proxy_url: config
            .gemini
            .proxy_url
            .filter(|value| !value.trim().is_empty())
            .or(defaults.proxy_url),
        timeout_seconds: config
            .gemini
            .timeout_seconds
            .unwrap_or(defaults.timeout_seconds),
    }
}

pub fn save_settings(settings: &AppSettings) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.gemini.default_model = Some(settings.default_model.clone());
    config.gemini.base_url = normalize_optional(settings.optional_base_url.clone());
    config.gemini.proxy_url = normalize_optional(settings.proxy_url.clone());
    config.gemini.timeout_seconds = Some(settings.timeout_seconds);
    config.output.directory = Some(settings.output_directory.clone());
    config.output.template = Some(settings.output_template.clone());
    save_config(&config)
}

pub fn set_gemini_api_key(api_key: &str) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.gemini.api_key = api_key.trim().to_string();
    save_config(&config)
}

pub fn get_gemini_api_key() -> io::Result<String> {
    let config = load_config()?;
    let key = config.gemini.api_key.trim().to_string();
    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Gemini API key is missing in ~/.visioncraft/config.toml",
        ));
    }
    Ok(key)
}

pub fn has_gemini_api_key() -> bool {
    get_gemini_api_key()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".visioncraft")
        .join("config.toml")
}

fn load_config() -> io::Result<LocalConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(LocalConfig::default());
    }
    let raw = fs::read_to_string(path)?;
    toml::from_str(&raw).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn save_config(config: &LocalConfig) -> io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(config)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, raw)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}
