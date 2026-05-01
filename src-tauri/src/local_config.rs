use crate::models::AppSettings;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LocalConfig {
    #[serde(default)]
    app: AppConfig,
    #[serde(default)]
    gemini: GeminiConfig,
    #[serde(default)]
    openai: OpenAiConfig,
    #[serde(default)]
    xai: XaiConfig,
    #[serde(default)]
    output: OutputConfig,
    #[serde(default)]
    prompts: PromptsConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AppConfig {
    #[serde(default)]
    default_provider: Option<String>,
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
struct OpenAiConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct XaiConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OutputConfig {
    #[serde(default)]
    directory: Option<String>,
    #[serde(default)]
    template: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PromptsConfig {
    #[serde(default)]
    directory: Option<String>,
    #[serde(default)]
    dsl_enabled: Option<bool>,
}

pub fn load_settings(defaults: AppSettings) -> AppSettings {
    let Ok(config) = load_config() else {
        return defaults;
    };

    let default_provider = config
        .app
        .default_provider
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(defaults.default_provider);
    let default_model = match default_provider.as_str() {
        "gpt-image" => config
            .openai
            .default_model
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.default_model),
        "grok-imagine" => config
            .xai
            .default_model
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.default_model),
        _ => config
            .gemini
            .default_model
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.default_model),
    };

    AppSettings {
        default_provider,
        default_model,
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
        prompt_directory: config
            .prompts
            .directory
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(defaults.prompt_directory),
        prompt_dsl_enabled: config
            .prompts
            .dsl_enabled
            .unwrap_or(defaults.prompt_dsl_enabled),
        optional_base_url: config
            .gemini
            .base_url
            .filter(|value| !value.trim().is_empty())
            .or(defaults.optional_base_url),
        openai_base_url: config
            .openai
            .base_url
            .filter(|value| !value.trim().is_empty())
            .or(defaults.openai_base_url),
        xai_base_url: config
            .xai
            .base_url
            .filter(|value| !value.trim().is_empty())
            .or(defaults.xai_base_url),
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
    config.app.default_provider = Some(settings.default_provider.clone());
    match settings.default_provider.as_str() {
        "gpt-image" => config.openai.default_model = Some(settings.default_model.clone()),
        "grok-imagine" => config.xai.default_model = Some(settings.default_model.clone()),
        _ => config.gemini.default_model = Some(settings.default_model.clone()),
    }
    config.gemini.base_url = normalize_optional(settings.optional_base_url.clone());
    config.openai.base_url = normalize_optional(settings.openai_base_url.clone());
    config.xai.base_url = normalize_optional(settings.xai_base_url.clone());
    config.gemini.proxy_url = normalize_optional(settings.proxy_url.clone());
    config.gemini.timeout_seconds = Some(settings.timeout_seconds);
    config.output.directory = Some(settings.output_directory.clone());
    config.output.template = Some(settings.output_template.clone());
    config.prompts.directory = Some(settings.prompt_directory.clone());
    config.prompts.dsl_enabled = Some(settings.prompt_dsl_enabled);
    save_config(&config)
}

pub fn save_output_template(template: &str) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.output.template = Some(template.trim().to_string());
    save_config(&config)
}

pub fn set_gemini_api_key(api_key: &str) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.gemini.api_key = api_key.trim().to_string();
    save_config(&config)
}

pub fn set_openai_api_key(api_key: &str) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.openai.api_key = api_key.trim().to_string();
    save_config(&config)
}

pub fn set_xai_api_key(api_key: &str) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.xai.api_key = api_key.trim().to_string();
    save_config(&config)
}

pub fn get_gemini_api_key() -> io::Result<String> {
    let config = load_config()?;
    let key = config.gemini.api_key.trim().to_string();
    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Gemini API key is missing in ~/.sozocraft/config.toml",
        ));
    }
    Ok(key)
}

pub fn get_openai_api_key() -> io::Result<String> {
    let config = load_config()?;
    let key = config.openai.api_key.trim().to_string();
    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "OpenAI API key is missing in ~/.sozocraft/config.toml",
        ));
    }
    Ok(key)
}

pub fn get_xai_api_key() -> io::Result<String> {
    let config = load_config()?;
    let key = config.xai.api_key.trim().to_string();
    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "xAI API key is missing in ~/.sozocraft/config.toml",
        ));
    }
    Ok(key)
}

pub fn has_gemini_api_key() -> bool {
    get_gemini_api_key()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn has_openai_api_key() -> bool {
    get_openai_api_key()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn has_xai_api_key() -> bool {
    get_xai_api_key()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn has_proxy_configured() -> bool {
    load_config()
        .ok()
        .and_then(|config| config.gemini.proxy_url)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".sozocraft")
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
