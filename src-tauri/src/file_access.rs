use base64::{engine::general_purpose, Engine as _};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::image_meta;

const MAX_TEXT_IMPORT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IMAGE_READ_BYTES: u64 = 100 * 1024 * 1024;
const MAX_VIDEO_PREVIEW_BYTES: u64 = 1024 * 1024 * 1024;

pub fn read_image_data_url(path: &str) -> Result<String, String> {
    let path = validate_readable_file(
        path,
        &["png", "jpg", "jpeg", "webp"],
        MAX_IMAGE_READ_BYTES,
        "image",
    )?;
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read image: {err}"))?;
    let mime = match file_extension(&path).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "image/png",
    };
    Ok(format!(
        "data:{mime};base64,{}",
        general_purpose::STANDARD.encode(bytes)
    ))
}

pub fn read_text_file(path: &str) -> Result<String, String> {
    let path = validate_readable_file(
        path,
        &["md", "markdown", "txt"],
        MAX_TEXT_IMPORT_BYTES,
        "text file",
    )?;
    fs::read_to_string(&path).map_err(|err| format!("Failed to read text file: {err}"))
}

pub fn read_image_text_metadata(path: &str) -> Result<HashMap<String, String>, String> {
    let path = validate_readable_file(
        path,
        &["png", "jpg", "jpeg", "webp"],
        MAX_IMAGE_READ_BYTES,
        "image",
    )?;
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read image metadata: {err}"))?;
    Ok(image_meta::read_png_text_chunks(&bytes)
        .into_iter()
        .collect())
}

pub fn validate_generated_video_preview(
    path: &str,
    generated_video_paths: &[String],
) -> Result<PathBuf, String> {
    let requested = PathBuf::from(path.trim());
    if requested.as_os_str().is_empty() {
        return Err("Choose a generated video first.".to_string());
    }
    let metadata = fs::symlink_metadata(&requested)
        .map_err(|error| format!("Failed to inspect generated video: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Generated video preview must be a regular file.".to_string());
    }
    if metadata.len() > MAX_VIDEO_PREVIEW_BYTES {
        return Err("Generated video is too large to preview.".to_string());
    }
    let canonical = requested
        .canonicalize()
        .map_err(|error| format!("Failed to resolve generated video: {error}"))?;
    if file_extension(&canonical).as_deref() != Some("mp4") {
        return Err("Only generated MP4 videos can be previewed.".to_string());
    }
    let is_generated_video = generated_video_paths.iter().any(|path| {
        PathBuf::from(path)
            .canonicalize()
            .map(|candidate| candidate == canonical)
            .unwrap_or(false)
    });
    if !is_generated_video {
        return Err("Video is not present in SozoCraft generation history.".to_string());
    }
    Ok(canonical)
}

fn validate_readable_file(
    path: &str,
    allowed_extensions: &[&str],
    max_bytes: u64,
    label: &str,
) -> Result<PathBuf, String> {
    let path = PathBuf::from(path.trim());
    if path.as_os_str().is_empty() {
        return Err(format!("Choose a {label} first."));
    }
    if !path.is_file() {
        return Err(format!("The selected {label} is not a file."));
    }
    let extension = file_extension(&path).ok_or_else(|| {
        format!(
            "Unsupported {label} type. Use one of: {}.",
            allowed_extensions.join(", ")
        )
    })?;
    if !allowed_extensions
        .iter()
        .any(|allowed| *allowed == extension.as_str())
    {
        return Err(format!(
            "Unsupported {label} type `.{extension}`. Use one of: {}.",
            allowed_extensions.join(", ")
        ));
    }
    let size = fs::metadata(&path)
        .map_err(|err| format!("Failed to inspect {label}: {err}"))?
        .len();
    if size > max_bytes {
        return Err(format!(
            "The selected {label} is too large ({:.1} MB). Maximum supported size is {:.1} MB.",
            size as f64 / 1_048_576.0,
            max_bytes as f64 / 1_048_576.0
        ));
    }
    Ok(path)
}

fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{validate_generated_video_preview, validate_readable_file, MAX_TEXT_IMPORT_BYTES};
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn readable_file_validation_rejects_unsupported_extension() {
        let path = temp_path("exe");
        fs::write(&path, "not a prompt").expect("write temp file");

        let err = validate_readable_file(
            path.to_string_lossy().as_ref(),
            &["md", "txt"],
            MAX_TEXT_IMPORT_BYTES,
            "text file",
        )
        .unwrap_err();

        assert!(err.contains("Unsupported text file type"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn readable_file_validation_rejects_oversized_file() {
        let path = temp_path("txt");
        let file = fs::File::create(&path).expect("create temp file");
        file.set_len(MAX_TEXT_IMPORT_BYTES + 1)
            .expect("grow temp file");

        let err = validate_readable_file(
            path.to_string_lossy().as_ref(),
            &["md", "txt"],
            MAX_TEXT_IMPORT_BYTES,
            "text file",
        )
        .unwrap_err();

        assert!(err.contains("too large"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn video_preview_requires_an_exact_generated_mp4_path() {
        let generated = temp_path("mp4");
        let other = temp_path("mp4");
        fs::write(&generated, b"video").expect("write generated video");
        fs::write(&other, b"video").expect("write other video");
        let allowed = vec![generated.to_string_lossy().to_string()];

        let canonical =
            validate_generated_video_preview(generated.to_string_lossy().as_ref(), &allowed)
                .expect("generated video should be allowed");
        assert_eq!(canonical, generated.canonicalize().unwrap());

        let error = validate_generated_video_preview(other.to_string_lossy().as_ref(), &allowed)
            .unwrap_err();
        assert!(error.contains("generation history"));
        let _ = fs::remove_file(generated);
        let _ = fs::remove_file(other);
    }

    fn temp_path(extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sozocraft-{}.{}", Uuid::new_v4(), extension))
    }
}
