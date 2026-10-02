use super::arguments::Command;
use std::{collections::HashSet, path::PathBuf};

#[derive(Debug, Default)]
pub(super) struct GenerateImage {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub prompt_file: Option<String>,
    pub references: Vec<String>,
    pub aspect_ratio: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub thinking_level: Option<String>,
    pub output: Option<PathBuf>,
    pub dry_run: bool,
}

pub(super) fn parse(args: &[String]) -> Result<Command, String> {
    if args == ["image", "--help"]
        || args == ["image", "-h"]
        || (args.len() == 3 && args[1] == "generate" && matches!(args[2].as_str(), "--help" | "-h"))
    {
        return Ok(Command::Help);
    }
    if args.get(1).map(String::as_str) != Some("generate") {
        return Err("Expected image generate. Use --help.".to_string());
    }
    let mut options = GenerateImage::default();
    let mut seen = HashSet::new();
    let mut iter = args.iter().skip(2);
    while let Some(flag) = iter.next() {
        if ![
            "--provider",
            "--model",
            "--prompt",
            "--prompt-file",
            "--reference",
            "--aspect-ratio",
            "--size",
            "--quality",
            "--thinking-level",
            "--output",
            "--dry-run",
        ]
        .contains(&flag.as_str())
        {
            return Err("Unknown option for image generate. Use --help.".to_string());
        }
        if !seen.insert(flag) && flag != "--reference" {
            return Err(format!("Duplicate option: {flag}"));
        }
        if flag == "--dry-run" {
            options.dry_run = true;
            continue;
        }
        let value = iter
            .next()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("Missing value for {flag}."))?;
        match flag.as_str() {
            "--provider" => options.provider = Some(value.clone()),
            "--model" => options.model = Some(value.clone()),
            "--prompt" => options.prompt = Some(value.clone()),
            "--prompt-file" => options.prompt_file = Some(value.clone()),
            "--reference" => options.references.push(value.clone()),
            "--aspect-ratio" => options.aspect_ratio = Some(value.clone()),
            "--size" => options.size = Some(value.clone()),
            "--quality" => options.quality = Some(value.clone()),
            "--thinking-level" => options.thinking_level = Some(value.clone()),
            "--output" => options.output = Some(PathBuf::from(value)),
            _ => unreachable!(),
        }
    }
    if options.prompt.is_some() == options.prompt_file.is_some() {
        return Err("Supply exactly one of --prompt or --prompt-file.".to_string());
    }
    if options.references.len() > 16 {
        return Err(
            "Image generation accepts at most 16 reference images, subject to model limits."
                .to_string(),
        );
    }
    Ok(Command::Image(options))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_missing_and_video_options() {
        for tail in [
            vec![],
            vec!["--prompt", "x", "--prompt", "y"],
            vec!["--prompt", "x", "--duration", "8"],
            vec!["--prompt", "x", "--size"],
            vec!["--prompt", "x", "--prompt-file", "x.txt"],
        ] {
            let args: Vec<String> = [vec!["image", "generate"], tail]
                .concat()
                .into_iter()
                .map(str::to_string)
                .collect();
            assert!(parse(&args).is_err());
        }
    }
}
