use std::{collections::HashSet, ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub(super) enum Command {
    Help,
    Generate(Generate),
    Status {
        operation: String,
        model: String,
    },
    Wait {
        operation: String,
        model: String,
        output: Option<PathBuf>,
        max_wait: u64,
    },
}

#[derive(Debug)]
pub(super) struct Generate {
    pub model: String,
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
    if args.first().map(String::as_str) != Some("video") {
        return Err(
            "Expected video generate, video status, or video wait. Use --help.".to_string(),
        );
    }
    if args == ["video", "--help"] || args == ["video", "-h"] {
        return Ok(Command::Help);
    }
    let action = args.get(1).map(String::as_str).unwrap_or_default();
    if !["generate", "status", "wait"].contains(&action) {
        return Err(
            "Expected video generate, video status, or video wait. Use --help.".to_string(),
        );
    }
    if args.len() == 3 && matches!(args[2].as_str(), "--help" | "-h") {
        return Ok(Command::Help);
    }
    let mut options = Generate {
        model: "veo-3.1-generate-preview".to_string(),
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
            "status" => ["--operation", "--model"].contains(&flag.as_str()),
            _ => ["--operation", "--model", "--output", "--max-wait"].contains(&flag.as_str()),
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
    if options.model == "gemini-omni-1.1-flash" && !seen.contains("--duration") {
        options.duration = 5;
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
        let max_references = if options.model == "gemini-omni-1.1-flash" {
            6
        } else {
            3
        };
        if options.references.len() > max_references {
            return Err(format!(
                "This model accepts at most {max_references} reference images."
            ));
        }
        return Ok(Command::Generate(options));
    }
    let operation = operation.ok_or("Supply --operation NAME.")?;
    validate_operation(&options.model, &operation)?;
    if action == "status" {
        Ok(Command::Status {
            operation,
            model: options.model,
        })
    } else {
        Ok(Command::Wait {
            operation,
            model: options.model,
            output: options.output,
            max_wait: options.max_wait,
        })
    }
}

pub(super) fn validate_model(model: &str) -> Result<(), String> {
    if matches!(
        model,
        "veo-3.1-generate-preview" | "veo-3.1-lite-generate-preview" | "gemini-omni-1.1-flash"
    ) {
        Ok(())
    } else {
        Err("Supported models: veo-3.1-generate-preview, veo-3.1-lite-generate-preview, gemini-omni-1.1-flash.".to_string())
    }
}

pub(super) fn validate_operation(model: &str, operation: &str) -> Result<(), String> {
    if model == "gemini-omni-1.1-flash" {
        crate::google_omni::validate_file_id(operation)
    } else {
        crate::google_veo::validate_operation_name(operation)?;
        if !operation
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
        {
            return Err("Invalid characters in Veo operation name.".to_string());
        }
        if !operation.starts_with(&format!("models/{model}/operations/"))
            && !operation.starts_with("operations/")
        {
            return Err("Expected a Veo operation name returned by video generate.".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(values: &[&str]) -> Result<Command, String> {
        parse(values.iter().map(OsString::from))
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
