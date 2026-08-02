use crate::{
    filename_template::{higgsfield_serial_pattern, resolve_higgsfield_output_path},
    models::AppSettings,
};
use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub fn source_filename_from_url(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()?
        .path_segments()?
        .next_back()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

pub async fn archive_bytes(
    settings: &AppSettings,
    provider: &str,
    model: &str,
    id: &str,
    batch_id: &str,
    source_filename: &str,
    datetime: DateTime<Local>,
    bytes: &[u8],
) -> Result<Option<PathBuf>, String> {
    if !settings.higgsfield_output_enabled {
        return Ok(None);
    }
    let id = next_serial_id(settings, provider, model, id, batch_id, datetime).await?;
    let path = archive_path(
        settings,
        provider,
        model,
        &id,
        batch_id,
        source_filename,
        datetime,
    )?;
    let parent = prepare_parent(&path, settings.higgsfield_output_directory.trim()).await?;
    write_atomic(&path, &parent, bytes).await?;
    Ok(Some(path))
}

pub async fn archive_file(
    settings: &AppSettings,
    provider: &str,
    model: &str,
    id: &str,
    batch_id: &str,
    source_filename: &str,
    datetime: DateTime<Local>,
    source_path: &Path,
) -> Result<Option<PathBuf>, String> {
    if !settings.higgsfield_output_enabled {
        return Ok(None);
    }
    let id = next_serial_id(settings, provider, model, id, batch_id, datetime).await?;
    let path = archive_path(
        settings,
        provider,
        model,
        &id,
        batch_id,
        source_filename,
        datetime,
    )?;
    prepare_parent(&path, settings.higgsfield_output_directory.trim()).await?;
    let temporary_path = temporary_path(&path);
    if let Err(error) = tokio::fs::copy(source_path, &temporary_path).await {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Err(format!("Failed to copy Higgsfield output: {error}"));
    }
    if let Err(error) = tokio::fs::rename(&temporary_path, &path).await {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Err(format!("Failed to finalize Higgsfield output: {error}"));
    }
    Ok(Some(path))
}

fn archive_path(
    settings: &AppSettings,
    provider: &str,
    model: &str,
    id: &str,
    batch_id: &str,
    source_filename: &str,
    datetime: DateTime<Local>,
) -> Result<PathBuf, String> {
    if settings.higgsfield_output_directory.trim().is_empty() {
        return Err("Higgsfield output directory cannot be empty.".to_string());
    }
    if settings.higgsfield_output_template.trim().is_empty() {
        return Err("Higgsfield filename template cannot be empty.".to_string());
    }
    resolve_higgsfield_output_path(
        settings.higgsfield_output_directory.trim(),
        settings.higgsfield_output_template.trim(),
        provider,
        model,
        id,
        batch_id,
        source_filename,
        datetime,
    )
    .map_err(|error| error.to_string())
}

async fn next_serial_id(
    settings: &AppSettings,
    provider: &str,
    model: &str,
    fallback_id: &str,
    batch_id: &str,
    datetime: DateTime<Local>,
) -> Result<String, String> {
    const ID_MARKER: &str = "SOZOCRAFTSERIALID";
    const SOURCE_MARKER: &str = "SOZOCRAFTSOURCEFILE";

    let fallback = fallback_id.parse::<u64>().unwrap_or(1).max(1);
    let Some(pattern_path) = higgsfield_serial_pattern(
        settings.higgsfield_output_directory.trim(),
        settings.higgsfield_output_template.trim(),
        provider,
        model,
        batch_id,
        datetime,
    )
    .map_err(|error| error.to_string())?
    else {
        return Ok(fallback_id.to_string());
    };
    let Some(parent) = pattern_path.parent() else {
        return Ok(fallback.to_string());
    };
    let Some(pattern) = pattern_path.file_name().and_then(|value| value.to_str()) else {
        return Ok(fallback.to_string());
    };
    if pattern_path
        .parent()
        .is_some_and(|value| value.to_string_lossy().contains(ID_MARKER))
    {
        return Ok(fallback.to_string());
    }

    let mut maximum = None;
    let mut entries = match tokio::fs::read_dir(parent).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(fallback.to_string())
        }
        Err(error) => {
            return Err(format!(
                "Failed to inspect existing Higgsfield outputs: {error}"
            ))
        }
    };
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| format!("Failed to inspect existing Higgsfield outputs: {error}"))?
    {
        if !entry
            .file_type()
            .await
            .map_err(|error| format!("Failed to inspect Higgsfield output: {error}"))?
            .is_file()
        {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        if let Some(serial) = extract_serial(pattern, &filename, ID_MARKER, SOURCE_MARKER) {
            maximum = Some(maximum.map_or(serial, |current: u64| current.max(serial)));
        }
    }
    Ok(maximum
        .map(|value| value.saturating_add(1).max(fallback))
        .unwrap_or(fallback)
        .to_string())
}

fn extract_serial(
    pattern: &str,
    filename: &str,
    id_marker: &str,
    source_marker: &str,
) -> Option<u64> {
    let bytes = filename.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let digits = &filename[start..index];
        if digits.len() <= 12 {
            let expected = pattern.replace(id_marker, digits);
            if wildcard_match(&expected, filename, source_marker) {
                if let Ok(value) = digits.parse::<u64>() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn wildcard_match(pattern: &str, value: &str, marker: &str) -> bool {
    let parts = pattern.split(marker).collect::<Vec<_>>();
    if parts.len() == 1 {
        return pattern == value;
    }
    if !value.starts_with(parts[0]) {
        return false;
    }
    let mut offset = parts[0].len();
    for part in &parts[1..parts.len() - 1] {
        let Some(relative) = value[offset..].find(part) else {
            return false;
        };
        offset += relative + part.len();
    }
    value[offset..].ends_with(parts.last().copied().unwrap_or_default())
}

async fn prepare_parent(path: &Path, output_root: &str) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Higgsfield output path has no parent directory.".to_string())?;
    tokio::fs::create_dir_all(output_root)
        .await
        .map_err(|error| format!("Failed to create Higgsfield output directory: {error}"))?;
    let canonical_root = tokio::fs::canonicalize(output_root)
        .await
        .map_err(|error| format!("Failed to validate Higgsfield output directory: {error}"))?;
    let mut existing_ancestor = parent;
    while !existing_ancestor.exists() {
        existing_ancestor = existing_ancestor.parent().ok_or_else(|| {
            "Higgsfield output path has no existing parent directory.".to_string()
        })?;
    }
    let canonical_ancestor = tokio::fs::canonicalize(existing_ancestor)
        .await
        .map_err(|error| format!("Failed to validate Higgsfield output path: {error}"))?;
    if !canonical_ancestor.starts_with(&canonical_root) {
        return Err(
            "Higgsfield filename template resolves outside its output directory.".to_string(),
        );
    }
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| format!("Failed to create Higgsfield output directory: {error}"))?;
    let canonical_parent = tokio::fs::canonicalize(parent)
        .await
        .map_err(|error| format!("Failed to validate Higgsfield output path: {error}"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(
            "Higgsfield filename template resolves outside its output directory.".to_string(),
        );
    }
    Ok(parent.to_path_buf())
}

async fn write_atomic(path: &Path, parent: &Path, bytes: &[u8]) -> Result<(), String> {
    debug_assert_eq!(path.parent(), Some(parent));
    let temporary_path = temporary_path(path);
    let mut file = tokio::fs::File::create(&temporary_path)
        .await
        .map_err(|error| format!("Failed to create Higgsfield output: {error}"))?;
    if let Err(error) = file.write_all(bytes).await {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Err(format!("Failed to write Higgsfield output: {error}"));
    }
    if let Err(error) = file.flush().await {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Err(format!("Failed to flush Higgsfield output: {error}"));
    }
    drop(file);
    if let Err(error) = tokio::fs::rename(&temporary_path, path).await {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Err(format!("Failed to finalize Higgsfield output: {error}"));
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("higgsfield_output");
    path.with_file_name(format!(".{filename}.{}.part", Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::{archive_bytes, extract_serial, source_filename_from_url};
    use crate::models::AppSettings;
    use chrono::Local;
    use uuid::Uuid;

    #[test]
    fn extracts_higgsfield_filename_without_query_string() {
        assert_eq!(
            source_filename_from_url(
                "https://cdn.example.com/results/hf_20260802_014525_job.mp4?token=secret"
            )
            .as_deref(),
            Some("hf_20260802_014525_job.mp4")
        );
    }

    #[test]
    fn extracts_only_the_serial_at_the_template_id_position() {
        assert_eq!(
            extract_serial(
                "260802_SOZOCRAFTSERIALID_SOZOCRAFTSOURCEFILE",
                "260802_004_hf_20260802_job.mp4",
                "SOZOCRAFTSERIALID",
                "SOZOCRAFTSOURCEFILE",
            ),
            Some(4)
        );
    }

    #[tokio::test]
    async fn continues_serial_ids_from_matching_existing_outputs() {
        let test_root = std::env::temp_dir().join(format!("sozocraft-hf-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&test_root).unwrap();
        std::fs::write(test_root.join("260802_001_hf_old.png"), b"old").unwrap();
        std::fs::write(test_root.join("260802_004_hf_old.mp4"), b"old").unwrap();

        let mut settings = AppSettings::default();
        settings.higgsfield_output_enabled = true;
        settings.higgsfield_output_directory = test_root.to_string_lossy().to_string();
        settings.higgsfield_output_template = "{yyMMdd}_{id:3}_{higgsfield_filename}".to_string();
        let datetime = chrono::NaiveDate::from_ymd_opt(2026, 8, 2)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .single()
            .unwrap();
        let result = archive_bytes(
            &settings,
            "higgsfield",
            "seedance-2",
            "001",
            "batch",
            "hf_new.mp4",
            datetime,
            b"new",
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(
            result.file_name().and_then(|value| value.to_str()),
            Some("260802_005_hf_new.mp4")
        );
        std::fs::remove_dir_all(test_root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_archive_subdirectories_that_are_symlinks_outside_root() {
        use std::os::unix::fs::symlink;

        let test_root = std::env::temp_dir().join(format!("sozocraft-hf-{}", Uuid::new_v4()));
        let archive_root = test_root.join("archive");
        let outside_root = test_root.join("outside");
        std::fs::create_dir_all(&archive_root).unwrap();
        std::fs::create_dir_all(&outside_root).unwrap();
        symlink(&outside_root, archive_root.join("escaped")).unwrap();

        let mut settings = AppSettings::default();
        settings.higgsfield_output_enabled = true;
        settings.higgsfield_output_directory = archive_root.to_string_lossy().to_string();
        settings.higgsfield_output_template = "escaped/{higgsfield_filename}".to_string();
        let result = archive_bytes(
            &settings,
            "higgsfield",
            "seedance-2",
            "001",
            "batch",
            "hf_test.png",
            Local::now(),
            b"raw image",
        )
        .await;

        assert!(result.is_err());
        assert!(!outside_root.join("hf_test.png").exists());
        std::fs::remove_file(archive_root.join("escaped")).unwrap();
        std::fs::remove_dir_all(test_root).unwrap();
    }
}
