use super::{
    arguments::{validate_operation, Command, Generate},
    emit,
};
use crate::{
    file_access,
    filename_template::resolve_output_path,
    google_omni::{GoogleOmniClient, OmniStatus},
    google_veo::{GoogleVeoClient, GoogleVeoStatus},
    local_config,
    models::{
        AppSettings, ReferenceImageInput, VideoGenerationOptions, VideoGenerationRequest,
        VideoInputMode, VideoProvider,
    },
};
use chrono::Local;
use serde_json::json;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::time::Instant;
use uuid::Uuid;

enum Client {
    Veo(GoogleVeoClient),
    Omni(GoogleOmniClient),
}

enum Status {
    Pending,
    Completed(String),
}

impl Client {
    fn new(model: &str, key: &str, settings: &AppSettings) -> Result<Self, String> {
        let proxy = settings
            .gemini_proxy_enabled
            .then(|| settings.proxy_url.clone())
            .flatten();
        if model == "gemini-omni-1.1-flash" {
            Ok(Self::Omni(GoogleOmniClient::new(
                key.to_string(),
                settings.optional_base_url.clone(),
                proxy,
                settings.gemini_timeout_seconds,
            )?))
        } else {
            Ok(Self::Veo(GoogleVeoClient::new(
                key.to_string(),
                settings.optional_base_url.clone(),
                proxy,
                settings.gemini_timeout_seconds,
            )?))
        }
    }

    async fn start(&self, request: &VideoGenerationRequest) -> Result<String, String> {
        match self {
            Self::Veo(client) => client.start(request).await,
            Self::Omni(client) => client.start(request).await,
        }
    }

    async fn poll(&self, operation: &str) -> Result<Status, String> {
        match self {
            Self::Veo(client) => match client.poll(operation).await?.status {
                GoogleVeoStatus::Pending => Ok(Status::Pending),
                GoogleVeoStatus::Done { url } => Ok(Status::Completed(url)),
                GoogleVeoStatus::Failed(error) => Err(error),
            },
            Self::Omni(client) => match client.poll(operation).await? {
                OmniStatus::Pending => Ok(Status::Pending),
                OmniStatus::Done(url) => Ok(Status::Completed(url)),
                OmniStatus::Failed => Err("Gemini Omni video processing failed.".to_string()),
            },
        }
    }

    async fn download(&self, url: &str, path: &Path) -> Result<u64, String> {
        match self {
            Self::Veo(client) => client.download_to(url, path).await,
            Self::Omni(client) => client.download_to(url, path).await,
        }
    }
}

pub(super) async fn execute(command: Command) -> Result<(), String> {
    // Validate local inputs before credentials or any paid request. Dry runs never load secrets.
    let request = if let Command::Generate(options) = &command {
        Some(build_request(options)?)
    } else {
        None
    };
    if let (Command::Generate(options), Some(request)) = (&command, &request) {
        if options.dry_run {
            if let Some(output) = &options.output {
                validate_output(output)?;
            }
            return emit(&json!({
                "status": "validated", "model": request.model, "inputMode": request.input_mode,
                "options": request.options, "inputImageCount": request.reference_images.as_ref().map_or(0, Vec::len)
                    + usize::from(request.starting_image.is_some()) + usize::from(request.ending_image.is_some()),
            }));
        }
    }
    let key = local_config::get_gemini_api_key().map_err(|_| "Could not load Gemini API key from ~/.sozocraft/config.toml. Configure it in SozoCraft and check that the file is valid.".to_string())?;
    let settings = local_config::load_settings(AppSettings::default());
    execute_with_settings(command, request, &key, &settings)
        .await
        .map_err(|error| redact(&error, &key))
}

async fn execute_with_settings(
    command: Command,
    request: Option<VideoGenerationRequest>,
    key: &str,
    settings: &AppSettings,
) -> Result<(), String> {
    match command {
        Command::Generate(options) => {
            let request = request.ok_or("Missing validated video request.")?;
            let client = Client::new(&options.model, key, settings)?;
            // Check local write access before starting a generation. No-wait only submits.
            let output = if options.no_wait {
                None
            } else {
                Some(Output::prepare(options.output, settings, &options.model)?)
            };
            let operation = client.start(&request).await?;
            validate_operation(&options.model, &operation)?;
            emit(&json!({"status": "submitted", "model": options.model, "operation": operation}))?;
            if let Some(output) = output {
                wait(
                    &client,
                    &options.model,
                    &operation,
                    output,
                    options.max_wait,
                )
                .await?;
            }
            Ok(())
        }
        Command::Status { operation, model } => {
            let client = Client::new(&model, key, settings)?;
            let status = match client.poll(&operation).await? {
                Status::Pending => "pending",
                Status::Completed(_) => "completed",
            };
            emit(&json!({"status": status, "model": model, "operation": operation}))
        }
        Command::Wait {
            operation,
            model,
            output,
            max_wait,
        } => {
            let client = Client::new(&model, key, settings)?;
            wait(
                &client,
                &model,
                &operation,
                Output::prepare(output, settings, &model)?,
                max_wait,
            )
            .await
        }
        Command::Help => Ok(()),
    }
}

async fn wait(
    client: &Client,
    model: &str,
    operation: &str,
    output: Output,
    max_wait: u64,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(max_wait);
    loop {
        let status = tokio::time::timeout_at(deadline, client.poll(operation)).await
            .map_err(|_| "Local monitoring timed out. Resume with video wait using the same model and operation; do not submit again.".to_string())??;
        match status {
            Status::Pending => {
                tokio::time::sleep_until((Instant::now() + Duration::from_secs(5)).min(deadline))
                    .await
            }
            Status::Completed(url) => {
                let bytes = client.download(&url, &output.partial).await?;
                output.publish()?;
                return emit(
                    &json!({"status": "completed", "model": model, "operation": operation, "outputPath": output.final_path, "bytes": bytes}),
                );
            }
        }
    }
}

fn build_request(options: &Generate) -> Result<VideoGenerationRequest, String> {
    let prompt = match (&options.prompt, &options.prompt_file) {
        (Some(prompt), None) => prompt.clone(),
        (None, Some(path)) if path == "-" => read_prompt(std::io::stdin().lock())?,
        (None, Some(path)) => file_access::read_text_file(path)?,
        _ => return Err("Supply exactly one prompt source.".to_string()),
    };
    if options.end.is_some() && options.start.is_none() {
        return Err("--end-image requires --start-image.".to_string());
    }
    if options.model != "gemini-omni-1.1-flash"
        && options.start.is_some()
        && !options.references.is_empty()
    {
        return Err("Frame images cannot be combined with --reference.".to_string());
    }
    let input_mode = if options.end.is_some() {
        VideoInputMode::Frames
    } else if options.start.is_some() {
        VideoInputMode::Image
    } else if !options.references.is_empty() {
        VideoInputMode::Reference
    } else {
        VideoInputMode::Text
    };
    let request = VideoGenerationRequest {
        task_id: None,
        provider: VideoProvider::GoogleVeo,
        model: options.model.clone(),
        prompt,
        prompt_snapshot: None,
        input_mode,
        starting_image: options.start.as_deref().map(read_image).transpose()?,
        ending_image: options.end.as_deref().map(read_image).transpose()?,
        reference_images: Some(
            options
                .references
                .iter()
                .map(|path| read_image(path))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        options: VideoGenerationOptions {
            duration: options.duration,
            aspect_ratio: options.aspect_ratio.clone(),
            resolution: options.resolution.clone(),
            generate_audio: None,
        },
    };
    request.validate()?;
    Ok(request)
}

fn read_prompt(reader: impl Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    reader
        .take(50_001)
        .read_to_end(&mut bytes)
        .map_err(|_| "Failed to read prompt from stdin.")?;
    if bytes.len() > 50_000 {
        return Err("Video prompt is too large.".to_string());
    }
    String::from_utf8(bytes).map_err(|_| "Prompt must be UTF-8.".to_string())
}

fn read_image(path: &str) -> Result<ReferenceImageInput, String> {
    let value = file_access::read_image_data_url(path)?;
    let (header, data) = value
        .split_once(";base64,")
        .ok_or("Invalid image encoding.")?;
    Ok(ReferenceImageInput {
        name: Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("Invalid image filename.")?
            .to_string(),
        mime_type: header.trim_start_matches("data:").to_string(),
        data: data.to_string(),
        asset_id: None,
    })
}

fn redact(error: &str, key: &str) -> String {
    error.replace(key, "[redacted]")
}

fn validate_output(path: &Path) -> Result<(), String> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("mp4"))
    {
        return Err("Output must be a new .mp4 file.".to_string());
    }
    match fs::symlink_metadata(path) {
        Ok(_) => Err("Output already exists; choose a new file.".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Cannot inspect output path.".to_string()),
    }
}

struct Output {
    final_path: PathBuf,
    partial: PathBuf,
}

impl Output {
    fn prepare(path: Option<PathBuf>, settings: &AppSettings, model: &str) -> Result<Self, String> {
        let path = match path {
            Some(path) => path,
            None => {
                let batch_id = Uuid::new_v4().to_string();
                resolve_output_path(
                    &settings.output_directory,
                    &settings.output_template,
                    "google",
                    model,
                    "001",
                    &batch_id[..6],
                    "mp4",
                    Local::now(),
                )
                .map_err(|error| error.to_string())?
            }
        };
        validate_output(&path)?;
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|_| "Cannot determine working directory.")?
                .join(path)
        };
        let parent = path.parent().ok_or("Invalid output directory.")?;
        fs::create_dir_all(parent).map_err(|_| "Cannot create output directory.")?;
        let parent = parent
            .canonicalize()
            .map_err(|_| "Cannot resolve output directory.")?;
        let final_path = parent.join(path.file_name().ok_or("Invalid output filename.")?);
        let partial = parent.join(format!(".sozocraft-cli-{}.part", Uuid::new_v4()));
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|_| "Cannot write to output directory.")?;
        Ok(Self {
            final_path,
            partial,
        })
    }

    fn publish(&self) -> Result<(), String> {
        // A hard link atomically publishes the complete file without replacing any existing path.
        fs::hard_link(&self.partial, &self.final_path).map_err(|_| "Cannot publish MP4 output; the destination may already exist or the filesystem may not support hard links.".to_string())
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.partial);
    }
}

#[cfg(test)]
mod tests {
    use super::super::arguments::parse;
    use super::*;
    use std::ffi::OsString;

    fn options(values: &[&str]) -> Generate {
        let Command::Generate(value) = parse(values.iter().map(OsString::from)).unwrap() else {
            panic!("expected generate")
        };
        value
    }

    #[test]
    fn validates_model_specific_options_without_network() {
        let mut value = options(&["video", "generate", "--prompt", "Animate pixel character"]);
        assert!(build_request(&value).is_ok());
        value.duration = 5;
        assert!(build_request(&value).is_err());
        value.model = "gemini-omni-1.1-flash".to_string();
        value.resolution = "360p".to_string();
        assert!(build_request(&value).is_ok());
        value.end = Some("missing.png".to_string());
        assert!(build_request(&value)
            .unwrap_err()
            .contains("requires --start-image"));
    }

    #[test]
    fn validates_image_signatures_and_frame_modes() {
        let dir = std::env::temp_dir().join(format!("sozocraft-cli-test-{}", Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("sprite.png");
        fs::write(&path, b"not an image").unwrap();
        let mut value = options(&["video", "generate", "--prompt", "walk"]);
        value.start = Some(path.to_str().unwrap().to_string());
        assert!(build_request(&value).is_err());
        fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
        assert_eq!(
            build_request(&value).unwrap().input_mode,
            VideoInputMode::Image
        );
        value.end = value.start.clone();
        assert_eq!(
            build_request(&value).unwrap().input_mode,
            VideoInputMode::Frames
        );
        value.references.push(value.start.clone().unwrap());
        assert!(build_request(&value).is_err());
        value.model = "gemini-omni-1.1-flash".to_string();
        assert_eq!(
            build_request(&value).unwrap().input_mode,
            VideoInputMode::Frames
        );
        value.end = None;
        assert_eq!(
            build_request(&value).unwrap().input_mode,
            VideoInputMode::Image
        );
        let mut request = build_request(&value).unwrap();
        let reference = request.reference_images.as_ref().unwrap()[0].clone();
        request.reference_images = Some(vec![reference; 7]);
        assert!(request
            .validate()
            .unwrap_err()
            .contains("6 reference images"));
        request.reference_images = Some(vec![crate::models::ReferenceImageInput {
            data: "bm90IGFuIGltYWdl".to_string(),
            ..request.starting_image.clone().unwrap()
        }]);
        assert!(request.validate().is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn output_publication_never_overwrites_and_cleans_partial() {
        let dir = std::env::temp_dir().join(format!("sozocraft-cli-test-{}", Uuid::new_v4()));
        let path = dir.join("sprite.mp4");
        let output = Output::prepare(
            Some(path.clone()),
            &AppSettings::default(),
            "veo-3.1-generate-preview",
        )
        .unwrap();
        let partial = output.partial.clone();
        fs::write(&partial, b"new video").unwrap();
        fs::write(&path, b"existing video").unwrap();
        assert!(output.publish().is_err());
        drop(output);
        assert!(!partial.exists());
        assert_eq!(fs::read(&path).unwrap(), b"existing video");
        fs::remove_file(&path).unwrap();
        let output = Output::prepare(
            Some(path.clone()),
            &AppSettings::default(),
            "veo-3.1-generate-preview",
        )
        .unwrap();
        fs::write(&output.partial, b"complete video").unwrap();
        output.publish().unwrap();
        drop(output);
        assert_eq!(fs::read(path).unwrap(), b"complete video");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_oversized_stdin_and_redacts_key() {
        assert!(read_prompt(&vec![b'x'; 50_001][..]).is_err());
        assert!(read_prompt(&b"\xff"[..]).is_err());
        assert_eq!(redact("echoed secret", "secret"), "echoed [redacted]");
    }

    #[test]
    fn default_output_uses_shared_dated_template_and_google_model_name() {
        let dir = std::env::temp_dir().join(format!("sozocraft-cli-test-{}", Uuid::new_v4()));
        let settings = AppSettings {
            output_directory: dir.to_str().unwrap().to_string(),
            output_template:
                "{yyMMdd}/{datetime:yyyyMMdd_HHmmss}_{provider}_{model}_{batch_id}_{id}".to_string(),
            ..AppSettings::default()
        };
        for model in [
            "veo-3.1-generate-preview",
            "veo-3.1-lite-generate-preview",
            "gemini-omni-1.1-flash",
        ] {
            let before = Local::now();
            let output = Output::prepare(None, &settings, model).unwrap();
            let after = Local::now();
            let date = output
                .final_path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            assert!(
                date == before.format("%y%m%d").to_string()
                    || date == after.format("%y%m%d").to_string()
            );
            let name = output.final_path.file_name().unwrap().to_str().unwrap();
            assert!(name.contains(&format!("_google_{model}_")));
            assert!(name.ends_with("_001.mp4"));
            assert_eq!(
                name.split(&format!("_google_{model}_"))
                    .next()
                    .unwrap()
                    .len(),
                15
            );
            fs::write(&output.partial, b"test video").unwrap();
            output.publish().unwrap();
            assert!(Output::prepare(Some(output.final_path.clone()), &settings, model).is_err());
            assert_eq!(fs::read(&output.final_path).unwrap(), b"test video");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fixed_templates_choose_a_new_name_for_existing_default_outputs() {
        let dir = std::env::temp_dir().join(format!("sozocraft-cli-test-{}", Uuid::new_v4()));
        let settings = AppSettings {
            output_directory: dir.to_str().unwrap().to_string(),
            output_template: "dated/test_{provider}_{id}.{extension}".to_string(),
            ..AppSettings::default()
        };
        let output = Output::prepare(None, &settings, "veo-3.1-generate-preview").unwrap();
        fs::write(&output.partial, b"first").unwrap();
        output.publish().unwrap();
        let next = Output::prepare(None, &settings, "veo-3.1-generate-preview").unwrap();
        assert_ne!(output.final_path, next.final_path);
        assert_eq!(
            next.final_path.file_name().unwrap(),
            "test_google_001_0001.mp4"
        );
        assert_eq!(fs::read(&output.final_path).unwrap(), b"first");
        drop(next);
        drop(output);
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn dispatches_both_models_using_shared_gemini_settings() {
        use std::{io::Write, net::TcpListener};
        for model in [
            "veo-3.1-generate-preview",
            "veo-3.1-lite-generate-preview",
            "gemini-omni-1.1-flash",
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let mut captured = Vec::new();
                for response in if model == "gemini-omni-1.1-flash" {
                    vec![
                        json!({"status":"completed","steps":[{"type":"model_output","content":[{"type":"video","uri":"https://generativelanguage.googleapis.com/v1beta/files/test_file"}]}]}),
                        json!({"state":"PROCESSING"}),
                    ]
                } else {
                    vec![
                        json!({"name":format!("models/{model}/operations/test_job")}),
                        json!({"done":false}),
                    ]
                } {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    loop {
                        let mut buffer = [0u8; 4096];
                        let count = stream.read(&mut buffer).unwrap();
                        assert!(count > 0);
                        bytes.extend_from_slice(&buffer[..count]);
                        if let Some(index) = bytes.windows(4).position(|value| value == b"\r\n\r\n")
                        {
                            let headers = String::from_utf8_lossy(&bytes[..index]);
                            let length: usize = headers
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length: ")
                                        .map(str::to_string)
                                })
                                .map(|value| value.parse().unwrap())
                                .unwrap_or(0);
                            if bytes.len() >= index + 4 + length {
                                break;
                            }
                        }
                    }
                    captured.push(String::from_utf8(bytes).unwrap());
                    let body = response.to_string();
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
                }
                captured
            });
            let settings = AppSettings {
                optional_base_url: Some(format!("http://{address}/v1beta/models")),
                gemini_proxy_enabled: false,
                ..AppSettings::default()
            };
            let client = Client::new(model, "test-key-only", &settings).unwrap();
            let request = build_request(&options(&[
                "video",
                "generate",
                "--model",
                model,
                "--prompt",
                "Animate pixels",
            ]))
            .unwrap();
            let operation = client.start(&request).await.unwrap();
            validate_operation(model, &operation).unwrap();
            assert!(matches!(
                client.poll(&operation).await.unwrap(),
                Status::Pending
            ));
            let captured = server.join().unwrap();
            assert!(captured.iter().all(|value| value
                .to_ascii_lowercase()
                .contains("x-goog-api-key: test-key-only")));
            if model == "gemini-omni-1.1-flash" {
                assert!(captured[0].starts_with("POST /v1beta/interactions "));
                assert!(captured[0].contains("gemini-omni-1.1-flash"));
                let body: serde_json::Value =
                    serde_json::from_str(captured[0].split_once("\r\n\r\n").unwrap().1).unwrap();
                assert_eq!(body["store"], true);
                assert_eq!(body["background"], false);
                assert_eq!(body["stream"], false);
                assert!(captured[1].starts_with("GET /v1beta/files/test_file "));
            } else {
                assert!(captured[0]
                    .starts_with(&format!("POST /v1beta/models/{model}:predictLongRunning ")));
                assert!(captured[1]
                    .starts_with(&format!("GET /v1beta/models/{model}/operations/test_job ")));
            }
        }
    }
}
