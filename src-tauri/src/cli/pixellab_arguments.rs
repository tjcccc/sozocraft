use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub(super) enum PixelLab {
    Balance {
        dry_run: bool,
    },
    Characters {
        limit: u32,
        offset: u32,
        dry_run: bool,
    },
    Character {
        id: String,
        dry_run: bool,
    },
    Job {
        id: String,
        dry_run: bool,
    },
    Submit {
        request: PathBuf,
        reference: Option<PathBuf>,
        create: bool,
        dry_run: bool,
    },
    Download {
        id: String,
        output: PathBuf,
        dry_run: bool,
    },
}

pub(super) fn parse(args: &[String]) -> Result<super::arguments::Command, String> {
    use super::arguments::Command;
    if args.len() >= 2
        && args
            .last()
            .is_some_and(|s| matches!(s.as_str(), "--help" | "-h"))
        && args.len() <= 3
    {
        return Ok(Command::Help);
    }
    let action = args.get(1).map(String::as_str).unwrap_or_default();
    let allowed: &[&str] = match action {
        "balance" | "character" | "job" => &[],
        "characters" => &["--limit", "--offset"],
        "create-character" => &["--request", "--reference"],
        "animate" => &["--request"],
        "download" => &["--output"],
        _ => return Err("Expected pixellab balance, characters, character, create-character, animate, job, or download. Use --help.".into()),
    };
    let mut options = HashMap::new();
    let mut dry_run = false;
    let mut id = None;
    let mut iter = args.iter().skip(2);
    while let Some(flag) = iter.next() {
        if flag == "--dry-run" {
            if dry_run {
                return Err("Duplicate --dry-run.".into());
            }
            dry_run = true;
        } else if allowed.contains(&flag.as_str()) {
            let value = iter
                .next()
                .filter(|v| !v.starts_with("--"))
                .ok_or("Missing PixelLab option value.")?;
            if options.insert(flag.as_str(), value.as_str()).is_some() {
                return Err("Duplicate PixelLab option.".into());
            }
        } else if ["character", "job", "download"].contains(&action)
            && id.is_none()
            && !flag.starts_with('-')
        {
            id = Some(crate::pixellab::request::checked_id(flag)?);
        } else {
            return Err("Unknown option for this PixelLab command. Use --help.".into());
        }
    }
    let required = |flag| {
        options
            .get(flag)
            .copied()
            .ok_or_else(|| format!("Supply {flag}."))
    };
    let number = |flag, default| {
        options.get(flag).map_or(Ok(default), |v| {
            v.parse::<u32>()
                .map_err(|_| "PixelLab pagination must use nonnegative integers.".to_string())
        })
    };
    let command = match action {
        "balance" => PixelLab::Balance { dry_run },
        "characters" => {
            let limit = number("--limit", 50)?;
            if !(1..=100).contains(&limit) {
                return Err("Character limit must be 1-100.".into());
            }
            PixelLab::Characters {
                limit,
                offset: number("--offset", 0)?,
                dry_run,
            }
        }
        "character" => PixelLab::Character {
            id: id.ok_or("Supply character UUID.")?,
            dry_run,
        },
        "job" => PixelLab::Job {
            id: id.ok_or("Supply job UUID.")?,
            dry_run,
        },
        "create-character" | "animate" => PixelLab::Submit {
            request: PathBuf::from(required("--request")?),
            reference: options.get("--reference").map(PathBuf::from),
            create: action == "create-character",
            dry_run,
        },
        "download" => PixelLab::Download {
            id: id.ok_or("Supply character UUID.")?,
            output: PathBuf::from(required("--output")?),
            dry_run,
        },
        _ => unreachable!(),
    };
    Ok(Command::PixelLab(command))
}
