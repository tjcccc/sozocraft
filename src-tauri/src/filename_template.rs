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
    #[error("Higgsfield output directory must be an absolute path.")]
    HiggsfieldDirectoryNotAbsolute,
    #[error("Higgsfield filename template must be relative and cannot contain '..'.")]
    UnsafeHiggsfieldTemplate,
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

pub fn resolve_higgsfield_output_path<Tz>(
    output_dir: &str,
    template: &str,
    provider: &str,
    model: &str,
    id: &str,
    batch_id: &str,
    source_filename: &str,
    datetime: DateTime<Tz>,
) -> Result<PathBuf, FilenameTemplateError>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let output_root = Path::new(output_dir);
    if !output_root.is_absolute() {
        return Err(FilenameTemplateError::HiggsfieldDirectoryNotAbsolute);
    }
    let template_path = Path::new(template);
    if template_path.is_absolute()
        || template_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(FilenameTemplateError::UnsafeHiggsfieldTemplate);
    }

    let source_filename = Path::new(source_filename)
        .file_name()
        .and_then(|value| value.to_str())
        .map(safe_segment)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "higgsfield_output".to_string());
    let extension = Path::new(&source_filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let has_source_filename_token = template.contains("{higgsfield_filename}");
    let template = template.replace("{higgsfield_filename}", &source_filename);
    let rendered = render_template(
        &template, provider, model, id, batch_id, extension, datetime,
    );
    if rendered.trim().is_empty() {
        return Err(FilenameTemplateError::EmptyFilename);
    }

    let mut candidate = output_root.join(rendered);
    if !template.contains("{extension}") && !has_source_filename_token && !extension.is_empty() {
        let filename = candidate
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("higgsfield_output");
        candidate = candidate.with_file_name(format!("{filename}.{extension}"));
    }
    if !candidate.starts_with(output_root) {
        return Err(FilenameTemplateError::UnsafeHiggsfieldTemplate);
    }
    unique_path(candidate)
}

pub fn higgsfield_serial_pattern<Tz>(
    output_dir: &str,
    template: &str,
    provider: &str,
    model: &str,
    batch_id: &str,
    datetime: DateTime<Tz>,
) -> Result<Option<PathBuf>, FilenameTemplateError>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    const ID_MARKER: &str = "SOZOCRAFTSERIALID";
    const SOURCE_MARKER: &str = "SOZOCRAFTSOURCEFILE";

    let output_root = Path::new(output_dir);
    if !output_root.is_absolute() {
        return Err(FilenameTemplateError::HiggsfieldDirectoryNotAbsolute);
    }
    let template_path = Path::new(template);
    if template_path.is_absolute()
        || template_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(FilenameTemplateError::UnsafeHiggsfieldTemplate);
    }

    let (mut pattern, replaced_id) = replace_id_tokens(template.to_string(), ID_MARKER);
    if !replaced_id {
        return Ok(None);
    }
    let has_source_filename = pattern.contains("{higgsfield_filename}");
    let has_extension = pattern.contains("{extension}");
    pattern = pattern.replace("{higgsfield_filename}", SOURCE_MARKER);
    pattern = pattern.replace("{extension}", SOURCE_MARKER);
    pattern = render_template(&pattern, provider, model, "", batch_id, "", datetime);
    let mut path = output_root.join(pattern);
    if !has_source_filename && !has_extension {
        let filename = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("higgsfield_output");
        path = path.with_file_name(format!("{filename}{SOURCE_MARKER}"));
    }
    if !path.starts_with(output_root) {
        return Err(FilenameTemplateError::UnsafeHiggsfieldTemplate);
    }
    Ok(Some(path))
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
    rendered = replace_id_width_tokens(rendered, id);
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

fn replace_id_width_tokens(mut rendered: String, id: &str) -> String {
    let mut search_from = 0;
    while let Some(relative_start) = rendered[search_from..].find("{id:") {
        let start = search_from + relative_start;
        let Some(relative_end) = rendered[start..].find('}') else {
            break;
        };
        let end = start + relative_end;
        let width_text = &rendered[start + "{id:".len()..end];
        let replacement = width_text
            .parse::<usize>()
            .ok()
            .filter(|width| (1..=12).contains(width))
            .and_then(|width| {
                id.parse::<u64>()
                    .ok()
                    .map(|number| format!("{number:0width$}"))
            });
        if let Some(replacement) = replacement {
            rendered.replace_range(start..=end, &replacement);
            search_from = start + replacement.len();
        } else {
            search_from = end + 1;
        }
    }
    rendered
}

fn replace_id_tokens(mut rendered: String, replacement: &str) -> (String, bool) {
    let mut replaced = false;
    let mut search_from = 0;
    while let Some(relative_start) = rendered[search_from..].find("{id") {
        let start = search_from + relative_start;
        let Some(relative_end) = rendered[start..].find('}') else {
            break;
        };
        let end = start + relative_end;
        let token = &rendered[start..=end];
        let valid = token == "{id}"
            || token
                .strip_prefix("{id:")
                .and_then(|value| value.strip_suffix('}'))
                .and_then(|value| value.parse::<usize>().ok())
                .is_some_and(|width| (1..=12).contains(&width));
        if valid {
            rendered.replace_range(start..=end, replacement);
            search_from = start + replacement.len();
            replaced = true;
        } else {
            search_from = end + 1;
        }
    }
    (rendered, replaced)
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
    fn formats_numeric_ids_with_requested_width() {
        let date = Utc.with_ymd_and_hms(2026, 8, 2, 10, 9, 8).unwrap();
        let path = resolve_output_path(
            "/tmp/out",
            "{id}_{id:2}_{id:4}.{extension}",
            "higgsfield",
            "seedance-2",
            "001",
            "batch",
            "mp4",
            date,
        )
        .unwrap();

        assert!(path.to_string_lossy().ends_with("001_01_0001.mp4"));
    }

    #[test]
    fn resolves_higgsfield_original_filename_inside_archive_root() {
        let date = Utc.with_ymd_and_hms(2026, 8, 2, 10, 9, 8).unwrap();
        let path = resolve_higgsfield_output_path(
            "/tmp/higgsfield",
            "{yyMMdd} {id:4} {higgsfield_filename}",
            "higgsfield",
            "seedance-2",
            "001",
            "batch",
            "hf_20260802_014525_99f8.mp4",
            date,
        )
        .unwrap();

        assert!(path
            .to_string_lossy()
            .ends_with("260802 0001 hf_20260802_014525_99f8.mp4"));
    }

    #[test]
    fn renders_higgsfield_serial_scan_pattern() {
        let date = Utc.with_ymd_and_hms(2026, 8, 2, 10, 9, 8).unwrap();
        let pattern = higgsfield_serial_pattern(
            "/tmp/higgsfield",
            "{yyMMdd}_{id:3}_{higgsfield_filename}",
            "higgsfield",
            "seedance-2",
            "batch",
            date,
        )
        .unwrap()
        .unwrap();

        assert!(pattern
            .to_string_lossy()
            .ends_with("260802_SOZOCRAFTSERIALID_SOZOCRAFTSOURCEFILE"));
    }

    #[test]
    fn rejects_higgsfield_templates_that_escape_archive_root() {
        let date = Utc.with_ymd_and_hms(2026, 8, 2, 10, 9, 8).unwrap();
        let error = resolve_higgsfield_output_path(
            "/tmp/higgsfield",
            "../{higgsfield_filename}",
            "higgsfield",
            "seedance-2",
            "001",
            "batch",
            "hf.mp4",
            date,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FilenameTemplateError::UnsafeHiggsfieldTemplate
        ));
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
