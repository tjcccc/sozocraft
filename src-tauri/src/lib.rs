mod app_state;
mod error_log;
mod filename_template;
mod gemini;
mod gemini_models;
mod image_meta;
mod local_config;
mod models;
mod openai_image;
mod prompt_library;
mod reference_image_cache;
mod xai_image;

use app_state::{load_state, save_state};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Local, Utc};
use error_log::GenerationErrorLog;
use filename_template::resolve_output_path;
use gemini::GeminiClient;
use models::{
    AppSettings, AppState, GenerationBatch, GenerationRequest, GenerationStatus, OutputImage,
};
use openai_image::OpenAiImageClient;
use prompt_library::{
    CreatePromptRequest, PromptDocument, PromptListItem, RenderPromptResult, SavePromptRequest,
    UpdatePromptMetadataRequest,
};
use serde::Serialize;
use std::{collections::HashMap, fs, path::PathBuf, sync::Mutex};
use tauri::State as TauriState;
use tokio::sync::watch;
use uuid::Uuid;
use xai_image::XaiImageClient;

#[derive(Default)]
struct GenerationRuntime {
    cancellations: Mutex<HashMap<String, watch::Sender<bool>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigStatus {
    config_path: String,
    has_api_key: bool,
    has_openai_api_key: bool,
    has_xai_api_key: bool,
    has_proxy: bool,
}

#[tauri::command]
fn get_config_status() -> ConfigStatus {
    ConfigStatus {
        config_path: local_config::config_path().to_string_lossy().to_string(),
        has_api_key: local_config::has_gemini_api_key(),
        has_openai_api_key: local_config::has_openai_api_key(),
        has_xai_api_key: local_config::has_xai_api_key(),
        has_proxy: local_config::has_proxy_configured(),
    }
}

#[tauri::command]
fn save_output_template(template: String) -> Result<(), String> {
    local_config::save_output_template(&template).map_err(|err| err.to_string())
}

#[tauri::command]
fn cancel_generation_task(
    runtime: TauriState<'_, GenerationRuntime>,
    task_id: String,
) -> Result<bool, String> {
    let cancellations = runtime
        .cancellations
        .lock()
        .map_err(|_| "Generation cancellation state is unavailable.".to_string())?;
    let Some(sender) = cancellations.get(&task_id) else {
        return Ok(false);
    };
    sender
        .send(true)
        .map_err(|_| "Generation task is no longer active.".to_string())?;
    Ok(true)
}

#[tauri::command]
fn load_app_state() -> Result<AppState, String> {
    let state = load_state().map_err(|err| err.to_string())?;
    let _ = prompt_library::rescan_prompt_directory(&state.settings.prompt_directory);
    Ok(state)
}

#[tauri::command]
fn save_app_settings(settings: AppSettings) -> Result<AppState, String> {
    let mut state = load_state().map_err(|err| err.to_string())?;
    local_config::save_settings(&settings).map_err(|err| err.to_string())?;
    state.settings = settings;
    save_state(&state).map_err(|err| err.to_string())?;
    Ok(state)
}

#[tauri::command]
fn save_current_prompt(prompt: String) -> Result<AppState, String> {
    let mut state = load_state().map_err(|err| err.to_string())?;
    state.current_prompt = prompt;
    save_state(&state).map_err(|err| err.to_string())?;
    Ok(state)
}

#[tauri::command]
fn save_current_prompt_id(prompt_id: Option<String>) -> Result<AppState, String> {
    let mut state = load_state().map_err(|err| err.to_string())?;
    state.current_prompt_id = prompt_id;
    save_state(&state).map_err(|err| err.to_string())?;
    Ok(state)
}

#[tauri::command]
fn list_prompts(
    prompt_directory: String,
    query: Option<String>,
) -> Result<Vec<PromptListItem>, String> {
    prompt_library::list_prompts(&prompt_directory, query)
}

#[tauri::command]
fn rescan_prompt_library(prompt_directory: String) -> Result<Vec<PromptListItem>, String> {
    prompt_library::rescan_prompt_directory(&prompt_directory)
}

#[tauri::command]
fn create_prompt(
    prompt_directory: String,
    request: CreatePromptRequest,
) -> Result<PromptDocument, String> {
    prompt_library::create_prompt(&prompt_directory, request)
}

#[tauri::command]
fn read_prompt(prompt_directory: String, id: String) -> Result<PromptDocument, String> {
    prompt_library::read_prompt(&prompt_directory, &id)
}

#[tauri::command]
fn save_prompt(
    prompt_directory: String,
    request: SavePromptRequest,
) -> Result<PromptDocument, String> {
    prompt_library::save_prompt(&prompt_directory, request)
}

#[tauri::command]
fn update_prompt_metadata(
    prompt_directory: String,
    request: UpdatePromptMetadataRequest,
) -> Result<PromptListItem, String> {
    prompt_library::update_prompt_metadata(&prompt_directory, request)
}

#[tauri::command]
fn delete_prompt(prompt_directory: String, id: String) -> Result<(), String> {
    prompt_library::delete_prompt(&prompt_directory, &id)
}

#[tauri::command]
fn render_prompt_source(
    source: String,
    prompt_directory: Option<String>,
    current_prompt_id: Option<String>,
) -> RenderPromptResult {
    if let Some(prompt_directory) = prompt_directory {
        return prompt_library::render_prompt_source_with_library(
            &source,
            &prompt_directory,
            current_prompt_id.as_deref(),
        );
    }
    prompt_library::render_prompt_source(&source)
}

#[tauri::command]
fn set_gemini_api_key(api_key: String) -> Result<bool, String> {
    local_config::set_gemini_api_key(&api_key).map_err(|err| err.to_string())?;
    Ok(true)
}

#[tauri::command]
fn set_openai_api_key(api_key: String) -> Result<bool, String> {
    local_config::set_openai_api_key(&api_key).map_err(|err| err.to_string())?;
    Ok(true)
}

#[tauri::command]
fn set_xai_api_key(api_key: String) -> Result<bool, String> {
    local_config::set_xai_api_key(&api_key).map_err(|err| err.to_string())?;
    Ok(true)
}

#[tauri::command]
fn has_gemini_api_key() -> bool {
    local_config::has_gemini_api_key()
}

#[tauri::command]
fn has_openai_api_key() -> bool {
    local_config::has_openai_api_key()
}

#[tauri::command]
fn has_xai_api_key() -> bool {
    local_config::has_xai_api_key()
}

#[tauri::command]
fn read_image_data_url(path: String) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read image: {err}"))?;
    let mime = match PathBuf::from(&path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    };
    Ok(format!(
        "data:{mime};base64,{}",
        general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
fn export_rendered_prompt(output_path: String, rendered_prompt: String) -> Result<String, String> {
    let path = PathBuf::from(output_path.trim());
    if path.as_os_str().is_empty() {
        return Err("Choose an export path before exporting.".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create prompt export directory: {err}"))?;
    }

    fs::write(&path, rendered_prompt)
        .map_err(|err| format!("Failed to export rendered prompt: {err}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn generate_images(
    runtime: TauriState<'_, GenerationRuntime>,
    mut request: GenerationRequest,
) -> Result<GenerationBatch, String> {
    let task_id = request
        .task_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    request.task_id = Some(task_id.clone());
    let (sender, receiver) = watch::channel(false);
    {
        let mut cancellations = runtime
            .cancellations
            .lock()
            .map_err(|_| "Generation cancellation state is unavailable.".to_string())?;
        cancellations.insert(task_id.clone(), sender);
    }

    let result = generate_images_inner(request, receiver).await;
    if let Ok(mut cancellations) = runtime.cancellations.lock() {
        cancellations.remove(&task_id);
    }
    result
}

async fn generate_images_inner(
    mut request: GenerationRequest,
    mut cancellation: watch::Receiver<bool>,
) -> Result<GenerationBatch, String> {
    request.validate()?;
    reference_image_cache::optimize_request_reference_images(&mut request)?;

    let rendered_prompt = request.prompt.trim().to_string();
    let prompt_snapshot = request
        .prompt_snapshot
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| request.prompt.clone());
    let mut state = load_state().map_err(|err| err.to_string())?;
    let settings = state.settings.clone();
    let batch_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let filename_datetime = Local::now();

    let mut batch = GenerationBatch {
        id: batch_id.clone(),
        provider: request.provider.clone(),
        model: request.model.clone(),
        prompt_snapshot: prompt_snapshot.clone(),
        status: GenerationStatus::Running,
        images: Vec::new(),
        created_at,
        completed_at: None,
        error: None,
    };

    let mut last_error: Option<String> = None;
    let attempts = if ["gpt-image", "grok-imagine"].contains(&request.provider.as_str()) {
        1
    } else {
        request.batch_count
    };
    let mut cancelled = false;
    for index in 0..attempts {
        if *cancellation.borrow() {
            cancelled = true;
            break;
        }
        let item_request = request.clone();

        let provider_result = tokio::select! {
            result = generate_with_provider(&item_request, &settings) => result,
            changed = cancellation.changed() => {
                if changed.is_ok() && *cancellation.borrow() {
                    cancelled = true;
                    break;
                }
                continue;
            }
        };

        match provider_result {
            Ok(response) => {
                if *cancellation.borrow() {
                    cancelled = true;
                    break;
                }
                if response.images.is_empty() {
                    let message = no_image_data_error(&request.provider, &response.metadata);
                    error_log::log_generation_error(&GenerationErrorLog {
                        timestamp: Utc::now(),
                        batch_id: &batch_id,
                        provider: &request.provider,
                        model: &request.model,
                        attempt: index + 1,
                        kind: "no_image_data",
                        message: &message,
                        base_url: request
                            .base_url
                            .as_deref()
                            .or(effective_base_url(&request.provider, &settings)),
                        proxy_url: provider_proxy_ref(&request.provider, &settings),
                        timeout_seconds: provider_timeout(&request.provider, &settings),
                        reference_image_count: error_log::reference_image_count(&item_request),
                        response_metadata: Some(error_log::no_image_metadata(&response.metadata)),
                    });
                    last_error = Some(message);
                    continue;
                }

                for image in response.images {
                    if *cancellation.borrow() {
                        cancelled = true;
                        break;
                    }
                    let image_id = Uuid::new_v4().to_string();
                    let file_id = format!("{:03}", batch.images.len() + 1);

                    let image_bytes = if image_meta::is_png(&image.bytes) {
                        image.bytes
                    } else {
                        match image_meta::to_png(&image.bytes) {
                            Ok(png) => png,
                            Err(err) => {
                                last_error = Some(format!(
                                    "Failed to convert generated image to PNG: {err}"
                                ));
                                continue;
                            }
                        }
                    };

                    let path = resolve_output_path(
                        &settings.output_directory,
                        &request.output_template,
                        filename_provider(&request.provider),
                        filename_model(&request.model),
                        &file_id,
                        &short_id(&batch_id),
                        "png",
                        filename_datetime,
                    )
                    .map_err(|err| err.to_string())?;

                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|err| format!("Failed to create output directory: {err}"))?;
                    }

                    let filename = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or_default()
                        .to_string();
                    let image_created_at = Utc::now();

                    let vc_meta = serde_json::json!({
                        "schemaVersion": 1,
                        "promptSnapshot": prompt_snapshot.clone(),
                        "renderedPrompt": rendered_prompt.clone(),
                        "provider": request.provider.clone(),
                        "model": request.model.clone(),
                        "options": {
                            "aspectRatio": request.options.aspect_ratio.clone(),
                            "imageSize": request.options.image_size.clone(),
                            "temperature": request.options.temperature,
                            "topP": request.options.top_p,
                            "thinkingLevel": request.options.thinking_level.clone(),
                            "quality": request.options.quality.clone(),
                        },
                        "batchId": batch_id.clone(),
                        "imageId": image_id.clone(),
                        "createdAt": image_created_at,
                        "batchCreatedAt": created_at,
                        "responseMetadata": response.metadata.clone(),
                    });
                    let vc_str = serde_json::to_string(&vc_meta).unwrap_or_default();
                    let with_prompt =
                        image_meta::embed_png_itxt(&image_bytes, "prompt", &rendered_prompt);
                    let write_bytes =
                        image_meta::embed_png_itxt(&with_prompt, "sozocraft", &vc_str);

                    fs::write(&path, &write_bytes)
                        .map_err(|err| format!("Failed to save generated image: {err}"))?;

                    batch.images.push(OutputImage {
                        id: image_id,
                        batch_id: batch_id.clone(),
                        provider: request.provider.clone(),
                        model: request.model.clone(),
                        path: path.to_string_lossy().to_string(),
                        filename,
                        created_at: image_created_at,
                        prompt_snapshot: prompt_snapshot.clone(),
                        metadata: Some(response.metadata.clone()),
                    });
                }
                if cancelled {
                    break;
                }
            }
            Err(err) => {
                let proxy_note = provider_proxy_ref(&request.provider, &settings)
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("Proxy configured: {value}"))
                    .unwrap_or_else(|| {
                        "No proxy active for this provider. Save Proxy URL in settings and enable provider proxy if needed.".to_string()
                    });
                let detailed_error = err.clone();
                let message = format!("{detailed_error}. {proxy_note}");
                error_log::log_generation_error(&GenerationErrorLog {
                    timestamp: Utc::now(),
                    batch_id: &batch_id,
                    provider: &request.provider,
                    model: &request.model,
                    attempt: index + 1,
                    kind: "request_failed",
                    message: &message,
                    base_url: request
                        .base_url
                        .as_deref()
                        .or(effective_base_url(&request.provider, &settings)),
                    proxy_url: provider_proxy_ref(&request.provider, &settings),
                    timeout_seconds: provider_timeout(&request.provider, &settings),
                    reference_image_count: error_log::reference_image_count(&item_request),
                    response_metadata: None,
                });
                last_error = Some(format!("{err}. {proxy_note}"));
            }
        }
    }

    if cancelled {
        batch.status = GenerationStatus::Cancelled;
        batch.error = Some("Generation cancelled.".to_string());
        batch.completed_at = Some(Utc::now());
    } else if batch.images.is_empty() {
        batch.status = GenerationStatus::Failed;
        batch.error = Some(last_error.unwrap_or_else(|| "Generation failed.".to_string()));
    } else {
        batch.status = GenerationStatus::Completed;
        batch.completed_at = Some(Utc::now());
    }

    state.current_prompt = prompt_snapshot;
    state.batches.insert(0, batch.clone());
    state.batches.truncate(50);
    save_state(&state).map_err(|err| err.to_string())?;

    if batch.status == GenerationStatus::Failed {
        Err(batch
            .error
            .clone()
            .unwrap_or_else(|| "Generation did not complete.".to_string()))
    } else {
        Ok(batch)
    }
}

fn filename_provider(provider: &str) -> &str {
    match provider {
        "nano-banana" => "gemini",
        "gpt-image" => "openai",
        "grok-imagine" => "xai",
        value => value,
    }
}

fn filename_model(model: &str) -> &str {
    match model {
        "gemini-3-pro-image-preview" => "nano-banana-pro",
        "gemini-3.1-flash-image-preview" => "nano-banana-2",
        "gemini-2.5-flash-image" => "nano-banana",
        "gpt-image-2" => "gpt-image-2",
        "grok-imagine-image" => "grok-imagine",
        value => value,
    }
}

fn effective_base_url<'a>(provider: &str, settings: &'a AppSettings) -> Option<&'a str> {
    match provider {
        "gpt-image" => settings.openai_base_url.as_deref(),
        "grok-imagine" => settings.xai_base_url.as_deref(),
        _ => settings.optional_base_url.as_deref(),
    }
}

fn provider_timeout(provider: &str, settings: &AppSettings) -> u64 {
    match provider {
        "gpt-image" => settings.openai_timeout_seconds,
        "grok-imagine" => settings.xai_timeout_seconds,
        _ => settings.gemini_timeout_seconds,
    }
}

fn provider_proxy(provider: &str, settings: &AppSettings) -> Option<String> {
    let enabled = match provider {
        "gpt-image" => settings.openai_proxy_enabled,
        "grok-imagine" => settings.xai_proxy_enabled,
        _ => settings.gemini_proxy_enabled,
    };
    enabled.then(|| settings.proxy_url.clone()).flatten()
}

fn provider_proxy_ref<'a>(provider: &str, settings: &'a AppSettings) -> Option<&'a str> {
    let enabled = match provider {
        "gpt-image" => settings.openai_proxy_enabled,
        "grok-imagine" => settings.xai_proxy_enabled,
        _ => settings.gemini_proxy_enabled,
    };
    enabled.then_some(settings.proxy_url.as_deref()).flatten()
}

#[derive(Debug, Clone)]
struct ProviderGeneratedImage {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ProviderResponse {
    images: Vec<ProviderGeneratedImage>,
    metadata: serde_json::Value,
}

async fn generate_with_provider(
    request: &GenerationRequest,
    settings: &AppSettings,
) -> Result<ProviderResponse, String> {
    match request.provider.as_str() {
        "nano-banana" => {
            let client = GeminiClient::new(
                local_config::get_gemini_api_key().map_err(|err| err.to_string())?,
                request
                    .base_url
                    .clone()
                    .or_else(|| settings.optional_base_url.clone()),
                provider_proxy("nano-banana", settings),
                provider_timeout("nano-banana", settings),
            )
            .map_err(|err| err.to_string())?;
            let response = client
                .generate(request)
                .await
                .map_err(|err| err.to_string())?;
            Ok(ProviderResponse {
                images: response
                    .images
                    .into_iter()
                    .map(|image| ProviderGeneratedImage { bytes: image.bytes })
                    .collect(),
                metadata: response.metadata,
            })
        }
        "gpt-image" => {
            let client = OpenAiImageClient::new(
                local_config::get_openai_api_key().map_err(|err| err.to_string())?,
                request
                    .base_url
                    .clone()
                    .or_else(|| settings.openai_base_url.clone()),
                provider_proxy("gpt-image", settings),
                provider_timeout("gpt-image", settings),
            )
            .map_err(|err| err.to_string())?;
            let response = client
                .generate(request)
                .await
                .map_err(|err| err.to_string())?;
            Ok(ProviderResponse {
                images: response
                    .images
                    .into_iter()
                    .map(|image| ProviderGeneratedImage { bytes: image.bytes })
                    .collect(),
                metadata: response.metadata,
            })
        }
        "grok-imagine" => {
            let client = XaiImageClient::new(
                local_config::get_xai_api_key().map_err(|err| err.to_string())?,
                request
                    .base_url
                    .clone()
                    .or_else(|| settings.xai_base_url.clone()),
                provider_proxy("grok-imagine", settings),
                provider_timeout("grok-imagine", settings),
            )
            .map_err(|err| err.to_string())?;
            let response = client
                .generate(request)
                .await
                .map_err(|err| err.to_string())?;
            Ok(ProviderResponse {
                images: response
                    .images
                    .into_iter()
                    .map(|image| ProviderGeneratedImage { bytes: image.bytes })
                    .collect(),
                metadata: response.metadata,
            })
        }
        value => Err(format!("Unsupported image provider: {value}")),
    }
}

fn no_image_data_error(provider: &str, metadata: &serde_json::Value) -> String {
    match provider {
        "nano-banana" => gemini::no_image_data_error(metadata),
        "gpt-image" => "OpenAI returned no image data.".to_string(),
        "grok-imagine" => "xAI returned no image data.".to_string(),
        _ => "Provider returned no image data.".to_string(),
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(6).collect()
}

pub fn run() {
    tauri::Builder::default()
        .manage(GenerationRuntime::default())
        .invoke_handler(tauri::generate_handler![
            load_app_state,
            save_app_settings,
            save_current_prompt,
            save_current_prompt_id,
            list_prompts,
            rescan_prompt_library,
            create_prompt,
            read_prompt,
            save_prompt,
            update_prompt_metadata,
            delete_prompt,
            render_prompt_source,
            export_rendered_prompt,
            save_output_template,
            set_gemini_api_key,
            set_openai_api_key,
            set_xai_api_key,
            has_gemini_api_key,
            has_openai_api_key,
            has_xai_api_key,
            read_image_data_url,
            cancel_generation_task,
            generate_images,
            get_config_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running SozoCraft");
}
