use chrono::{DateTime, TimeZone};
use sanitize_filename::sanitize;
use std::fmt::Display;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FilenameTemplateError {
    #[error("Output filename template produced an empty filename.")]
    EmptyFilename,
    #[error("Output path has no parent directory.")]
    NoParent,
}

pub fn resolve_output_path<Tz>(
    output_dir: &str,
    template: &str,
    provider: &str,
    model: &str,
    id: &str,
    batch_id: &str,
    extension: &str,
    datetime: DateTime<Tz>,
) -> Result<PathBuf, FilenameTemplateError>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let rendered = render_template(template, provider, model, id, batch_id, extension, datetime);
    if rendered.trim().is_empty() {
        return Err(FilenameTemplateError::EmptyFilename);
    }

    let rendered_path = PathBuf::from(rendered);
    let mut candidate = if rendered_path.is_absolute() {
        rendered_path
    } else {
        Path::new(output_dir).join(rendered_path)
    };

    if !template.contains("{extension}") && !extension.is_empty() {
        let filename = candidate
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("image")
            .to_string();
        candidate = candidate.with_file_name(format!("{filename}.{extension}"));
    }

    unique_path(candidate)
}

fn render_template<Tz>(
    template: &str,
    provider: &str,
    model: &str,
    id: &str,
    batch_id: &str,
    extension: &str,
    datetime: DateTime<Tz>,
) -> String
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let mut rendered = template.to_string();
    rendered = rendered.replace("{provider}", &safe_segment(provider));
    rendered = rendered.replace("{model}", &safe_segment(model));
    rendered = rendered.replace("{id}", &safe_segment(id));
    rendered = rendered.replace("{batch_id}", &safe_segment(batch_id));
    rendered = rendered.replace("{extension}", &safe_segment(extension));

    while let Some(start) = rendered.find("{datetime:") {
        let Some(relative_end) = rendered[start..].find('}') else {
            break;
        };
        let end = start + relative_end;
        let token = &rendered[start..=end];
        let format = &rendered[start + "{datetime:".len()..end];
        let chrono_format = convert_datetime_format(format);
        rendered = rendered.replace(token, &datetime.format(&chrono_format).to_string());
    }
    rendered = rendered.replace("{datetime}", &datetime.format("%Y%m%d_%H%M%S").to_string());
    for token in [
        "yyyyMMdd_HHmmss",
        "yyyyMMdd",
        "yyMMdd_HHmmss",
        "yyMMdd",
        "yyyy",
        "yy",
        "MM",
        "dd",
        "HH",
        "mm",
        "ss",
    ] {
        let placeholder = format!("{{{token}}}");
        if rendered.contains(&placeholder) {
            rendered = rendered.replace(
                &placeholder,
                &datetime.format(&convert_datetime_format(token)).to_string(),
            );
        }
    }
    rendered
}

fn convert_datetime_format(format: &str) -> String {
    format
        .replace("yyyy", "%Y")
        .replace("yy", "%y")
        .replace("MMdd", "%m%d")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

fn safe_segment(value: &str) -> String {
    let sanitized = sanitize(value).replace(' ', "_");
    sanitized
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

fn unique_path(path: PathBuf) -> Result<PathBuf, FilenameTemplateError> {
    if !path.exists() {
        return Ok(path);
    }

    let parent = path.parent().ok_or(FilenameTemplateError::NoParent)?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..=9999 {
        let filename = match extension {
            Some(ext) => format!("{stem}_{index:04}.{ext}"),
            None => format!("{stem}_{index:04}"),
        };
        let candidate = parent.join(filename);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn resolves_template_with_safe_names() {
        let date = Utc.with_ymd_and_hms(2026, 4, 26, 9, 8, 7).unwrap();
        let path = resolve_output_path(
            "/tmp/out",
            "{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}_{batch_id}.{extension}",
            "nano/banana",
            "gemini 3 pro:image",
            "abc",
            "batch",
            "png",
            date,
        )
        .unwrap();

        let rendered = path.to_string_lossy();
        assert!(rendered.contains("20260426_090807_abc_batch.png"));
        assert!(!rendered.contains("nano/banana"));
        assert!(!rendered.contains("pro:image"));
    }

    #[test]
    fn resolves_date_token_folder() {
        let date = Utc.with_ymd_and_hms(2026, 4, 26, 9, 8, 7).unwrap();
        let path = resolve_output_path(
            "/tmp/out",
            "{yyMMdd}/{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}",
            "gemini",
            "nano-banana-2",
            "001",
            "batch",
            "jpg",
            date,
        )
        .unwrap();

        let rendered = path.to_string_lossy();
        assert!(rendered.contains("260426/gemini_nano-banana-2_20260426_090807_001.jpg"));
    }

    #[test]
    fn appends_extension_when_template_omits_it() {
        let date = Utc.with_ymd_and_hms(2026, 4, 26, 9, 8, 7).unwrap();
        let path = resolve_output_path(
            "/tmp/out",
            "{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}",
            "gemini",
            "nano-banana-2",
            "001",
            "batch",
            "webp",
            date,
        )
        .unwrap();

        let rendered = path.to_string_lossy();
        assert!(
            rendered.ends_with(".webp"),
            "expected .webp suffix, got: {rendered}"
        );
        assert!(rendered.contains("gemini_nano-banana-2_20260426_090807_001.webp"));
    }

    #[test]
    fn leaves_legacy_uppercase_year_tokens_literal() {
        let date = Utc.with_ymd_and_hms(2026, 4, 26, 9, 8, 7).unwrap();
        let path = resolve_output_path(
            "/tmp/out",
            "{YYMMdd}/{provider}_{model}_{datetime:YYYYMMdd_HHmmss}_{id}.{extension}",
            "gemini",
            "nano-banana-2",
            "001",
            "batch",
            "jpg",
            date,
        )
        .unwrap();

        let rendered = path.to_string_lossy();
        assert!(rendered.contains("{YYMMdd}/gemini_nano-banana-2_YYYY0426_090807_001.jpg"));
    }
}
