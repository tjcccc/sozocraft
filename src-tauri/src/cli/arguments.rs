use super::video_models;
pub(super) use super::video_models::validate_operation;
use std::{collections::HashSet, ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub(super) enum Command {
    Help,
    Version,
    Image(super::image_arguments::GenerateImage),
    Generate(Generate),
    Status {
        operation: String,
        model: String,
        platform: Option<String>,
    },
    Wait {
        operation: String,
        model: String,
        platform: Option<String>,
        output: Option<PathBuf>,
        max_wait: u64,
    },
}

#[derive(Debug)]
pub(super) struct Generate {
    pub model: String,
    pub platform: Option<String>,
    pub audio: Option<bool>,
    pub prompt: Option<String>,
    pub prompt_file: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub references: Vec<String>,
    pub duration: u8,
    pub aspect_ratio: String,
    pub resolution: String,
    pub output: Option<PathBuf>,
    pub max_wait: u64,
    pub no_wait: bool,
    pub dry_run: bool,
}

pub(super) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let args = args
        .into_iter()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "Arguments must be UTF-8.".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(Command::Help);
    }
    if args == ["--version"] || args == ["-V"] {
        return Ok(Command::Version);
    }
    if args.first().map(String::as_str) == Some("image") {
        return super::image_arguments::parse(&args);
    }
    if args.first().map(String::as_str) != Some("video") {
        return Err(
            "Expected image generate, video generate, video status, or video wait. Use --help."
                .to_string(),
        );
    }
    if args == ["video", "--help"] || args == ["video", "-h"] {
        return Ok(Command::Help);
    }
    let action = args.get(1).map(String::as_str).unwrap_or_default();
    if !["generate", "status", "wait"].contains(&action) {
        return Err(
            "Expected image generate, video generate, video status, or video wait. Use --help."
                .to_string(),
        );
    }
    if args.len() == 3 && matches!(args[2].as_str(), "--help" | "-h") {
        return Ok(Command::Help);
    }
    let mut options = Generate {
        model: "veo-3.1-generate-preview".to_string(),
        platform: None,
        audio: None,
        prompt: None,
        prompt_file: None,
        start: None,
        end: None,
        references: Vec::new(),
        duration: 8,
        aspect_ratio: "16:9".to_string(),
        resolution: "720p".to_string(),
        output: None,
        max_wait: 900,
        no_wait: false,
        dry_run: false,
    };
    let mut operation = None;
    let mut seen = HashSet::new();
    let mut args = args.iter().skip(2);
    while let Some(flag) = args.next() {
        let allowed = match action {
            "generate" => [
                "--model",
                "--platform",
                "--generate-audio",
                "--prompt",
                "--prompt-file",
                "--start-image",
                "--end-image",
                "--reference",
                "--duration",
                "--aspect-ratio",
                "--resolution",
                "--output",
                "--max-wait",
                "--no-wait",
                "--dry-run",
            ]
            .contains(&flag.as_str()),
            "status" => ["--operation", "--model", "--platform"].contains(&flag.as_str()),
            _ => [
                "--operation",
                "--model",
                "--platform",
                "--output",
                "--max-wait",
            ]
            .contains(&flag.as_str()),
        };
        if !allowed {
            return Err("Unknown option for this command. Use --help.".to_string());
        }
        if !seen.insert(flag.as_str()) && flag != "--reference" {
            return Err(format!("Duplicate option: {flag}"));
        }
        if flag == "--no-wait" {
            options.no_wait = true;
            continue;
        }
        if flag == "--dry-run" {
            options.dry_run = true;
            continue;
        }
        let value = args
            .next()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("Missing value for {flag}."))?;
        match flag.as_str() {
            "--model" => options.model = value.clone(),
            "--platform" => options.platform = Some(value.clone()),
            "--generate-audio" => {
                options.audio = Some(match value.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return Err("--generate-audio must be true or false.".to_string()),
                })
            }
            "--prompt" => options.prompt = Some(value.clone()),
            "--prompt-file" => options.prompt_file = Some(value.clone()),
            "--start-image" => options.start = Some(value.clone()),
            "--end-image" => options.end = Some(value.clone()),
            "--reference" => options.references.push(value.clone()),
            "--duration" => {
                options.duration = value
                    .parse()
                    .map_err(|_| "Duration must be an integer.".to_string())?
            }
            "--aspect-ratio" => options.aspect_ratio = value.clone(),
            "--resolution" => options.resolution = value.clone(),
            "--output" => options.output = Some(PathBuf::from(value)),
            "--max-wait" => {
                options.max_wait = value
                    .parse()
                    .map_err(|_| "Max wait must be an integer.".to_string())?
            }
            "--operation" => operation = Some(value.clone()),
            _ => unreachable!(),
        }
    }
    if !(1..=3600).contains(&options.max_wait) {
        return Err("Max wait must be between 1 and 3600 seconds.".to_string());
    }
    validate_model(&options.model)?;
    let provider = video_models::provider(&options.model)?;
    if let Some(platform) = options.platform.as_deref() {
        video_models::validate_platform(&options.model, platform)?;
    }
    if options.audio.is_some() && provider == crate::models::VideoProvider::GoogleVeo {
        return Err("Google video models do not support --generate-audio.".to_string());
    }
    if (options.model == "gemini-omni-1.1-flash"
        || provider != crate::models::VideoProvider::GoogleVeo)
        && !seen.contains("--duration")
    {
        options.duration = 5;
    }
    if provider == crate::models::VideoProvider::GrokImagine && !seen.contains("--resolution") {
        options.resolution = "480p".to_string();
    }
    if action == "generate" {
        if options.model == "veo-3.1-lite-generate-preview" && !options.references.is_empty() {
            return Err(
                "Veo 3.1 Lite does not support --reference; use --start-image/--end-image."
                    .to_string(),
            );
        }
        if options.prompt.is_some() == options.prompt_file.is_some() {
            return Err("Supply exactly one of --prompt or --prompt-file.".to_string());
        }
        if options.no_wait && (options.output.is_some() || seen.contains("--max-wait")) {
            return Err("--no-wait cannot be combined with --output or --max-wait; use video wait to download.".to_string());
        }
        let max_references = video_models::max_references(&options.model);
        if options.references.len() > max_references {
            return Err(format!(
                "This model accepts at most {max_references} reference images."
            ));
        }
        return Ok(Command::Generate(options));
    }
    if provider == crate::models::VideoProvider::Seedance && options.platform.is_none() {
        return Err(
            "Seedance status/wait requires --platform ark or higgsfield from the submitted event."
                .to_string(),
        );
    }
    let operation = operation.ok_or("Supply --operation NAME.")?;
    validate_operation(&options.model, &operation)?;
    if options.platform.as_deref() == Some("higgsfield") {
        crate::higgsfield_video::validate_job_id(&operation)?;
    }
    if action == "status" {
        Ok(Command::Status {
            operation,
            model: options.model,
            platform: options.platform,
        })
    } else {
        Ok(Command::Wait {
            operation,
            model: options.model,
            platform: options.platform,
            output: options.output,
            max_wait: options.max_wait,
        })
    }
}

pub(super) fn validate_model(model: &str) -> Result<(), String> {
    video_models::provider(model).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(values: &[&str]) -> Result<Command, String> {
        parse(values.iter().map(OsString::from))
    }

    #[test]
    fn new_video_models_have_safe_defaults_and_explicit_recovery_routes() {
        for (model, resolution) in [
            ("doubao-seedance-2-0-260128", "720p"),
            ("grok-imagine-video-1.5", "480p"),
        ] {
            let Command::Generate(value) = command(&[
                "video",
                "generate",
                "--model",
                model,
                "--prompt",
                "walk",
                "--generate-audio",
                "false",
            ])
            .unwrap() else {
                panic!("expected generate")
            };
            assert_eq!(value.duration, 5);
            assert_eq!(value.resolution, resolution);
            assert_eq!(value.audio, Some(false));
        }
        assert!(command(&[
            "video",
            "generate",
            "--prompt",
            "walk",
            "--generate-audio",
            "false"
        ])
        .is_err());
        assert!(command(&[
            "video",
            "status",
            "--model",
            "doubao-seedance-2-0-260128",
            "--operation",
            "task-123"
        ])
        .is_err());
        assert!(command(&[
            "video",
            "status",
            "--model",
            "doubao-seedance-2-0-260128",
            "--platform",
            "ark",
            "--operation",
            "task-123"
        ])
        .is_ok());
        assert!(command(&[
            "video",
            "status",
            "--model",
            "grok-imagine-video-1.5",
            "--platform",
            "higgsfield",
            "--operation",
            "task-123"
        ])
        .is_err());
    }

    #[test]
    fn rejects_ambiguous_or_inapplicable_options() {
        for values in [
            vec!["video", "generate"],
            vec![
                "video",
                "generate",
                "--prompt",
                "x",
                "--prompt-file",
                "x.md",
            ],
            vec!["video", "generate", "--prompt", "x", "--prompt", "y"],
            vec![
                "video",
                "status",
                "--operation",
                "operations/abc",
                "--output",
                "x.mp4",
            ],
            vec![
                "video",
                "generate",
                "--prompt",
                "x",
                "--no-wait",
                "--output",
                "x.mp4",
            ],
            vec!["video", "wait", "--operation", "../secret"],
            vec!["video", "wait", "--operation", "operations/%2e%2e"],
            vec![
                "video",
                "wait",
                "--operation",
                "operations/abc",
                "--max-wait",
                "0",
            ],
        ] {
            assert!(command(&values).is_err(), "{values:?}");
        }
    }

    #[test]
    fn parses_frame_pair_and_defaults() {
        let Command::Generate(options) = command(&[
            "video",
            "generate",
            "--prompt",
            "walk",
            "--start-image",
            "start.png",
            "--end-image",
            "end.png",
        ])
        .unwrap() else {
            panic!("expected generate")
        };
        assert_eq!(options.duration, 8);
        assert_eq!(options.start.as_deref(), Some("start.png"));
        assert_eq!(options.end.as_deref(), Some("end.png"));
        assert!(!options.no_wait);
    }

    #[test]
    fn omni_defaults_and_resume_ids_are_model_specific() {
        let Command::Generate(options) = command(&[
            "video",
            "generate",
            "--model",
            "gemini-omni-1.1-flash",
            "--prompt",
            "walk",
        ])
        .unwrap() else {
            panic!("expected generate")
        };
        assert_eq!(options.duration, 5);
        assert!(command(&[
            "video",
            "status",
            "--model",
            "gemini-omni-1.1-flash",
            "--operation",
            "file_abc-123"
        ])
        .is_ok());
        assert!(command(&["video", "status", "--operation", "file_abc-123"]).is_err());
        assert!(command(&[
            "video",
            "status",
            "--model",
            "gemini-omni-1.1-flash",
            "--operation",
            "operations/abc"
        ])
        .is_err());
    }

    #[test]
    fn lite_resume_ids_match_the_model_and_asset_references_are_rejected() {
        assert!(command(&[
            "video",
            "status",
            "--model",
            "veo-3.1-lite-generate-preview",
            "--operation",
            "models/veo-3.1-lite-generate-preview/operations/job"
        ])
        .is_ok());
        assert!(command(&[
            "video",
            "status",
            "--model",
            "veo-3.1-generate-preview",
            "--operation",
            "models/veo-3.1-lite-generate-preview/operations/job"
        ])
        .is_err());
        assert!(command(&[
            "video",
            "generate",
            "--model",
            "veo-3.1-lite-generate-preview",
            "--prompt",
            "walk",
            "--reference",
            "sprite.png"
        ])
        .is_err());
    }
}
