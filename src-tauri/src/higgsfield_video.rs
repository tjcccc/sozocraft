use crate::{
    higgsfield::{
        collect_result_urls, find_scalar_key, job_id_from_metadata, normalized_cli_path,
        parse_json_output, run_cli,
    },
    local_config,
    models::{ReferenceImageInput, VideoGenerationRequest},
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::{redirect, Client, Proxy, Url};
use serde_json::{json, Value};
use std::{
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const MAX_VIDEO_BYTES: usize = 1024 * 1024 * 1024;
const CREATE_RECOVERY_ATTEMPTS: u8 = 3;
const CREATE_RECOVERY_CLOCK_SKEW_SECONDS: i64 = 30;
const CREATE_RECOVERY_LIST_SIZE: &str = "20";

#[derive(Debug, Clone)]
pub struct HiggsfieldVideoClient {
    cli_path: String,
    proxy_url: Option<String>,
    timeout_seconds: u64,
    download_client: Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HiggsfieldVideoStatus {
    Pending,
    Done { url: String },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct HiggsfieldVideoPollResponse {
    pub status: HiggsfieldVideoStatus,
    pub metadata: Value,
}

#[derive(Debug, Default)]
struct TempInputFiles {
    directory: Option<PathBuf>,
    paths: Vec<PathBuf>,
}

impl Drop for TempInputFiles {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = fs::remove_file(path);
        }
        if let Some(directory) = &self.directory {
            let _ = fs::remove_dir(directory);
        }
    }
}

impl HiggsfieldVideoClient {
    pub fn new(
        cli_path: Option<String>,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, String> {
        let proxy_url = proxy_url.filter(|value| !value.trim().is_empty());
        let timeout = Duration::from_secs(timeout_seconds.max(30));
        let mut builder = Client::builder()
            .timeout(timeout)
            .redirect(redirect::Policy::custom(|attempt| {
                if is_safe_result_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }));
        if let Some(value) = proxy_url.as_deref() {
            builder = builder.proxy(Proxy::all(value.trim()).map_err(|error| error.to_string())?);
        }
        Ok(Self {
            cli_path: normalized_cli_path(cli_path),
            proxy_url,
            timeout_seconds,
            download_client: builder.build().map_err(|error| error.to_string())?,
        })
    }

    pub async fn start(&self, request: &VideoGenerationRequest) -> Result<String, String> {
        let input_files = materialize_inputs(request)?;
        let args = build_create_args(request, &input_files)?;
        let (job_type, mode) = job_type_and_mode(&request.model)?;
        let started_at = Utc::now();
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = match run_cli(
            &self.cli_path,
            &arg_refs,
            Duration::from_secs(self.timeout_seconds.max(30) + 60),
            self.proxy_url.as_deref(),
        )
        .await
        {
            Ok(output) => output,
            Err(error) if is_ambiguous_create_error(&error) => {
                return match self
                    .recover_created_job(request, job_type, mode, started_at)
                    .await
                {
                    Ok(Some(job_id)) => Ok(job_id),
                    Ok(None) => Err(format!(
                        "{error} The request may have reached Higgsfield, but SozoCraft could not uniquely identify a matching recent job. Check Higgsfield History before retrying."
                    )),
                    Err(recovery_error) => Err(format!(
                        "{error} The request may have reached Higgsfield, and the recovery check failed: {recovery_error}"
                    )),
                };
            }
            Err(error) => return Err(error),
        };
        let metadata = parse_json_output(&output.stdout)
            .unwrap_or_else(|| json!({ "stdout": output.stdout, "stderr": output.stderr }));
        let job_id = video_job_id_from_metadata(&metadata).ok_or_else(|| {
            format!(
                "Higgsfield CLI created no identifiable video job. Response: {}",
                truncate(&metadata.to_string())
            )
        })?;
        validate_job_id(&job_id)?;
        Ok(job_id)
    }

    async fn recover_created_job(
        &self,
        request: &VideoGenerationRequest,
        job_type: &str,
        mode: Option<&str>,
        started_at: DateTime<Utc>,
    ) -> Result<Option<String>, String> {
        let mut last_error = None;
        let mut queried_history = false;

        for attempt in 0..CREATE_RECOVERY_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            let list_output = match run_cli(
                &self.cli_path,
                &[
                    "generate",
                    "list",
                    "--video",
                    "--size",
                    CREATE_RECOVERY_LIST_SIZE,
                    "--json",
                    "--no-color",
                ],
                Duration::from_secs(30),
                self.proxy_url.as_deref(),
            )
            .await
            {
                Ok(output) => output,
                Err(error) => {
                    last_error = Some(error);
                    continue;
                }
            };
            queried_history = true;
            let Some(list) = parse_json_output(&list_output.stdout) else {
                last_error = Some("Higgsfield returned invalid JSON for recent jobs.".to_string());
                continue;
            };
            let candidate_ids = recent_job_ids(&list, job_type, started_at, Utc::now());
            let mut matches = Vec::new();

            for candidate_id in candidate_ids.into_iter().take(5) {
                let detail_output = match run_cli(
                    &self.cli_path,
                    &["generate", "get", &candidate_id, "--json", "--no-color"],
                    Duration::from_secs(30),
                    self.proxy_url.as_deref(),
                )
                .await
                {
                    Ok(output) => output,
                    Err(error) => {
                        last_error = Some(error);
                        continue;
                    }
                };
                let Some(detail) = parse_json_output(&detail_output.stdout) else {
                    continue;
                };
                if job_matches_request(&detail, request, job_type, mode) {
                    matches.push(candidate_id);
                }
            }

            if matches.len() == 1 {
                return Ok(matches.pop());
            }
            if matches.len() > 1 {
                return Ok(None);
            }
        }

        if !queried_history {
            return Err(
                last_error.unwrap_or_else(|| "Higgsfield recent-job lookup failed.".to_string())
            );
        }
        Ok(None)
    }

    pub async fn poll(&self, job_id: &str) -> Result<HiggsfieldVideoPollResponse, String> {
        validate_job_id(job_id)?;
        let output = run_cli(
            &self.cli_path,
            &["generate", "get", job_id, "--json", "--no-color"],
            Duration::from_secs(30),
            self.proxy_url.as_deref(),
        )
        .await?;
        let metadata = parse_json_output(&output.stdout)
            .unwrap_or_else(|| json!({ "stdout": output.stdout, "stderr": output.stderr }));
        if let Some(url) = collect_result_urls(&metadata).into_iter().next() {
            let parsed = Url::parse(&url)
                .map_err(|_| "Higgsfield returned an invalid video URL.".to_string())?;
            if !is_safe_result_url(&parsed) {
                return Err("Higgsfield returned an unsafe video URL.".to_string());
            }
            return Ok(HiggsfieldVideoPollResponse {
                status: HiggsfieldVideoStatus::Done { url },
                metadata: without_result_urls(metadata),
            });
        }

        let status = find_scalar_key(&metadata, &["status", "state"])
            .unwrap_or_default()
            .to_ascii_lowercase();
        let response = if ["failed", "error", "cancelled", "canceled", "rejected"]
            .iter()
            .any(|value| status.contains(value))
        {
            let reason = find_scalar_key(
                &metadata,
                &[
                    "message",
                    "error",
                    "reason",
                    "failure_reason",
                    "failureReason",
                    "detail",
                ],
            )
            .unwrap_or_else(|| metadata.to_string());
            HiggsfieldVideoStatus::Failed(format!(
                "Higgsfield video generation failed: {}",
                truncate(&reason)
            ))
        } else if ["completed", "complete", "succeeded", "success", "done"]
            .iter()
            .any(|value| status.contains(value))
        {
            HiggsfieldVideoStatus::Failed(format!(
                "Higgsfield completed without a video URL. Response: {}",
                truncate(&metadata.to_string())
            ))
        } else {
            HiggsfieldVideoStatus::Pending
        };
        Ok(HiggsfieldVideoPollResponse {
            status: response,
            metadata,
        })
    }

    pub async fn download_to(&self, video_url: &str, output_path: &Path) -> Result<u64, String> {
        let url = Url::parse(video_url)
            .map_err(|_| "Higgsfield returned an invalid video URL.".to_string())?;
        if !is_safe_result_url(&url) {
            return Err("Higgsfield returned an unsafe video URL.".to_string());
        }
        let mut response = self
            .download_client
            .get(url)
            .send()
            .await
            .map_err(|error| format!("Higgsfield video download failed: {error}"))?;
        if !is_safe_result_url(response.url()) {
            return Err("Higgsfield redirected to an unsafe video URL.".to_string());
        }
        if !response.status().is_success() {
            return Err(format!(
                "Higgsfield video download failed with HTTP {}.",
                response.status()
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_VIDEO_BYTES as u64)
        {
            return Err("Higgsfield video exceeds the 1 GiB download limit.".to_string());
        }

        let mut file = tokio::fs::File::create(output_path)
            .await
            .map_err(|error| format!("Failed to create video output: {error}"))?;
        let mut written = 0_u64;
        let mut header = Vec::with_capacity(12);
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| format!("Higgsfield video download failed: {error}"))?
        {
            written = written.saturating_add(chunk.len() as u64);
            if written > MAX_VIDEO_BYTES as u64 {
                let _ = tokio::fs::remove_file(output_path).await;
                return Err("Higgsfield video exceeds the 1 GiB download limit.".to_string());
            }
            if header.len() < 12 {
                let remaining = 12 - header.len();
                header.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("Failed to write Higgsfield video: {error}"))?;
        }
        file.flush()
            .await
            .map_err(|error| format!("Failed to flush Higgsfield video: {error}"))?;
        if header.len() < 12 || header.get(4..8) != Some(b"ftyp".as_slice()) {
            drop(file);
            let _ = tokio::fs::remove_file(output_path).await;
            return Err("Higgsfield returned a response that is not an MP4 video.".to_string());
        }
        Ok(written)
    }
}

fn build_create_args(
    request: &VideoGenerationRequest,
    inputs: &TempInputFiles,
) -> Result<Vec<String>, String> {
    let (job_type, mode) = job_type_and_mode(&request.model)?;
    let mut args = vec![
        "generate".to_string(),
        "create".to_string(),
        job_type.to_string(),
        "--prompt".to_string(),
        request.prompt.trim().to_string(),
        "--aspect_ratio".to_string(),
        request.options.aspect_ratio.clone(),
        "--duration".to_string(),
        request.options.duration.to_string(),
        "--resolution".to_string(),
        request.options.resolution.clone(),
        "--generate_audio".to_string(),
        request.options.generate_audio.unwrap_or(true).to_string(),
        "--bitrate_mode".to_string(),
        "standard".to_string(),
    ];
    if let Some(mode) = mode {
        args.extend(["--mode".to_string(), mode.to_string()]);
    }

    let mut path_index = 0;
    if request.starting_image.is_some() {
        args.extend([
            "--start-image".to_string(),
            input_path(inputs, path_index)?
                .to_string_lossy()
                .to_string(),
        ]);
        path_index += 1;
    }
    if request.ending_image.is_some() {
        args.extend([
            "--end-image".to_string(),
            input_path(inputs, path_index)?
                .to_string_lossy()
                .to_string(),
        ]);
        path_index += 1;
    }
    for _ in request.reference_images.as_deref().unwrap_or_default() {
        args.extend([
            "--image-references".to_string(),
            input_path(inputs, path_index)?
                .to_string_lossy()
                .to_string(),
        ]);
        path_index += 1;
    }
    args.extend(["--json".to_string(), "--no-color".to_string()]);
    Ok(args)
}

fn job_type_and_mode(model: &str) -> Result<(&'static str, Option<&'static str>), String> {
    Ok(match model {
        "doubao-seedance-2-0-260128" => ("seedance_2_0", Some("std")),
        "doubao-seedance-2-0-fast-260128" => ("seedance_2_0", Some("fast")),
        "doubao-seedance-2-0-mini-260615" => ("seedance_2_0_mini", None),
        _ => return Err(format!("Unsupported Higgsfield Seedance model: {model}")),
    })
}

fn is_ambiguous_create_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("request failed (no response received)")
        || error.contains("higgsfield cli command timed out")
}

fn recent_job_ids(
    value: &Value,
    job_type: &str,
    started_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<String> {
    let items = value
        .as_array()
        .or_else(|| value.get("jobs").and_then(Value::as_array))
        .map(Vec::as_slice)
        .unwrap_or_default();
    let earliest = started_at - ChronoDuration::seconds(CREATE_RECOVERY_CLOCK_SKEW_SECONDS);
    let latest = now + ChronoDuration::seconds(CREATE_RECOVERY_CLOCK_SKEW_SECONDS);

    items
        .iter()
        .filter(|item| item.get("job_type").and_then(Value::as_str) == Some(job_type))
        .filter_map(|item| {
            let created_at = DateTime::parse_from_rfc3339(item.get("created_at")?.as_str()?)
                .ok()?
                .with_timezone(&Utc);
            if created_at < earliest || created_at > latest {
                return None;
            }
            let id = item.get("id")?.as_str()?.trim().to_string();
            validate_job_id(&id).ok()?;
            Some(id)
        })
        .collect()
}

fn job_matches_request(
    value: &Value,
    request: &VideoGenerationRequest,
    job_type: &str,
    mode: Option<&str>,
) -> bool {
    if value.get("job_type").and_then(Value::as_str) != Some(job_type) {
        return false;
    }
    let Some(params) = value.get("params") else {
        return false;
    };
    let expected_media_count = usize::from(request.starting_image.is_some())
        + usize::from(request.ending_image.is_some())
        + request
            .reference_images
            .as_deref()
            .map_or(0, |images| images.len());
    let media_count = params
        .get("medias")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    params.get("prompt").and_then(Value::as_str).map(str::trim) == Some(request.prompt.trim())
        && params.get("aspect_ratio").and_then(Value::as_str)
            == Some(request.options.aspect_ratio.as_str())
        && params.get("duration").and_then(Value::as_u64)
            == Some(u64::from(request.options.duration))
        && params.get("resolution").and_then(Value::as_str)
            == Some(request.options.resolution.as_str())
        && params.get("generate_audio").and_then(Value::as_bool)
            == Some(request.options.generate_audio.unwrap_or(true))
        && mode.is_none_or(|expected| params.get("mode").and_then(Value::as_str) == Some(expected))
        && media_count == expected_media_count
}

fn input_path(inputs: &TempInputFiles, index: usize) -> Result<&Path, String> {
    inputs
        .paths
        .get(index)
        .map(PathBuf::as_path)
        .ok_or_else(|| "Higgsfield video input cache is incomplete.".to_string())
}

fn materialize_inputs(request: &VideoGenerationRequest) -> Result<TempInputFiles, String> {
    let images = request
        .starting_image
        .iter()
        .chain(request.ending_image.iter())
        .chain(
            request
                .reference_images
                .as_deref()
                .unwrap_or_default()
                .iter(),
        )
        .collect::<Vec<_>>();
    if images.is_empty() {
        return Ok(TempInputFiles::default());
    }
    let directory = local_config::config_dir()
        .join("cache")
        .join("higgsfield-video-inputs")
        .join(Uuid::new_v4().to_string());
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create Higgsfield video input cache: {error}"))?;
    let mut inputs = TempInputFiles {
        directory: Some(directory.clone()),
        paths: Vec::with_capacity(images.len()),
    };
    for (index, image) in images.into_iter().enumerate() {
        inputs.paths.push(write_input(&directory, image, index)?);
    }
    Ok(inputs)
}

fn write_input(
    directory: &Path,
    image: &ReferenceImageInput,
    index: usize,
) -> Result<PathBuf, String> {
    if image.asset_id.is_some() {
        return Err(
            "Volcengine Ark asset ids cannot be used as Higgsfield media references.".to_string(),
        );
    }
    let extension = match image.mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    };
    let path = directory.join(format!("{:03}.{extension}", index + 1));
    let bytes = general_purpose::STANDARD
        .decode(image.data.trim())
        .map_err(|error| {
            format!(
                "Failed to decode Higgsfield video input {}: {error}",
                image.name
            )
        })?;
    fs::write(&path, bytes)
        .map_err(|error| format!("Failed to write Higgsfield video input: {error}"))?;
    Ok(path)
}

fn validate_job_id(value: &str) -> Result<(), String> {
    if Uuid::parse_str(value).is_ok() {
        return Ok(());
    }
    if !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Ok(());
    }
    Err("Higgsfield returned an invalid video job id.".to_string())
}

fn video_job_id_from_metadata(value: &Value) -> Option<String> {
    job_id_from_metadata(value).or_else(|| match value {
        Value::String(id) => Some(id.trim().to_string()).filter(|id| !id.is_empty()),
        Value::Array(items) if items.len() == 1 => items
            .first()
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string),
        _ => None,
    })
}

fn is_safe_result_url(url: &Url) -> bool {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => !(ip.is_private() || ip.is_loopback() || ip.is_link_local()),
        Ok(IpAddr::V6(ip)) => !(ip.is_loopback() || ip.is_unspecified()),
        Err(_) => true,
    }
}

fn truncate(value: &str) -> String {
    const LIMIT: usize = 600;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    format!("{}...", value.chars().take(LIMIT - 3).collect::<String>())
}

fn without_result_urls(mut value: Value) -> Value {
    fn remove(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for key in [
                    "result_url",
                    "resultUrl",
                    "result_urls",
                    "resultUrls",
                    "output_url",
                    "outputUrl",
                    "output_urls",
                    "outputUrls",
                    "asset_url",
                    "assetUrl",
                ] {
                    map.remove(key);
                }
                for child in map.values_mut() {
                    remove(child);
                }
            }
            Value::Array(items) => {
                for item in items {
                    remove(item);
                }
            }
            _ => {}
        }
    }
    remove(&mut value);
    value
}

#[cfg(test)]
mod tests {
    use super::{
        build_create_args, is_ambiguous_create_error, is_safe_result_url, job_matches_request,
        recent_job_ids, video_job_id_from_metadata, TempInputFiles,
    };
    use crate::models::{
        VideoGenerationOptions, VideoGenerationRequest, VideoInputMode, VideoProvider,
    };
    use chrono::{TimeZone, Utc};
    use reqwest::Url;
    use serde_json::json;

    fn request(model: &str) -> VideoGenerationRequest {
        VideoGenerationRequest {
            task_id: None,
            provider: VideoProvider::Seedance,
            model: model.to_string(),
            prompt: "A cinematic walk".to_string(),
            prompt_snapshot: None,
            input_mode: VideoInputMode::Text,
            starting_image: None,
            ending_image: None,
            reference_images: None,
            options: VideoGenerationOptions {
                duration: 5,
                aspect_ratio: "16:9".to_string(),
                resolution: "720p".to_string(),
                generate_audio: Some(true),
            },
        }
    }

    #[test]
    fn maps_fast_model_to_seedance_mode() {
        let args = build_create_args(
            &request("doubao-seedance-2-0-fast-260128"),
            &TempInputFiles::default(),
        )
        .unwrap();
        assert_eq!(args[2], "seedance_2_0");
        assert!(args.windows(2).any(|pair| pair == ["--mode", "fast"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--bitrate_mode", "standard"]));
    }

    #[test]
    fn maps_mini_model_to_mini_job_type() {
        let args = build_create_args(
            &request("doubao-seedance-2-0-mini-260615"),
            &TempInputFiles::default(),
        )
        .unwrap();
        assert_eq!(args[2], "seedance_2_0_mini");
        assert!(!args.contains(&"--mode".to_string()));
    }

    #[test]
    fn rejects_local_result_urls() {
        assert!(!is_safe_result_url(
            &Url::parse("https://127.0.0.1/video.mp4").unwrap()
        ));
        assert!(is_safe_result_url(
            &Url::parse("https://example.cloudfront.net/video.mp4").unwrap()
        ));
    }

    #[test]
    fn parses_cli_video_job_id_array() {
        assert_eq!(
            video_job_id_from_metadata(&json!(["8b0fcef3-dd9a-40fb-a564-40ac97ebbd03"])),
            Some("8b0fcef3-dd9a-40fb-a564-40ac97ebbd03".to_string())
        );
        assert_eq!(
            video_job_id_from_metadata(&json!(["first", "second"])),
            None
        );
    }

    #[test]
    fn recognizes_ambiguous_create_transport_errors() {
        assert!(is_ambiguous_create_error(
            "Higgsfield CLI error: Error: request failed (no response received)"
        ));
        assert!(is_ambiguous_create_error(
            "Higgsfield CLI command timed out."
        ));
        assert!(!is_ambiguous_create_error(
            "Higgsfield CLI error: invalid aspect ratio"
        ));
    }

    #[test]
    fn selects_only_recent_jobs_with_the_expected_type() {
        let started_at = Utc.with_ymd_and_hms(2026, 8, 2, 1, 16, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 8, 2, 1, 16, 10).unwrap();
        let ids = recent_job_ids(
            &json!([
                {
                    "id": "11111111-1111-4111-8111-111111111111",
                    "job_type": "seedance_2_0",
                    "created_at": "2026-08-02T01:10:00Z"
                },
                {
                    "id": "22222222-2222-4222-8222-222222222222",
                    "job_type": "nano_banana_pro",
                    "created_at": "2026-08-02T01:16:05Z"
                },
                {
                    "id": "33333333-3333-4333-8333-333333333333",
                    "job_type": "seedance_2_0",
                    "created_at": "2026-08-02T01:16:05Z"
                }
            ]),
            "seedance_2_0",
            started_at,
            now,
        );
        assert_eq!(ids, ["33333333-3333-4333-8333-333333333333"]);
    }

    #[test]
    fn matches_recovered_job_by_full_request_signature() {
        let request = request("doubao-seedance-2-0-260128");
        let mut detail = json!({
            "job_type": "seedance_2_0",
            "params": {
                "prompt": "A cinematic walk",
                "aspect_ratio": "16:9",
                "duration": 5,
                "resolution": "720p",
                "generate_audio": true,
                "mode": "std",
                "medias": []
            }
        });
        assert!(job_matches_request(
            &detail,
            &request,
            "seedance_2_0",
            Some("std")
        ));
        detail["params"]["prompt"] = json!("A different request");
        assert!(!job_matches_request(
            &detail,
            &request,
            "seedance_2_0",
            Some("std")
        ));
    }
}
