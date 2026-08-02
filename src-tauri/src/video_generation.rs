use crate::{
    app_state::{load_state, save_state},
    error_log::{self, GenerationErrorLog},
    filename_template::resolve_output_path,
    google_veo::{GoogleVeoClient, GoogleVeoStatus},
    higgsfield_output,
    higgsfield_video::{HiggsfieldVideoClient, HiggsfieldVideoStatus},
    local_config,
    models::{
        AppSettings, AppState, GenerationBatch, GenerationMediaType, GenerationStatus, OutputVideo,
        VideoGenerationRequest, VideoInputMode, VideoProvider,
    },
    reference_image_cache,
    seedance_video::{SeedanceVideoClient, SeedanceVideoStatus},
    xai_video::{XaiVideoClient, XaiVideoStatus},
};
use chrono::{Local, Utc};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use tokio::{sync::watch, time::Instant};
use uuid::Uuid;

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const MAX_POLL_TIME: Duration = Duration::from_secs(15 * 60);

enum ProviderClient {
    Seedance(SeedanceVideoClient),
    Higgsfield(HiggsfieldVideoClient),
    Grok(XaiVideoClient),
    Veo(GoogleVeoClient),
}

enum PollStatus {
    Pending,
    Done { url: String, duration: Option<u8> },
    Failed(String),
}

struct PollResponse {
    status: PollStatus,
    metadata: Value,
}

#[derive(Clone)]
struct ProviderRuntime {
    base_url: Option<String>,
    proxy_url: Option<String>,
    timeout_seconds: u64,
    higgsfield_cli_path: Option<String>,
}

impl ProviderClient {
    fn is_higgsfield(&self) -> bool {
        matches!(self, Self::Higgsfield(_))
    }

    async fn start(&self, request: &VideoGenerationRequest) -> Result<String, String> {
        match self {
            Self::Seedance(client) => client.start(request).await,
            Self::Higgsfield(client) => client.start(request).await,
            Self::Grok(client) => client
                .start(request)
                .await
                .map_err(|error| error.to_string()),
            Self::Veo(client) => client.start(request).await,
        }
    }

    async fn poll(&self, request_id: &str) -> Result<PollResponse, String> {
        match self {
            Self::Seedance(client) => {
                let response = client.poll(request_id).await?;
                let status = match response.status {
                    SeedanceVideoStatus::Pending => PollStatus::Pending,
                    SeedanceVideoStatus::Done { url, duration } => {
                        PollStatus::Done { url, duration }
                    }
                    SeedanceVideoStatus::Failed(message) => PollStatus::Failed(message),
                    SeedanceVideoStatus::Cancelled => PollStatus::Failed(
                        "Volcengine Ark cancelled the Seedance task.".to_string(),
                    ),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
            Self::Higgsfield(client) => {
                let response = client.poll(request_id).await?;
                let status = match response.status {
                    HiggsfieldVideoStatus::Pending => PollStatus::Pending,
                    HiggsfieldVideoStatus::Done { url } => PollStatus::Done {
                        url,
                        duration: None,
                    },
                    HiggsfieldVideoStatus::Failed(message) => PollStatus::Failed(message),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
            Self::Grok(client) => {
                let response = client
                    .poll(request_id)
                    .await
                    .map_err(|error| error.to_string())?;
                let status = match response.status {
                    XaiVideoStatus::Pending => PollStatus::Pending,
                    XaiVideoStatus::Done { url, duration } => PollStatus::Done { url, duration },
                    XaiVideoStatus::Failed(message) => PollStatus::Failed(message),
                    XaiVideoStatus::Expired => PollStatus::Failed(
                        "xAI video request expired before completion.".to_string(),
                    ),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
            Self::Veo(client) => {
                let response = client.poll(request_id).await?;
                let status = match response.status {
                    GoogleVeoStatus::Pending => PollStatus::Pending,
                    GoogleVeoStatus::Done { url } => PollStatus::Done {
                        url,
                        duration: None,
                    },
                    GoogleVeoStatus::Failed(message) => PollStatus::Failed(message),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
        }
    }

    async fn download_to(&self, video_url: &str, output_path: &Path) -> Result<u64, String> {
        match self {
            Self::Seedance(client) => client.download_to(video_url, output_path).await,
            Self::Higgsfield(client) => client.download_to(video_url, output_path).await,
            Self::Grok(client) => client
                .download_to(video_url, output_path)
                .await
                .map_err(|error| error.to_string()),
            Self::Veo(client) => client.download_to(video_url, output_path).await,
        }
    }

    fn platform(&self) -> &'static str {
        match self {
            Self::Higgsfield(_) => "higgsfield",
            Self::Seedance(_) => "volcengine-ark",
            Self::Grok(_) => "xai",
            Self::Veo(_) => "google-gemini-api",
        }
    }

    fn filename_provider(&self) -> &'static str {
        match self {
            Self::Higgsfield(_) => "higgsfield",
            Self::Seedance(_) => "volcengine",
            Self::Grok(_) => "xai",
            Self::Veo(_) => "google",
        }
    }
}

pub async fn generate(
    mut request: VideoGenerationRequest,
    mut cancellation: watch::Receiver<bool>,
) -> Result<GenerationBatch, String> {
    request.validate()?;
    let starting_image_metadata = request
        .starting_image
        .as_ref()
        .map(|image| json!({ "name": image.name, "mimeType": image.mime_type }));
    let ending_image_metadata = request
        .ending_image
        .as_ref()
        .map(|image| json!({ "name": image.name, "mimeType": image.mime_type }));
    let reference_image_metadata = request
        .reference_images
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|image| json!({ "name": image.name, "mimeType": image.mime_type }))
        .collect::<Vec<_>>();
    let input_image_count = usize::from(request.starting_image.is_some())
        + usize::from(request.ending_image.is_some())
        + reference_image_metadata.len();
    reference_image_cache::optimize_video_input_images(&mut request)?;

    let prompt_snapshot = request
        .prompt_snapshot
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| request.prompt.clone());
    let mut state = load_state().map_err(|error| error.to_string())?;
    let settings = state.settings.clone();
    let runtime = provider_runtime(&settings, request.provider);
    let batch_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let mut batch = GenerationBatch {
        id: batch_id.clone(),
        media_type: GenerationMediaType::Video,
        provider: request.provider.id().to_string(),
        model: request.model.clone(),
        prompt_snapshot: prompt_snapshot.clone(),
        status: GenerationStatus::Running,
        images: Vec::new(),
        videos: Vec::new(),
        provider_request_id: None,
        created_at,
        completed_at: None,
        error: None,
    };

    let client = match create_client(request.provider, &runtime, &settings) {
        Ok(client) => client,
        Err(error) => {
            return fail(
                &mut state,
                &mut batch,
                &runtime,
                "client_setup_failed",
                error,
                input_image_count,
                None,
            )
        }
    };
    let request_id = match client.start(&request).await {
        Ok(request_id) => request_id,
        Err(error) => {
            return fail(
                &mut state,
                &mut batch,
                &runtime,
                "request_failed",
                error,
                input_image_count,
                None,
            )
        }
    };
    batch.provider_request_id = Some(request_id.clone());

    let started_at = Instant::now();
    let (video_url, provider_duration, response_metadata) = loop {
        if *cancellation.borrow() {
            return cancel(&mut state, &mut batch, &prompt_snapshot);
        }
        if started_at.elapsed() >= MAX_POLL_TIME {
            return fail(
                &mut state,
                &mut batch,
                &runtime,
                "poll_timeout",
                format!(
                    "Timed out after 15 minutes while waiting for {} video generation.",
                    request.provider.id()
                ),
                input_image_count,
                None,
            );
        }

        let poll_result = tokio::select! {
            result = client.poll(&request_id) => Some(result),
            changed = cancellation.changed() => {
                if changed.is_ok() && *cancellation.borrow() { None } else { continue }
            }
        };
        let Some(poll_result) = poll_result else {
            continue;
        };
        let poll = match poll_result {
            Ok(poll) => poll,
            Err(error) => {
                return fail(
                    &mut state,
                    &mut batch,
                    &runtime,
                    "poll_failed",
                    error,
                    input_image_count,
                    None,
                )
            }
        };
        match poll.status {
            PollStatus::Pending => {}
            PollStatus::Done { url, duration } => break (url, duration, poll.metadata),
            PollStatus::Failed(message) => {
                return fail(
                    &mut state,
                    &mut batch,
                    &runtime,
                    "provider_failed",
                    message,
                    input_image_count,
                    Some(poll.metadata),
                )
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(POLL_INTERVAL) => {}
            _ = cancellation.changed() => {}
        }
    };

    if *cancellation.borrow() {
        return cancel(&mut state, &mut batch, &prompt_snapshot);
    }

    let video_id = Uuid::new_v4().to_string();
    let created_video_at = Utc::now();
    let filename_datetime = Local::now();
    let path = match resolve_output_path(
        &settings.output_directory,
        &settings.output_template,
        client.filename_provider(),
        &request.model,
        "001",
        &short_id(&batch_id),
        "mp4",
        filename_datetime,
    ) {
        Ok(path) => path,
        Err(error) => {
            return fail(
                &mut state,
                &mut batch,
                &runtime,
                "output_path_failed",
                error.to_string(),
                input_image_count,
                Some(response_metadata),
            )
        }
    };
    let metadata_path = path.with_extension("mp4.json");
    let Some(parent) = path.parent() else {
        return fail(
            &mut state,
            &mut batch,
            &runtime,
            "output_path_failed",
            "Video output path has no parent directory.".to_string(),
            input_image_count,
            Some(response_metadata),
        );
    };
    if let Err(error) = tokio::fs::create_dir_all(parent).await {
        return fail(
            &mut state,
            &mut batch,
            &runtime,
            "output_write_failed",
            format!("Failed to create video output directory: {error}"),
            input_image_count,
            Some(response_metadata),
        );
    }
    let partial_path = path.with_extension("mp4.part");
    if let Err(error) = client.download_to(&video_url, &partial_path).await {
        return fail(
            &mut state,
            &mut batch,
            &runtime,
            "download_failed",
            error,
            input_image_count,
            Some(response_metadata),
        );
    }
    let higgsfield_output = if client.is_higgsfield() && settings.higgsfield_output_enabled {
        let source_filename = higgsfield_output::source_filename_from_url(&video_url)
            .filter(|value| Path::new(value).extension().is_some())
            .unwrap_or_else(|| format!("hf_{request_id}.mp4"));
        match higgsfield_output::archive_file(
            &settings,
            client.filename_provider(),
            &request.model,
            "001",
            &short_id(&batch_id),
            &source_filename,
            filename_datetime,
            &partial_path,
        )
        .await
        {
            Ok(Some(path)) => Some(json!({
                "sourceFilename": source_filename,
                "path": path.to_string_lossy(),
            })),
            Ok(None) => None,
            Err(message) => {
                error_log::log_generation_error(&GenerationErrorLog {
                    timestamp: Utc::now(),
                    batch_id: &batch_id,
                    provider: request.provider.id(),
                    model: &request.model,
                    attempt: 1,
                    kind: "higgsfield_output_failed",
                    message: &message,
                    base_url: runtime.base_url.as_deref(),
                    proxy_url: runtime.proxy_url.as_deref().map(|_| "<configured>"),
                    timeout_seconds: runtime.timeout_seconds,
                    reference_image_count: input_image_count,
                    response_metadata: None,
                });
                Some(json!({
                    "sourceFilename": source_filename,
                    "warning": message,
                }))
            }
        }
    } else {
        None
    };
    let duration = provider_duration.unwrap_or(request.options.duration);
    let metadata = json!({
        "schemaVersion": 1,
        "mediaType": "video",
        "promptSnapshot": prompt_snapshot,
        "renderedPrompt": request.prompt.trim(),
        "provider": request.provider.id(),
        "model": request.model,
        "platform": client.platform(),
        "input": {
            "mode": match request.input_mode {
                VideoInputMode::Text => "text-to-video",
                VideoInputMode::Image => "image-to-video",
                VideoInputMode::Frames => "start-end-frame-to-video",
                VideoInputMode::Reference => "reference-to-video",
            },
            "startingImage": starting_image_metadata,
            "endingImage": ending_image_metadata,
            "referenceImages": reference_image_metadata,
        },
        "options": {
            "duration": request.options.duration,
            "aspectRatio": request.options.aspect_ratio,
            "resolution": request.options.resolution,
            "generateAudio": request.options.generate_audio,
        },
        "batchId": batch_id,
        "videoId": video_id,
        "providerRequestId": request_id,
        "createdAt": created_video_at,
        "batchCreatedAt": created_at,
        "responseMetadata": response_metadata,
        "higgsfieldOutput": higgsfield_output,
    });

    if let Err(error) =
        finalize_video_and_metadata(&partial_path, &path, &metadata_path, &metadata).await
    {
        return fail(
            &mut state,
            &mut batch,
            &runtime,
            "output_write_failed",
            error,
            input_image_count,
            Some(metadata),
        );
    }

    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    batch.videos.push(OutputVideo {
        id: video_id,
        batch_id,
        provider: request.provider.id().to_string(),
        model: request.model,
        path: path.to_string_lossy().to_string(),
        filename,
        metadata_path: metadata_path.to_string_lossy().to_string(),
        created_at: created_video_at,
        prompt_snapshot: prompt_snapshot.clone(),
        duration,
        aspect_ratio: request.options.aspect_ratio,
        resolution: request.options.resolution,
        metadata: Some(metadata),
    });
    batch.status = GenerationStatus::Completed;
    batch.completed_at = Some(Utc::now());
    persist_batch(&mut state, &batch, &prompt_snapshot)?;
    Ok(batch)
}

fn provider_runtime(settings: &AppSettings, provider: VideoProvider) -> ProviderRuntime {
    match provider {
        VideoProvider::Seedance => ProviderRuntime {
            base_url: (settings.seedance_api_platform != "higgsfield")
                .then(|| settings.ark_base_url.clone())
                .flatten(),
            proxy_url: if settings.seedance_api_platform == "higgsfield" {
                settings
                    .effective_higgsfield_proxy_url()
                    .map(str::to_string)
            } else {
                settings
                    .ark_proxy_enabled
                    .then(|| settings.proxy_url.clone())
                    .flatten()
            },
            timeout_seconds: settings.ark_timeout_seconds,
            higgsfield_cli_path: settings.higgsfield_cli_path.clone(),
        },
        VideoProvider::GrokImagine => ProviderRuntime {
            base_url: settings.xai_base_url.clone(),
            proxy_url: settings
                .xai_proxy_enabled
                .then(|| settings.proxy_url.clone())
                .flatten(),
            timeout_seconds: settings.xai_timeout_seconds,
            higgsfield_cli_path: None,
        },
        VideoProvider::GoogleVeo => ProviderRuntime {
            base_url: settings.optional_base_url.clone(),
            proxy_url: settings
                .gemini_proxy_enabled
                .then(|| settings.proxy_url.clone())
                .flatten(),
            timeout_seconds: settings.gemini_timeout_seconds,
            higgsfield_cli_path: None,
        },
    }
}

fn create_client(
    provider: VideoProvider,
    runtime: &ProviderRuntime,
    settings: &AppSettings,
) -> Result<ProviderClient, String> {
    match provider {
        VideoProvider::Seedance if settings.seedance_api_platform == "higgsfield" => {
            Ok(ProviderClient::Higgsfield(HiggsfieldVideoClient::new(
                runtime.higgsfield_cli_path.clone(),
                runtime.proxy_url.clone(),
                runtime.timeout_seconds,
            )?))
        }
        VideoProvider::Seedance => Ok(ProviderClient::Seedance(SeedanceVideoClient::new(
            local_config::get_ark_api_key().map_err(|error| error.to_string())?,
            runtime.base_url.clone(),
            runtime.proxy_url.clone(),
            runtime.timeout_seconds,
        )?)),
        VideoProvider::GrokImagine => Ok(ProviderClient::Grok(
            XaiVideoClient::new(
                local_config::get_xai_api_key().map_err(|error| error.to_string())?,
                runtime.base_url.clone(),
                runtime.proxy_url.clone(),
                runtime.timeout_seconds,
            )
            .map_err(|error| error.to_string())?,
        )),
        VideoProvider::GoogleVeo => Ok(ProviderClient::Veo(GoogleVeoClient::new(
            local_config::get_gemini_api_key().map_err(|error| error.to_string())?,
            runtime.base_url.clone(),
            runtime.proxy_url.clone(),
            runtime.timeout_seconds,
        )?)),
    }
}

fn cancel(
    state: &mut AppState,
    batch: &mut GenerationBatch,
    prompt_snapshot: &str,
) -> Result<GenerationBatch, String> {
    batch.status = GenerationStatus::Cancelled;
    batch.completed_at = Some(Utc::now());
    batch.error = Some(
        "Stopped local monitoring. The provider may continue processing and charging for this job."
            .to_string(),
    );
    persist_batch(state, batch, prompt_snapshot)?;
    Ok(batch.clone())
}

async fn finalize_video_and_metadata(
    partial_path: &Path,
    video_path: &Path,
    metadata_path: &Path,
    metadata: &Value,
) -> Result<(), String> {
    if let Err(error) = tokio::fs::rename(partial_path, video_path).await {
        let _ = tokio::fs::remove_file(partial_path).await;
        return Err(format!("Failed to finalize generated video: {error}"));
    }
    let metadata_bytes = match serde_json::to_vec_pretty(metadata) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = tokio::fs::remove_file(video_path).await;
            return Err(format!("Failed to encode video metadata: {error}"));
        }
    };
    if let Err(error) = tokio::fs::write(metadata_path, metadata_bytes).await {
        let _ = tokio::fs::remove_file(video_path).await;
        return Err(format!("Failed to write video metadata: {error}"));
    }
    Ok(())
}

fn fail(
    state: &mut AppState,
    batch: &mut GenerationBatch,
    runtime: &ProviderRuntime,
    kind: &str,
    message: String,
    reference_image_count: usize,
    response_metadata: Option<Value>,
) -> Result<GenerationBatch, String> {
    error_log::log_generation_error(&GenerationErrorLog {
        timestamp: Utc::now(),
        batch_id: &batch.id,
        provider: &batch.provider,
        model: &batch.model,
        attempt: 1,
        kind,
        message: &message,
        base_url: runtime.base_url.as_deref(),
        proxy_url: runtime.proxy_url.as_deref().map(|_| "<configured>"),
        timeout_seconds: runtime.timeout_seconds,
        reference_image_count,
        response_metadata,
    });
    batch.status = GenerationStatus::Failed;
    batch.completed_at = Some(Utc::now());
    batch.error = Some(message.clone());
    persist_batch(state, batch, &batch.prompt_snapshot.clone())?;
    Err(message)
}

fn persist_batch(
    state: &mut AppState,
    batch: &GenerationBatch,
    prompt_snapshot: &str,
) -> Result<(), String> {
    state.current_prompt = prompt_snapshot.to_string();
    state.batches.retain(|item| item.id != batch.id);
    state.batches.insert(0, batch.clone());
    state.batches.truncate(50);
    save_state(state).map_err(|error| error.to_string())
}

fn short_id(id: &str) -> String {
    id.chars().take(6).collect()
}
