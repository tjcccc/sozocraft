use crate::{local_config, models::GenerationRequest};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Stdio, time::Duration};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiggsfieldStatus {
    pub cli_path: String,
    pub installed: bool,
    pub authenticated: bool,
    pub version: Option<String>,
    pub account: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HiggsfieldGeneratedImage {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HiggsfieldImageResponse {
    pub images: Vec<HiggsfieldGeneratedImage>,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
struct CliOutput {
    stdout: String,
    stderr: String,
}

#[derive(Debug, Default)]
struct TempReferenceFiles {
    directory: Option<PathBuf>,
    paths: Vec<PathBuf>,
}

impl TempReferenceFiles {
    fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

impl Drop for TempReferenceFiles {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = fs::remove_file(path);
        }
        if let Some(directory) = &self.directory {
            let _ = fs::remove_dir(directory);
        }
    }
}

const HIGGSFIELD_NANO_ASPECT_RATIOS: &[&str] = &[
    "auto", "1:1", "3:2", "2:3", "4:3", "3:4", "4:5", "5:4", "9:16", "16:9", "21:9",
];
const HIGGSFIELD_GPT_IMAGE_ASPECT_RATIOS: &[&str] =
    &["1:1", "4:3", "3:4", "16:9", "9:16", "3:2", "2:3"];
const HIGGSFIELD_GROK_ASPECT_RATIOS: &[&str] = &["1:1", "4:3", "3:4", "16:9", "9:16"];
const HIGGSFIELD_CREATE_ATTEMPTS: u32 = 3;
const HIGGSFIELD_DOWNLOAD_ATTEMPTS: u32 = 3;

pub async fn check_status(cli_path: Option<String>, proxy_url: Option<String>) -> HiggsfieldStatus {
    let cli_path = normalized_cli_path(cli_path);
    let proxy_url = clean_option(proxy_url.as_deref()).map(str::to_string);
    let version = match run_cli(
        &cli_path,
        &["version"],
        Duration::from_secs(15),
        proxy_url.as_deref(),
    )
    .await
    {
        Ok(output) => {
            first_non_empty_line(&output.stdout).or_else(|| first_non_empty_line(&output.stderr))
        }
        Err(err) => {
            return HiggsfieldStatus {
                cli_path,
                installed: false,
                authenticated: false,
                version: None,
                account: None,
                error: Some(err),
            };
        }
    };

    let account_status = match run_cli(
        &cli_path,
        &["account", "status", "--json", "--no-color"],
        Duration::from_secs(20),
        proxy_url.as_deref(),
    )
    .await
    {
        Ok(output) => Ok(output),
        Err(_) => status_fallback(&cli_path, proxy_url.as_deref()).await,
    };

    match account_status {
        Ok(output) => HiggsfieldStatus {
            cli_path,
            installed: true,
            authenticated: true,
            version,
            account: status_account_summary(&output.stdout),
            error: None,
        },
        Err(err) => HiggsfieldStatus {
            cli_path,
            installed: true,
            authenticated: false,
            version,
            account: None,
            error: Some(err),
        },
    }
}

async fn status_fallback(cli_path: &str, proxy_url: Option<&str>) -> Result<CliOutput, String> {
    run_cli(
        cli_path,
        &["auth", "status", "--json", "--no-color"],
        Duration::from_secs(20),
        proxy_url,
    )
    .await
}

pub async fn generate_image(
    request: &GenerationRequest,
    cli_path: Option<String>,
    timeout_seconds: u64,
    proxy_url: Option<String>,
) -> Result<HiggsfieldImageResponse, String> {
    if request.prompt.trim().is_empty() {
        return Err("Higgsfield CLI requires a prompt.".to_string());
    }
    validate_request_options(request)?;

    let cli_path = normalized_cli_path(cli_path);
    let proxy_url = clean_option(proxy_url.as_deref()).map(str::to_string);
    let unlimited_requested = request.options.unlimited.unwrap_or(false);

    let reference_files = materialize_reference_images(request)?;
    let args = build_generate_args(request, reference_files.paths(), timeout_seconds, false);
    let result = run_generate_create_with_retries(
        &cli_path,
        &args,
        Duration::from_secs(timeout_seconds.max(30) + 60),
        proxy_url.as_deref(),
    )
    .await;

    let output = result?;
    let parsed_metadata = parse_json_output(&output.stdout);
    let parsed_json_output = parsed_metadata.is_some();
    let mut metadata = parsed_metadata.unwrap_or_else(|| {
        json!({
            "stdout": output.stdout,
            "stderr": output.stderr,
        })
    });
    let mut urls = collect_result_urls(&metadata);
    if urls.is_empty() {
        if let Some(job_id) = job_id_from_metadata(&metadata) {
            match run_cli(
                &cli_path,
                &["generate", "get", &job_id, "--json", "--no-color"],
                Duration::from_secs(30),
                proxy_url.as_deref(),
            )
            .await
            {
                Ok(output) => {
                    if let Some(job_metadata) = parse_json_output(&output.stdout) {
                        urls = collect_result_urls(&job_metadata);
                        metadata = json!({
                            "jobId": job_id,
                            "create": metadata,
                            "job": job_metadata,
                        });
                    }
                }
                Err(err) => {
                    metadata = json!({
                        "jobId": job_id,
                        "create": metadata,
                        "jobFetchError": err,
                    });
                }
            }
        }
    }
    if urls.is_empty() {
        urls = collect_fallback_urls_from_text(&output.stdout, parsed_json_output);
    }
    let urls = dedupe_urls(urls);
    if urls.is_empty() {
        return Err(no_result_url_error(&metadata));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds.max(30)))
        .connect_timeout(Duration::from_secs(30))
        .http1_only()
        .user_agent("SozoCraft Higgsfield downloader")
        .build()
        .map_err(|err| format!("Failed to create Higgsfield result downloader: {err}"))?;
    let mut images = Vec::new();
    let mut download_errors = Vec::new();
    for url in &urls {
        match download_url(&client, url).await {
            Ok(bytes) => images.push(HiggsfieldGeneratedImage { bytes }),
            Err(err) => download_errors.push(format!("{url}: {err}")),
        }
    }

    if images.is_empty() {
        return Err(format!(
            "Failed to download Higgsfield result image. {}",
            download_errors.join("; ")
        ));
    }

    Ok(HiggsfieldImageResponse {
        images,
        metadata: json!({
            "platform": "higgsfield",
            "cliPath": cli_path,
            "model": request.model,
            "proxyApplied": proxy_url.is_some(),
            "unlimitedRequested": unlimited_requested,
            "unlimitedApplied": false,
            "resultUrls": urls,
            "raw": metadata,
        }),
    })
}

fn build_generate_args(
    request: &GenerationRequest,
    reference_files: &[PathBuf],
    timeout_seconds: u64,
    unlimited: bool,
) -> Vec<String> {
    let mut args = vec![
        "generate".to_string(),
        "create".to_string(),
        request.model.clone(),
        "--prompt".to_string(),
        cli_string_value(request.prompt.trim()),
    ];

    if let Some(aspect_ratio) = clean_option(request.options.aspect_ratio.as_deref()) {
        if !aspect_ratio.eq_ignore_ascii_case("auto") {
            args.extend(["--aspect_ratio".to_string(), aspect_ratio.to_string()]);
        }
    }

    if let Some(resolution) = clean_option(request.options.image_size.as_deref()) {
        if model_accepts_resolution(&request.model) {
            args.extend(["--resolution".to_string(), resolution.to_ascii_lowercase()]);
        }
    }

    if request.model == "gpt_image_2" {
        if let Some(quality) = clean_option(request.options.quality.as_deref()) {
            if ["low", "medium", "high"].contains(&quality) {
                args.extend(["--quality".to_string(), quality.to_string()]);
            }
        }
        if request.batch_count > 1 {
            args.extend(["--batch_size".to_string(), request.batch_count.to_string()]);
        }
    }

    if request.model == "grok_image" {
        if let Some(mode) = clean_option(request.options.quality.as_deref()) {
            if ["std", "pro"].contains(&mode) {
                args.extend(["--mode".to_string(), mode.to_string()]);
            }
        }
    }

    if unlimited {
        args.push("--unlimited".to_string());
    }

    for path in reference_files {
        args.extend(["--image".to_string(), path.to_string_lossy().to_string()]);
    }

    args.extend([
        "--wait".to_string(),
        "--wait-timeout".to_string(),
        format!("{}s", timeout_seconds.max(30)),
        "--json".to_string(),
        "--no-color".to_string(),
    ]);
    args
}

fn validate_request_options(request: &GenerationRequest) -> Result<(), String> {
    let Some(aspect_ratio) = clean_option(request.options.aspect_ratio.as_deref()) else {
        return Ok(());
    };
    let Some(supported) = supported_cli_aspect_ratios(&request.model) else {
        return Ok(());
    };
    if supported
        .iter()
        .any(|supported_ratio| supported_ratio.eq_ignore_ascii_case(aspect_ratio))
    {
        return Ok(());
    }

    Err(format!(
        "Higgsfield CLI does not support aspect ratio `{}` for {} yet. Use one of: {}.",
        aspect_ratio,
        higgsfield_model_label(&request.model),
        supported.join(", ")
    ))
}

fn supported_cli_aspect_ratios(model: &str) -> Option<&'static [&'static str]> {
    match model {
        "nano_banana_2" | "nano_banana_flash" | "nano_banana" => {
            Some(HIGGSFIELD_NANO_ASPECT_RATIOS)
        }
        "gpt_image_2" => Some(HIGGSFIELD_GPT_IMAGE_ASPECT_RATIOS),
        "grok_image" => Some(HIGGSFIELD_GROK_ASPECT_RATIOS),
        _ => None,
    }
}

fn higgsfield_model_label(model: &str) -> &'static str {
    match model {
        "nano_banana_2" => "Nano Banana Pro",
        "nano_banana_flash" => "Nano Banana 2",
        "nano_banana" => "Nano Banana",
        "gpt_image_2" => "GPT Image 2",
        "grok_image" => "Grok Imagine",
        _ => "this model",
    }
}

fn model_accepts_resolution(model: &str) -> bool {
    matches!(model, "gpt_image_2" | "nano_banana_2" | "nano_banana_flash")
}

fn clean_option(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn cli_string_value(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

async fn run_generate_create_with_retries(
    cli_path: &str,
    args: &[String],
    timeout: Duration,
    proxy_url: Option<&str>,
) -> Result<CliOutput, String> {
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let mut errors = Vec::new();
    for attempt in 1..=HIGGSFIELD_CREATE_ATTEMPTS {
        match run_cli(cli_path, &arg_refs, timeout, proxy_url).await {
            Ok(output) => return Ok(output),
            Err(err) => {
                let retryable = is_retryable_cli_create_error(&err);
                errors.push(format!("attempt {attempt}: {err}"));
                if !retryable || attempt == HIGGSFIELD_CREATE_ATTEMPTS {
                    return if errors.len() == 1 {
                        Err(errors.remove(0).replace("attempt 1: ", ""))
                    } else {
                        Err(format!(
                            "Higgsfield CLI failed after {attempt} attempts. {}",
                            errors.join("; ")
                        ))
                    };
                }
                tokio::time::sleep(Duration::from_secs(2 * u64::from(attempt))).await;
            }
        }
    }

    Err("Higgsfield CLI command failed.".to_string())
}

fn is_retryable_cli_create_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("cannot reach https://") && error.contains("cloudfront.net")
}

async fn run_cli(
    cli_path: &str,
    args: &[&str],
    timeout: Duration,
    proxy_url: Option<&str>,
) -> Result<CliOutput, String> {
    validate_cli_path(cli_path)?;
    let mut command = Command::new(cli_path);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(proxy_url) = clean_option(proxy_url) {
        command
            .env("HTTP_PROXY", proxy_url)
            .env("HTTPS_PROXY", proxy_url)
            .env("ALL_PROXY", proxy_url)
            .env("http_proxy", proxy_url)
            .env("https_proxy", proxy_url)
            .env("all_proxy", proxy_url);
    }
    let child = command.spawn().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!("Higgsfield CLI not found at `{cli_path}`.")
        } else {
            format!("Failed to start Higgsfield CLI: {err}")
        }
    })?;

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| "Higgsfield CLI command timed out.".to_string())?
        .map_err(|err| format!("Higgsfield CLI command failed: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(clean_cli_error(&stdout, &stderr));
    }

    Ok(CliOutput { stdout, stderr })
}

fn clean_cli_error(stdout: &str, stderr: &str) -> String {
    let message = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or("Higgsfield CLI command failed.");
    format!("Higgsfield CLI error: {message}")
}

fn materialize_reference_images(request: &GenerationRequest) -> Result<TempReferenceFiles, String> {
    let Some(reference_images) = &request.reference_images else {
        return Ok(TempReferenceFiles::default());
    };
    if reference_images.is_empty() {
        return Ok(TempReferenceFiles::default());
    }

    let directory = local_config::config_dir()
        .join("cache")
        .join("higgsfield-inputs")
        .join(Uuid::new_v4().to_string());
    fs::create_dir_all(&directory)
        .map_err(|err| format!("Failed to create Higgsfield input cache: {err}"))?;

    let mut temp_files = TempReferenceFiles {
        directory: Some(directory),
        paths: Vec::new(),
    };
    for (index, image) in reference_images.iter().enumerate() {
        let extension = extension_for_mime(&image.mime_type);
        let directory = temp_files
            .directory
            .as_ref()
            .ok_or_else(|| "Higgsfield input cache is unavailable.".to_string())?;
        let path = directory.join(format!("{:03}.{extension}", index + 1));
        let bytes = general_purpose::STANDARD
            .decode(image.data.trim())
            .map_err(|err| format!("Failed to decode reference image {}: {err}", image.name))?;
        fs::write(&path, bytes)
            .map_err(|err| format!("Failed to write Higgsfield reference image: {err}"))?;
        temp_files.paths.push(path);
    }
    Ok(temp_files)
}

fn extension_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    }
}

async fn download_url(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let mut errors = Vec::new();
    for attempt in 1..=HIGGSFIELD_DOWNLOAD_ATTEMPTS {
        match download_url_once(client, url).await {
            Ok(bytes) => return Ok(bytes),
            Err(err) => {
                errors.push(format!("attempt {attempt}: {err}"));
                if attempt < HIGGSFIELD_DOWNLOAD_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
                }
            }
        }
    }

    match download_url_with_curl(url, Duration::from_secs(180)).await {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            errors.push(format!("curl fallback: {err}"));
            Err(errors.join("; "))
        }
    }
}

async fn download_url_once(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("request failed: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|err| format!("invalid response body: {err}"))
}

async fn download_url_with_curl(url: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    let child = Command::new("curl")
        .args([
            "--location",
            "--fail",
            "--silent",
            "--show-error",
            "--retry",
            "2",
            "--retry-delay",
            "1",
            "--max-time",
            &timeout.as_secs().to_string(),
            "--output",
            "-",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| format!("failed to start curl fallback: {err}"))?;

    let output = tokio::time::timeout(timeout + Duration::from_secs(5), child.wait_with_output())
        .await
        .map_err(|_| "curl fallback timed out".to_string())?
        .map_err(|err| format!("curl fallback failed: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "curl exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    if output.stdout.is_empty() {
        return Err("curl returned an empty body".to_string());
    }
    Ok(output.stdout)
}

fn parse_json_output(stdout: &str) -> Option<Value> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok().or_else(|| {
        trimmed
            .lines()
            .rev()
            .filter_map(|line| serde_json::from_str(line.trim()).ok())
            .next()
    })
}

fn collect_result_urls(value: &Value) -> Vec<String> {
    let mut urls = Vec::new();
    collect_result_urls_inner(value, None, &mut urls);
    urls
}

fn job_id_from_metadata(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => ["id", "job_id", "jobId"]
            .iter()
            .find_map(|key| map.get(*key).and_then(Value::as_str))
            .map(str::to_string),
        Value::Array(items) => items.iter().find_map(job_id_from_metadata),
        _ => None,
    }
}

fn collect_result_urls_inner(value: &Value, key: Option<&str>, urls: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if key.map(is_result_url_key).unwrap_or(false) && is_http_url(text) {
                urls.push(text.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_result_urls_inner(item, key, urls);
            }
        }
        Value::Object(map) => {
            for (next_key, next_value) in map {
                collect_result_urls_inner(next_value, Some(next_key), urls);
            }
        }
        _ => {}
    }
}

fn is_result_url_key(key: &str) -> bool {
    matches!(
        key,
        "result_url"
            | "resultUrl"
            | "result_urls"
            | "resultUrls"
            | "output_url"
            | "outputUrl"
            | "output_urls"
            | "outputUrls"
            | "asset_url"
            | "assetUrl"
    )
}

fn collect_urls_from_text(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|value| {
            value.trim_matches(|ch: char| matches!(ch, '"' | '\'' | ',' | ')' | ']' | '}'))
        })
        .filter(|value| is_http_url(value))
        .map(str::to_string)
        .collect()
}

fn collect_fallback_urls_from_text(text: &str, parsed_json_output: bool) -> Vec<String> {
    if parsed_json_output {
        return Vec::new();
    }
    collect_urls_from_text(text)
}

fn dedupe_urls(urls: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for url in urls {
        if !deduped.contains(&url) {
            deduped.push(url);
        }
    }
    deduped
}

fn no_result_url_error(metadata: &Value) -> String {
    let mut details = Vec::new();
    if let Some(job_id) = job_id_from_metadata(metadata) {
        details.push(format!("job id: {job_id}"));
    }
    if let Some(status) = find_scalar_key(metadata, &["status", "state"]) {
        details.push(format!("status: {status}"));
    }
    if let Some(code) = find_scalar_key(
        metadata,
        &[
            "status_code",
            "statusCode",
            "code",
            "error_code",
            "errorCode",
            "finish_reason",
            "finishReason",
        ],
    ) {
        details.push(format!("code: {code}"));
    }
    if let Some(reason) = find_scalar_key(
        metadata,
        &[
            "message",
            "error",
            "reason",
            "failure_reason",
            "failureReason",
            "detail",
            "jobFetchError",
        ],
    ) {
        details.push(format!("message: {}", truncate_error_detail(&reason)));
    }

    if details.is_empty() {
        "Higgsfield CLI returned no result URL.".to_string()
    } else {
        format!(
            "Higgsfield CLI returned no result URL ({}).",
            details.join("; ")
        )
    }
}

fn find_scalar_key(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key).and_then(scalar_to_string) {
                    if !value.trim().is_empty() {
                        return Some(value.trim().to_string());
                    }
                }
            }
            map.values().find_map(|item| find_scalar_key(item, keys))
        }
        Value::Array(items) => items.iter().find_map(|item| find_scalar_key(item, keys)),
        _ => None,
    }
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn truncate_error_detail(value: &str) -> String {
    const MAX_DETAIL_LENGTH: usize = 360;
    if value.chars().count() <= MAX_DETAIL_LENGTH {
        return value.to_string();
    }
    let mut truncated = value
        .chars()
        .take(MAX_DETAIL_LENGTH.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn normalized_cli_path(cli_path: Option<String>) -> String {
    cli_path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "higgsfield".to_string())
}

fn validate_cli_path(cli_path: &str) -> Result<(), String> {
    let file_name = PathBuf::from(cli_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(cli_path)
        .to_ascii_lowercase();
    if file_name == "higgsfield" || file_name.starts_with("higgsfield.") {
        return Ok(());
    }
    Err("Higgsfield CLI path must point to a `higgsfield` executable.".to_string())
}

fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn status_account_summary(stdout: &str) -> Option<String> {
    let json = parse_json_output(stdout)?;
    if let Some(plan) = find_string_key(&json, &["subscription_plan_type", "plan"]) {
        return Some(format!("{} plan", plan));
    }
    if let Some(account) = find_string_key(&json, &["username", "name", "email"]) {
        return Some(account);
    }
    Some("Authenticated".to_string())
}

fn find_string_key(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(text) = map.get(*key).and_then(Value::as_str) {
                    if !text.trim().is_empty() {
                        return Some(text.trim().to_string());
                    }
                }
            }
            map.values().find_map(|item| find_string_key(item, keys))
        }
        Value::Array(items) => items.iter().find_map(|item| find_string_key(item, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_generate_args, collect_fallback_urls_from_text, collect_result_urls,
        is_retryable_cli_create_error, job_id_from_metadata, no_result_url_error,
        validate_cli_path, validate_request_options,
    };
    use crate::models::{GenerationOptions, GenerationRequest};
    use serde_json::json;

    #[test]
    fn gpt_image_args_include_batch_size_and_quality() {
        let request = request("gpt-image", "gpt_image_2", 3);
        let args = build_generate_args(&request, &[], 180, false);

        assert!(contains_pair(&args, "--aspect_ratio", "2:3"));
        assert!(contains_pair(&args, "--resolution", "2k"));
        assert!(contains_pair(&args, "--quality", "high"));
        assert!(contains_pair(&args, "--batch_size", "3"));
        assert!(args.contains(&"--wait".to_string()));
        assert!(args.contains(&"--json".to_string()));
    }

    #[test]
    fn grok_args_map_quality_to_mode() {
        let mut request = request("grok-imagine", "grok_image", 1);
        request.options.quality = Some("pro".to_string());
        let args = build_generate_args(&request, &[], 180, false);

        assert!(contains_pair(&args, "--mode", "pro"));
        assert!(!args.contains(&"--resolution".to_string()));
    }

    #[test]
    fn auto_aspect_ratio_is_omitted_from_cli_args() {
        let mut request = request("gpt-image", "gpt_image_2", 1);
        request.options.aspect_ratio = Some("auto".to_string());
        let args = build_generate_args(&request, &[], 180, false);

        assert!(!args.contains(&"--aspect_ratio".to_string()));
    }

    #[test]
    fn json_like_prompt_is_encoded_as_cli_string_value() {
        let mut request = request("nano-banana", "nano_banana_flash", 1);
        request.prompt = "{\"prompt\":\"keep this as text\"}".to_string();
        let args = build_generate_args(&request, &[], 180, false);
        let prompt_value = flag_value(&args, "--prompt").expect("prompt flag");

        let decoded = serde_json::from_str::<String>(prompt_value).expect("JSON string prompt");
        assert_eq!(decoded, request.prompt);
    }

    #[test]
    fn unsupported_cli_aspect_ratio_reports_model_limit() {
        let mut request = request("grok-imagine", "grok_image", 1);
        request.options.aspect_ratio = Some("2:1".to_string());
        let err = validate_request_options(&request).unwrap_err();

        assert!(err.contains("Grok Imagine"));
        assert!(err.contains("2:1"));
        assert!(err.contains("1:1"));
    }

    #[test]
    fn cloudfront_cannot_reach_errors_are_retryable() {
        assert!(is_retryable_cli_create_error(
            "Higgsfield CLI error: Error: Cannot reach https://d276s3zg8h21b2.cloudfront.net/input.png"
        ));
        assert!(!is_retryable_cli_create_error(
            "Higgsfield CLI error: Error: invalid aspect ratio"
        ));
    }

    #[test]
    fn cli_path_validation_rejects_unrelated_executables() {
        assert!(validate_cli_path("higgsfield").is_ok());
        assert!(validate_cli_path("/opt/homebrew/bin/higgsfield").is_ok());
        assert!(validate_cli_path("/bin/sh").is_err());
    }

    #[test]
    fn no_result_error_includes_job_status_code_and_message() {
        let message = no_result_url_error(&json!({
            "jobId": "job1",
            "job": {
                "status": "failed",
                "error_code": "CONTENT_POLICY",
                "message": "The request was rejected by policy."
            }
        }));

        assert!(message.contains("job id: job1"));
        assert!(message.contains("status: failed"));
        assert!(message.contains("code: CONTENT_POLICY"));
        assert!(message.contains("rejected by policy"));
    }

    #[test]
    fn result_url_extraction_prefers_url_like_keys() {
        let urls = collect_result_urls(&json!([
            { "id": "job1", "result_url": "https://cdn.example.com/a.png" },
            { "image": { "url": "https://cdn.example.com/b.png" } }
        ]));

        assert_eq!(urls, vec!["https://cdn.example.com/a.png".to_string()]);
    }

    #[test]
    fn result_url_extraction_skips_reference_media_urls() {
        let urls = collect_result_urls(&json!({
            "id": "job1",
            "result_url": "https://cdn.example.com/result.png",
            "params": {
                "medias": [
                    {
                        "data": {
                            "url": "https://cdn.example.com/reference.png",
                            "type": "media_input"
                        },
                        "role": "image"
                    }
                ]
            }
        }));

        assert_eq!(urls, vec!["https://cdn.example.com/result.png".to_string()]);
    }

    #[test]
    fn parsed_json_output_does_not_fallback_to_reference_urls() {
        let stdout = serde_json::to_string_pretty(&json!({
            "id": "job1",
            "status": "nsfw",
            "params": {
                "input_images": [
                    {
                        "url": "https://cdn.example.com/reference.png",
                        "type": "media_input"
                    }
                ]
            }
        }))
        .expect("pretty JSON");

        assert!(collect_fallback_urls_from_text(&stdout, true).is_empty());
        assert_eq!(
            collect_fallback_urls_from_text(&stdout, false),
            vec!["https://cdn.example.com/reference.png".to_string()]
        );
    }

    #[test]
    fn job_id_reads_top_level_id_only() {
        let id = job_id_from_metadata(&json!({
            "id": "job1",
            "params": {
                "medias": [
                    { "data": { "id": "input1" } }
                ]
            }
        }));

        assert_eq!(id.as_deref(), Some("job1"));
    }

    fn request(provider: &str, model: &str, batch_count: u32) -> GenerationRequest {
        GenerationRequest {
            task_id: None,
            provider: provider.to_string(),
            model: model.to_string(),
            prompt: "test prompt".to_string(),
            prompt_snapshot: None,
            batch_count,
            reference_images: None,
            output_template: "{id}.{extension}".to_string(),
            options: GenerationOptions {
                aspect_ratio: Some("2:3".to_string()),
                image_size: Some("2k".to_string()),
                temperature: None,
                top_p: None,
                thinking_level: None,
                quality: Some("high".to_string()),
                unlimited: None,
            },
            base_url: None,
        }
    }

    fn contains_pair(args: &[String], flag: &str, value: &str) -> bool {
        args.windows(2)
            .any(|window| window[0] == flag && window[1] == value)
    }

    fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.windows(2)
            .find(|window| window[0] == flag)
            .map(|window| window[1].as_str())
    }
}
