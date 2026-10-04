//! Restart-safe recovery for submitted video jobs.
//!
//! Video batches record their provider job before polling starts. After a
//! restart, Higgsfield jobs can resume monitoring by batch id; resuming never
//! submits a new (paid) provider job.

use crate::{
    app_state::{load_state, save_state},
    higgsfield_video,
    models::{
        AppSettings, AppState, GenerationBatch, GenerationMediaType, GenerationStatus,
        VideoProvider,
    },
    video_generation::{monitor_and_finalize, persist_batch},
    video_provider::{create_client, provider_runtime},
};
use chrono::Utc;
use std::{collections::HashSet, sync::Mutex};
use tokio::sync::watch;

const HIGGSFIELD_PLATFORM: &str = "higgsfield";

/// Video batch ids monitored by this process, so one job is never polled or
/// downloaded twice at the same time.
#[derive(Default)]
pub struct ActiveVideoBatches(Mutex<HashSet<String>>);

pub struct ActiveVideoBatchClaim<'a> {
    batches: &'a ActiveVideoBatches,
    id: String,
}

impl ActiveVideoBatches {
    pub fn claim(&self, batch_id: &str) -> Result<ActiveVideoBatchClaim<'_>, String> {
        let mut active = self
            .0
            .lock()
            .map_err(|_| "Video task state is unavailable.".to_string())?;
        if !active.insert(batch_id.to_string()) {
            return Err("This video job is already being monitored.".to_string());
        }
        Ok(ActiveVideoBatchClaim {
            batches: self,
            id: batch_id.to_string(),
        })
    }
}

impl Drop for ActiveVideoBatchClaim<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.batches.0.lock() {
            active.remove(&self.id);
        }
    }
}

/// Settles batches left running by a previous process. Resumable Higgsfield
/// video jobs stay running for the renderer to resume; everything else is
/// marked failed with guidance instead of being resubmitted.
pub fn reconcile_interrupted_batches_on_startup() -> Result<(), String> {
    let mut state = load_state().map_err(|error| error.to_string())?;
    if reconcile_interrupted_batches(&mut state) {
        save_state(&state).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn reconcile_interrupted_batches(state: &mut AppState) -> bool {
    let mut changed = false;
    for batch in state
        .batches
        .iter_mut()
        .filter(|batch| batch.status == GenerationStatus::Running)
    {
        if ensure_resumable(batch).is_ok() {
            continue;
        }
        batch.status = GenerationStatus::Failed;
        batch.completed_at = Some(Utc::now());
        batch.error = Some(interruption_message(batch));
        changed = true;
    }
    changed
}

fn interruption_message(batch: &GenerationBatch) -> String {
    if batch.media_type == GenerationMediaType::Image {
        return "SozoCraft closed before this image task finished.".to_string();
    }
    match batch.provider_request_id.as_deref() {
        None => "SozoCraft closed while submitting this video job, so the provider may or may not \
                 have created it. It was not resubmitted; check your provider account before \
                 generating again."
            .to_string(),
        Some(request_id) => format!(
            "Monitoring stopped when SozoCraft closed. Provider request ID: {request_id}. \
             Desktop resume currently supports Higgsfield video jobs only."
        ),
    }
}

/// Checks that a stored batch is a submitted Higgsfield video job that has not
/// completed. All resume inputs come from Rust-owned state, never the renderer.
fn ensure_resumable(batch: &GenerationBatch) -> Result<(), String> {
    if batch.media_type != GenerationMediaType::Video {
        return Err("Only video jobs can be resumed.".to_string());
    }
    if batch.status == GenerationStatus::Completed {
        return Err("This video job has already completed.".to_string());
    }
    let Some(request_id) = batch.provider_request_id.as_deref() else {
        return Err("This video job has no provider request ID to resume.".to_string());
    };
    let Some(job) = batch.video_job.as_ref() else {
        return Err("This video job predates resumable history.".to_string());
    };
    if job.provider != VideoProvider::Seedance || job.platform != HIGGSFIELD_PLATFORM {
        return Err("Desktop resume currently supports Higgsfield video jobs only.".to_string());
    }
    higgsfield_video::validate_job_id(request_id)
}

/// Pins the submission platform so a later Settings change cannot route the
/// stored job id to a different provider API.
fn resume_settings(mut settings: AppSettings) -> AppSettings {
    settings.seedance_api_platform = HIGGSFIELD_PLATFORM.to_string();
    settings
}

pub async fn resume(
    batch_id: &str,
    cancellation: watch::Receiver<bool>,
    active_batches: &ActiveVideoBatches,
) -> Result<GenerationBatch, String> {
    let _claim = active_batches.claim(batch_id)?;
    let state = load_state().map_err(|error| error.to_string())?;
    let mut batch = state
        .batches
        .into_iter()
        .find(|batch| batch.id == batch_id)
        .ok_or_else(|| "Video job was not found in history.".to_string())?;
    if batch.status == GenerationStatus::Completed && batch.media_type == GenerationMediaType::Video
    {
        // A duplicate resume after the job already finished is a no-op.
        return Ok(batch);
    }
    ensure_resumable(&batch)?;

    let settings = resume_settings(state.settings);
    let runtime = provider_runtime(&settings, VideoProvider::Seedance);
    let client = create_client(VideoProvider::Seedance, &batch.model, &runtime, &settings)?;
    if client.platform() != HIGGSFIELD_PLATFORM {
        return Err("Could not route this job to Higgsfield.".to_string());
    }

    batch.status = GenerationStatus::Running;
    batch.completed_at = None;
    batch.error = None;
    persist_batch(&batch, None)?;
    monitor_and_finalize(batch, &client, &settings, &runtime, cancellation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{VideoGenerationOptions, VideoInputMode, VideoJobContext};

    const JOB_ID: &str = "3f2b8c1e-7a4d-4e9b-9c2a-1d5e6f7a8b9c";

    fn video_batch(status: GenerationStatus, platform: &str) -> GenerationBatch {
        GenerationBatch {
            id: "batch-1".to_string(),
            media_type: GenerationMediaType::Video,
            provider: "seedance".to_string(),
            model: "seedance-2.0".to_string(),
            prompt_snapshot: "A lighthouse at dusk".to_string(),
            status,
            images: Vec::new(),
            videos: Vec::new(),
            provider_request_id: Some(JOB_ID.to_string()),
            video_job: Some(VideoJobContext {
                provider: VideoProvider::Seedance,
                platform: platform.to_string(),
                rendered_prompt: "A lighthouse at dusk".to_string(),
                input_mode: VideoInputMode::Text,
                starting_image: None,
                ending_image: None,
                reference_images: Vec::new(),
                options: VideoGenerationOptions {
                    duration: 5,
                    aspect_ratio: "16:9".to_string(),
                    resolution: "720p".to_string(),
                    generate_audio: None,
                },
            }),
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        }
    }

    #[test]
    fn accepts_unfinished_higgsfield_jobs() {
        for status in [
            GenerationStatus::Running,
            GenerationStatus::Failed,
            GenerationStatus::Cancelled,
        ] {
            assert!(ensure_resumable(&video_batch(status, "higgsfield")).is_ok());
        }
    }

    #[test]
    fn rejects_jobs_that_cannot_be_resumed_safely() {
        let completed = video_batch(GenerationStatus::Completed, "higgsfield");
        assert!(ensure_resumable(&completed)
            .unwrap_err()
            .contains("completed"));

        let ark = video_batch(GenerationStatus::Failed, "volcengine-ark");
        assert!(ensure_resumable(&ark).unwrap_err().contains("Higgsfield"));

        let mut unsubmitted = video_batch(GenerationStatus::Running, "higgsfield");
        unsubmitted.provider_request_id = None;
        assert!(ensure_resumable(&unsubmitted).is_err());

        let mut legacy = video_batch(GenerationStatus::Failed, "higgsfield");
        legacy.video_job = None;
        assert!(ensure_resumable(&legacy).unwrap_err().contains("predates"));

        let mut unsafe_id = video_batch(GenerationStatus::Failed, "higgsfield");
        unsafe_id.provider_request_id = Some("--json; rm".to_string());
        assert!(ensure_resumable(&unsafe_id).is_err());

        let mut image = video_batch(GenerationStatus::Failed, "higgsfield");
        image.media_type = GenerationMediaType::Image;
        assert!(ensure_resumable(&image).is_err());
    }

    #[test]
    fn startup_keeps_resumable_jobs_and_settles_the_rest() {
        let mut unsubmitted = video_batch(GenerationStatus::Running, "higgsfield");
        unsubmitted.id = "unsubmitted".to_string();
        unsubmitted.provider_request_id = None;
        let mut ark = video_batch(GenerationStatus::Running, "volcengine-ark");
        ark.id = "ark".to_string();
        let mut image = video_batch(GenerationStatus::Running, "higgsfield");
        image.id = "image".to_string();
        image.media_type = GenerationMediaType::Image;
        let finished = video_batch(GenerationStatus::Completed, "higgsfield");
        let mut state = AppState::default();
        state.batches = vec![
            video_batch(GenerationStatus::Running, "higgsfield"),
            unsubmitted,
            ark,
            image,
            finished,
        ];

        assert!(reconcile_interrupted_batches(&mut state));
        let statuses = state
            .batches
            .iter()
            .map(|batch| batch.status.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            statuses,
            [
                GenerationStatus::Running,
                GenerationStatus::Failed,
                GenerationStatus::Failed,
                GenerationStatus::Failed,
                GenerationStatus::Completed,
            ]
        );
        assert!(state.batches[1]
            .error
            .as_deref()
            .unwrap()
            .contains("not resubmitted"));
        assert!(state.batches[2].error.as_deref().unwrap().contains(JOB_ID));
        assert!(!reconcile_interrupted_batches(&mut state));
    }

    #[test]
    fn resume_pins_higgsfield_even_after_settings_change() {
        let mut settings = AppSettings::default();
        settings.seedance_api_platform = "ark".to_string();
        assert_eq!(
            resume_settings(settings).seedance_api_platform,
            "higgsfield"
        );
    }

    #[test]
    fn claims_each_batch_once_until_released() {
        let active = ActiveVideoBatches::default();
        let claim = active.claim("batch-1").unwrap();
        assert!(active.claim("batch-1").is_err());
        drop(claim);
        assert!(active.claim("batch-1").is_ok());
    }

    #[test]
    fn job_context_round_trips_and_legacy_batches_parse() {
        let batch = video_batch(GenerationStatus::Running, "higgsfield");
        let value = serde_json::to_value(&batch).unwrap();
        assert_eq!(value["videoJob"]["platform"], "higgsfield");
        let parsed: GenerationBatch = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.video_job.unwrap().options.resolution, "720p");

        let mut legacy = serde_json::to_value(&batch).unwrap();
        legacy.as_object_mut().unwrap().remove("videoJob");
        let parsed: GenerationBatch = serde_json::from_value(legacy).unwrap();
        assert!(parsed.video_job.is_none());
    }
}
