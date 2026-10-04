use crate::{
    app_state::{load_state, save_state},
    error_log::{self, GenerationErrorLog},
    filename_template::resolve_output_path,
    higgsfield_output,
    models::{
        AppSettings, AppState, GenerationBatch, GenerationMediaType, GenerationStatus, OutputVideo,
        ReferenceImageInput, VideoGenerationRequest, VideoInputMode, VideoJobContext,
    },
    reference_image_cache,
    video_provider::{
        create_client, provider_runtime, PollStatus, ProviderClient, ProviderRuntime,
    },
    video_recovery::ActiveVideoBatches,
};
use chrono::{Local, Utc};
use serde_json::{json, Value};
use std::{path::Path, sync::Mutex, time::Duration};
use tokio::{sync::watch, time::Instant};
use uuid::Uuid;

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const MAX_POLL_TIME: Duration = Duration::from_secs(15 * 60);

pub async fn generate(
    mut request: VideoGenerationRequest,
    cancellation: watch::Receiver<bool>,
    active_batches: &ActiveVideoBatches,
) -> Result<GenerationBatch, String> {
    request.validate()?;
    let image_metadata =
        |image: &ReferenceImageInput| json!({ "name": image.name, "mimeType": image.mime_type });
    let starting_image = request.starting_image.as_ref().map(image_metadata);
    let ending_image = request.ending_image.as_ref().map(image_metadata);
    let reference_images = request
        .reference_images
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(image_metadata)
        .collect::<Vec<_>>();
    reference_image_cache::optimize_video_input_images(&mut request)?;

    let prompt_snapshot = request
        .prompt_snapshot
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| request.prompt.clone());
    let settings = load_state().map_err(|error| error.to_string())?.settings;
    let runtime = provider_runtime(&settings, request.provider);
    let batch_id = Uuid::new_v4().to_string();
    let _claim = active_batches.claim(&batch_id)?;
    let mut batch = GenerationBatch {
        id: batch_id,
        media_type: GenerationMediaType::Video,
        provider: request.provider.id().to_string(),
        model: request.model.clone(),
        prompt_snapshot: prompt_snapshot.clone(),
        status: GenerationStatus::Running,
        images: Vec::new(),
        videos: Vec::new(),
        provider_request_id: None,
        video_job: None,
        created_at: Utc::now(),
        completed_at: None,
        error: None,
    };
    let input_image_count = usize::from(starting_image.is_some())
        + usize::from(ending_image.is_some())
        + reference_images.len();

    let client = match create_client(request.provider, &request.model, &runtime, &settings) {
        Ok(client) => client,
        Err(error) => {
            return fail(
                &mut batch,
                &runtime,
                "client_setup_failed",
                error,
                input_image_count,
                None,
            )
        }
    };
    batch.video_job = Some(VideoJobContext {
        provider: request.provider,
        platform: client.platform().to_string(),
        rendered_prompt: request.prompt.trim().to_string(),
        input_mode: request.input_mode,
        starting_image,
        ending_image,
        reference_images,
        options: request.options.clone(),
    });
    // Record the submission before contacting the provider so an interrupted
    // create is visible in history instead of silently lost.
    persist_batch(&batch, Some(&prompt_snapshot))?;
    let request_id = match client.start(&request).await {
        Ok(request_id) => request_id,
        Err(error) => {
            return fail(
                &mut batch,
                &runtime,
                "request_failed",
                error,
                input_image_count,
                None,
            )
        }
    };
    batch.provider_request_id = Some(request_id);
    persist_batch(&batch, None)?;

    monitor_and_finalize(batch, &client, &settings, &runtime, cancellation).await
}

/// Polls an already-submitted provider job, then downloads and records its
/// video. Never submits a new provider job.
pub(crate) async fn monitor_and_finalize(
    mut batch: GenerationBatch,
    client: &ProviderClient,
    settings: &AppSettings,
    runtime: &ProviderRuntime,
    mut cancellation: watch::Receiver<bool>,
) -> Result<GenerationBatch, String> {
    let (Some(request_id), Some(job)) =
        (batch.provider_request_id.clone(), batch.video_job.clone())
    else {
        return Err("Video batch has no submitted provider job to monitor.".to_string());
    };
    let input_image_count = job.input_image_count();
    let prompt_snapshot = batch.prompt_snapshot.clone();
    let batch_id = batch.id.clone();
    let created_at = batch.created_at;
    let model = batch.model.clone();

    let started_at = Instant::now();
    let (video_url, provider_duration, response_metadata) = loop {
        if *cancellation.borrow() {
            return cancel(&mut batch);
        }
        if started_at.elapsed() >= MAX_POLL_TIME {
            return fail(
                &mut batch,
                runtime,
                "poll_timeout",
                format!(
                    "Timed out after 15 minutes while waiting for {} video generation.",
                    job.provider.id()
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
                    &mut batch,
                    runtime,
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
                    &mut batch,
                    runtime,
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
        return cancel(&mut batch);
    }

    let video_id = Uuid::new_v4().to_string();
    let created_video_at = Utc::now();
    let filename_datetime = Local::now();
    let path = match resolve_output_path(
        &settings.output_directory,
        &settings.output_template,
        client.filename_provider(),
        &model,
        "001",
        &short_id(&batch_id),
        "mp4",
        filename_datetime,
    ) {
        Ok(path) => path,
        Err(error) => {
            return fail(
                &mut batch,
                runtime,
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
            &mut batch,
            runtime,
            "output_path_failed",
            "Video output path has no parent directory.".to_string(),
            input_image_count,
            Some(response_metadata),
        );
    };
    if let Err(error) = tokio::fs::create_dir_all(parent).await {
        return fail(
            &mut batch,
            runtime,
            "output_write_failed",
            format!("Failed to create video output directory: {error}"),
            input_image_count,
            Some(response_metadata),
        );
    }
    let partial_path = path.with_extension("mp4.part");
    if let Err(error) = client.download_to(&video_url, &partial_path).await {
        return fail(
            &mut batch,
            runtime,
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
            settings,
            client.filename_provider(),
            &model,
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
                    provider: job.provider.id(),
                    model: &model,
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
    let duration = provider_duration.unwrap_or(job.options.duration);
    let metadata = json!({
        "schemaVersion": 1,
        "mediaType": "video",
        "promptSnapshot": prompt_snapshot,
        "renderedPrompt": job.rendered_prompt,
        "provider": job.provider.id(),
        "model": model,
        "platform": client.platform(),
        "input": {
            "mode": match job.input_mode {
                VideoInputMode::Text => "text-to-video",
                VideoInputMode::Image => "image-to-video",
                VideoInputMode::Frames => "start-end-frame-to-video",
                VideoInputMode::Reference => "reference-to-video",
            },
            "startingImage": job.starting_image,
            "endingImage": job.ending_image,
            "referenceImages": job.reference_images,
        },
        "options": {
            "duration": job.options.duration,
            "aspectRatio": job.options.aspect_ratio,
            "resolution": job.options.resolution,
            "generateAudio": job.options.generate_audio,
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
            &mut batch,
            runtime,
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
        provider: job.provider.id().to_string(),
        model,
        path: path.to_string_lossy().to_string(),
        filename,
        metadata_path: metadata_path.to_string_lossy().to_string(),
        created_at: created_video_at,
        prompt_snapshot: prompt_snapshot.clone(),
        duration,
        aspect_ratio: job.options.aspect_ratio.clone(),
        resolution: job.options.resolution.clone(),
        metadata: Some(metadata),
    });
    batch.status = GenerationStatus::Completed;
    batch.completed_at = Some(Utc::now());
    persist_batch(&batch, None)?;
    Ok(batch)
}

fn cancel(batch: &mut GenerationBatch) -> Result<GenerationBatch, String> {
    batch.status = GenerationStatus::Cancelled;
    batch.completed_at = Some(Utc::now());
    batch.error = Some(
        "Stopped local monitoring. The provider may continue processing and charging for this job."
            .to_string(),
    );
    persist_batch(batch, None)?;
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
    persist_batch(batch, None)?;
    Err(message)
}

/// Upserts a batch into freshly loaded app state so long-running video jobs
/// never write back stale settings or history from when they started.
pub(crate) fn persist_batch(
    batch: &GenerationBatch,
    current_prompt: Option<&str>,
) -> Result<(), String> {
    static PERSIST_LOCK: Mutex<()> = Mutex::new(());
    let _guard = PERSIST_LOCK
        .lock()
        .map_err(|_| "Video history state is unavailable.".to_string())?;
    let mut state = load_state().map_err(|error| error.to_string())?;
    if let Some(prompt) = current_prompt {
        state.current_prompt = prompt.to_string();
    }
    upsert_batch(&mut state, batch);
    save_state(&state).map_err(|error| error.to_string())
}

fn upsert_batch(state: &mut AppState, batch: &GenerationBatch) {
    if let Some(existing) = state.batches.iter_mut().find(|item| item.id == batch.id) {
        *existing = batch.clone();
        return;
    }
    state.batches.insert(0, batch.clone());
    state.batches.truncate(50);
}

fn short_id(id: &str) -> String {
    id.chars().take(6).collect()
}
