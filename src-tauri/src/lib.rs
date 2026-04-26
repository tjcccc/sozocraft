mod app_state;
mod filename_template;
mod gemini;
mod local_config;
mod models;

use app_state::{load_state, save_state};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Local, Utc};
use filename_template::resolve_output_path;
use gemini::GeminiClient;
use models::{
    AppSettings, AppState, GenerationBatch, GenerationRequest, GenerationStatus, OutputImage,
};
use std::{fs, path::PathBuf};
use uuid::Uuid;

#[tauri::command]
fn load_app_state() -> Result<AppState, String> {
    load_state().map_err(|err| err.to_string())
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
fn set_gemini_api_key(api_key: String) -> Result<bool, String> {
    local_config::set_gemini_api_key(&api_key).map_err(|err| err.to_string())?;
    Ok(true)
}

#[tauri::command]
fn has_gemini_api_key() -> bool {
    local_config::has_gemini_api_key()
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
async fn generate_images(request: GenerationRequest) -> Result<GenerationBatch, String> {
    request.validate()?;

    let api_key = local_config::get_gemini_api_key().map_err(|err| err.to_string())?;
    let mut state = load_state().map_err(|err| err.to_string())?;
    let settings = state.settings.clone();
    let batch_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let filename_datetime = Local::now();

    let mut batch = GenerationBatch {
        id: batch_id.clone(),
        provider: request.provider.clone(),
        model: request.model.clone(),
        prompt_snapshot: request.prompt.clone(),
        status: GenerationStatus::Running,
        images: Vec::new(),
        created_at,
        completed_at: None,
        error: None,
    };

    let client = GeminiClient::new(
        api_key,
        request
            .base_url
            .clone()
            .or_else(|| settings.optional_base_url.clone()),
        settings.proxy_url.clone(),
        settings.timeout_seconds,
    )
    .map_err(|err| err.to_string())?;

    let mut last_error: Option<String> = None;
    for index in 0..request.batch_count {
        let mut item_request = request.clone();
        if let Some(seed) = request.options.seed {
            item_request.options.seed = Some(seed + index as i64);
        }

        match client.generate(&item_request).await {
            Ok(response) => {
                if response.images.is_empty() {
                    last_error = Some("Gemini returned no image data.".to_string());
                    continue;
                }

                for image in response.images {
                    let image_id = Uuid::new_v4().to_string();
                    let file_id = format!("{:03}", batch.images.len() + 1);
                    let path = resolve_output_path(
                        &settings.output_directory,
                        &request.output_template,
                        filename_provider(&request.provider),
                        filename_model(&request.model),
                        &file_id,
                        &short_id(&batch_id),
                        image.extension.as_str(),
                        filename_datetime,
                    )
                    .map_err(|err| err.to_string())?;

                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|err| format!("Failed to create output directory: {err}"))?;
                    }
                    fs::write(&path, &image.bytes)
                        .map_err(|err| format!("Failed to save generated image: {err}"))?;

                    let filename = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or_default()
                        .to_string();
                    batch.images.push(OutputImage {
                        id: image_id,
                        batch_id: batch_id.clone(),
                        provider: request.provider.clone(),
                        model: request.model.clone(),
                        path: path.to_string_lossy().to_string(),
                        filename,
                        created_at: Utc::now(),
                        prompt_snapshot: request.prompt.clone(),
                        metadata: Some(response.metadata.clone()),
                    });
                }
            }
            Err(err) => {
                let proxy_note = settings
                    .proxy_url
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("Proxy configured: {value}"))
                    .unwrap_or_else(|| {
                        "No proxy configured. Set gemini.proxy_url in ~/.visioncraft/config.toml or save Proxy URL in settings.".to_string()
                    });
                last_error = Some(format!("{err}. {proxy_note}"));
            }
        }
    }

    if batch.images.is_empty() {
        batch.status = GenerationStatus::Failed;
        batch.error = Some(last_error.unwrap_or_else(|| "Generation failed.".to_string()));
    } else {
        batch.status = GenerationStatus::Completed;
        batch.completed_at = Some(Utc::now());
    }

    state.current_prompt = request.prompt.clone();
    state.batches.insert(0, batch.clone());
    state.batches.truncate(50);
    save_state(&state).map_err(|err| err.to_string())?;

    if batch.status == GenerationStatus::Failed {
        Err(batch
            .error
            .clone()
            .unwrap_or_else(|| "Generation failed.".to_string()))
    } else {
        Ok(batch)
    }
}

fn filename_provider(provider: &str) -> &str {
    match provider {
        "nano-banana" => "gemini",
        value => value,
    }
}

fn filename_model(model: &str) -> &str {
    match model {
        "gemini-3-pro-image-preview" => "nano-banana-pro",
        "gemini-3.1-flash-image-preview" => "nano-banana-2",
        "gemini-2.5-flash-image" => "nano-banana",
        value => value,
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(6).collect()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_app_state,
            save_app_settings,
            save_current_prompt,
            set_gemini_api_key,
            has_gemini_api_key,
            read_image_data_url,
            generate_images
        ])
        .run(tauri::generate_context!())
        .expect("error while running VisionCraft");
}
