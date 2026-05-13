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
    openrouter: OpenRouterConfig,
    #[serde(default)]
    xai: XaiConfig,
    #[serde(default)]
    higgsfield: HiggsfieldConfig,
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
    api_platform: Option<String>,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    proxy_url: Option<String>,
    #[serde(default)]
    proxy_enabled: Option<bool>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OpenAiConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    api_platform: Option<String>,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    proxy_enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OpenRouterConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct XaiConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    api_platform: Option<String>,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    proxy_enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct HiggsfieldConfig {
    #[serde(default)]
    cli_path: Option<String>,
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
    #[serde(default)]
    editor_only: Option<bool>,
    #[serde(default)]
    preview_placement: Option<String>,
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
    let nano_banana_api_platform = normalize_nano_banana_api_platform(
        config.gemini.api_platform.clone(),
        defaults.nano_banana_api_platform.clone(),
    );
    let openai_api_platform = normalize_openai_api_platform(
        config.openai.api_platform.clone(),
        defaults.openai_api_platform.clone(),
    );
    let grok_api_platform = normalize_grok_api_platform(
        config.xai.api_platform.clone(),
        defaults.grok_api_platform.clone(),
    );
    let default_model = match default_provider.as_str() {
        "gpt-image" => config
            .openai
            .default_model
            .filter(|value| !value.trim().is_empty())
            .map(|value| normalize_gpt_image_model(&openai_api_platform, value))
            .unwrap_or_else(|| default_gpt_image_model(&openai_api_platform)),
        "grok-imagine" => config
            .xai
            .default_model
            .filter(|value| !value.trim().is_empty())
            .map(|value| normalize_grok_model(&grok_api_platform, value))
            .unwrap_or_else(|| default_grok_model(&grok_api_platform)),
        _ => config
            .gemini
            .default_model
            .filter(|value| !value.trim().is_empty())
            .map(|value| normalize_nano_banana_model(&nano_banana_api_platform, value))
            .unwrap_or_else(|| default_nano_banana_model(&nano_banana_api_platform)),
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
        prompt_editor_only: config
            .prompts
            .editor_only
            .unwrap_or(defaults.prompt_editor_only),
        prompt_preview_placement: normalize_preview_placement(
            config.prompts.preview_placement,
            defaults.prompt_preview_placement,
        ),
        nano_banana_api_platform,
        gemini_proxy_enabled: config
            .gemini
            .proxy_enabled
            .unwrap_or(defaults.gemini_proxy_enabled),
        openai_api_platform,
        openai_proxy_enabled: config
            .openai
            .proxy_enabled
            .unwrap_or(defaults.openai_proxy_enabled),
        grok_api_platform,
        xai_proxy_enabled: config
            .xai
            .proxy_enabled
            .unwrap_or(defaults.xai_proxy_enabled),
        higgsfield_cli_path: config
            .higgsfield
            .cli_path
            .filter(|value| !value.trim().is_empty())
            .or(defaults.higgsfield_cli_path),
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
        openrouter_base_url: config
            .openrouter
            .base_url
            .filter(|value| !value.trim().is_empty())
            .or(defaults.openrouter_base_url),
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
        gemini_timeout_seconds: config
            .gemini
            .timeout_seconds
            .unwrap_or(defaults.gemini_timeout_seconds),
        openai_timeout_seconds: config
            .openai
            .timeout_seconds
            .unwrap_or(defaults.openai_timeout_seconds),
        xai_timeout_seconds: config
            .xai
            .timeout_seconds
            .unwrap_or(defaults.xai_timeout_seconds),
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
    config.gemini.api_platform = Some(normalize_nano_banana_api_platform(
        Some(settings.nano_banana_api_platform.clone()),
        "gemini".to_string(),
    ));
    config.gemini.base_url = normalize_optional(settings.optional_base_url.clone());
    config.openai.api_platform = Some(normalize_openai_api_platform(
        Some(settings.openai_api_platform.clone()),
        "openai".to_string(),
    ));
    config.openai.base_url = normalize_optional(settings.openai_base_url.clone());
    config.openrouter.base_url = normalize_optional(settings.openrouter_base_url.clone());
    config.xai.api_platform = Some(normalize_grok_api_platform(
        Some(settings.grok_api_platform.clone()),
        "xai".to_string(),
    ));
    config.xai.base_url = normalize_optional(settings.xai_base_url.clone());
    config.higgsfield.cli_path = normalize_optional(settings.higgsfield_cli_path.clone());
    config.gemini.proxy_url = normalize_optional(settings.proxy_url.clone());
    config.gemini.proxy_enabled = Some(settings.gemini_proxy_enabled);
    config.openai.proxy_enabled = Some(settings.openai_proxy_enabled);
    config.xai.proxy_enabled = Some(settings.xai_proxy_enabled);
    config.gemini.timeout_seconds = Some(settings.gemini_timeout_seconds);
    config.openai.timeout_seconds = Some(settings.openai_timeout_seconds);
    config.xai.timeout_seconds = Some(settings.xai_timeout_seconds);
    config.output.directory = Some(settings.output_directory.clone());
    config.output.template = Some(settings.output_template.clone());
    config.prompts.directory = Some(settings.prompt_directory.clone());
    config.prompts.dsl_enabled = Some(settings.prompt_dsl_enabled);
    config.prompts.editor_only = Some(settings.prompt_editor_only);
    config.prompts.preview_placement = Some(normalize_preview_placement(
        Some(settings.prompt_preview_placement.clone()),
        "bottom".to_string(),
    ));
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

pub fn set_openrouter_api_key(api_key: &str) -> io::Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.openrouter.api_key = api_key.trim().to_string();
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

pub fn get_openrouter_api_key() -> io::Result<String> {
    let config = load_config()?;
    let key = config.openrouter.api_key.trim().to_string();
    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "OpenRouter API key is missing in ~/.sozocraft/config.toml",
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

pub fn has_openrouter_api_key() -> bool {
    get_openrouter_api_key()
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

fn normalize_preview_placement(value: Option<String>, fallback: String) -> String {
    match value.as_deref().map(str::trim) {
        Some("bottom" | "right" | "hidden") => value.unwrap().trim().to_string(),
        _ => fallback,
    }
}

fn normalize_openai_api_platform(value: Option<String>, fallback: String) -> String {
    match value.as_deref().map(str::trim) {
        Some("higgsfield") => "higgsfield".to_string(),
        Some("openrouter") => "openrouter".to_string(),
        Some("openai") => "openai".to_string(),
        _ => fallback,
    }
}

fn normalize_nano_banana_api_platform(value: Option<String>, fallback: String) -> String {
    match value.as_deref().map(str::trim) {
        Some("higgsfield") => "higgsfield".to_string(),
        Some("gemini") => "gemini".to_string(),
        _ => fallback,
    }
}

fn normalize_grok_api_platform(value: Option<String>, fallback: String) -> String {
    match value.as_deref().map(str::trim) {
        Some("higgsfield") => "higgsfield".to_string(),
        Some("xai") => "xai".to_string(),
        _ => fallback,
    }
}

fn default_nano_banana_model(platform: &str) -> String {
    match platform {
        "higgsfield" => "nano_banana_2".to_string(),
        _ => "gemini-3-pro-image-preview".to_string(),
    }
}

fn default_gpt_image_model(platform: &str) -> String {
    match platform {
        "higgsfield" => "gpt_image_2".to_string(),
        "openrouter" => "openai/gpt-5.4-image-2".to_string(),
        _ => "gpt-image-2".to_string(),
    }
}

fn default_grok_model(platform: &str) -> String {
    match platform {
        "higgsfield" => "grok_image".to_string(),
        _ => "grok-imagine-image-quality".to_string(),
    }
}

fn normalize_nano_banana_model(platform: &str, model: String) -> String {
    match (platform, model.as_str()) {
        ("higgsfield", "nano_banana_2" | "nano_banana_flash" | "nano_banana") => model,
        (
            "gemini",
            "gemini-3-pro-image-preview"
            | "gemini-3.1-flash-image-preview"
            | "gemini-2.5-flash-image",
        ) => model,
        _ => default_nano_banana_model(platform),
    }
}

fn normalize_gpt_image_model(platform: &str, model: String) -> String {
    match (platform, model.as_str()) {
        ("higgsfield", "gpt_image_2") => model,
        ("openrouter", "openai/gpt-5.4-image-2") => model,
        ("openai", "gpt-image-2") => model,
        _ => default_gpt_image_model(platform),
    }
}

fn normalize_grok_model(platform: &str, model: String) -> String {
    match (platform, model.as_str()) {
        ("higgsfield", "grok_image") => model,
        ("xai", "grok-imagine-image-quality" | "grok-imagine-image") => model,
        _ => default_grok_model(platform),
    }
}
