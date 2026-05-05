use crate::{local_config, models::GenerationRequest};
use base64::{engine::general_purpose, Engine as _};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, ColorType, DynamicImage};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const CACHE_VERSION: &str = "v1";
const JPEG_QUALITY: u8 = 85;
const MAX_LONG_EDGE: u32 = 1600;
const MIN_BYTES_TO_OPTIMIZE: usize = 300 * 1024;
const CACHE_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

pub fn optimize_request_reference_images(request: &mut GenerationRequest) -> Result<(), String> {
    cleanup_old_cache_files();

    let Some(reference_images) = request.reference_images.as_mut() else {
        return Ok(());
    };

    for image in reference_images {
        let Some(optimized) = optimize_reference_image(&image.mime_type, &image.data)? else {
            continue;
        };
        image.mime_type = "image/jpeg".to_string();
        image.name = jpeg_name(&image.name);
        image.data = general_purpose::STANDARD.encode(&optimized);
    }

    Ok(())
}

fn optimize_reference_image(mime_type: &str, data: &str) -> Result<Option<Vec<u8>>, String> {
    if !is_jpeg_or_png(mime_type) {
        return Ok(None);
    }

    let original = general_purpose::STANDARD
        .decode(data.trim())
        .map_err(|err| format!("Failed to decode reference image: {err}"))?;

    if original.len() < MIN_BYTES_TO_OPTIMIZE {
        return Ok(None);
    }

    let cache_path = cache_file_path(&original);
    if let Ok(cached) = fs::read(&cache_path) {
        return Ok(Some(cached));
    }

    let image = image::load_from_memory(&original)
        .map_err(|err| format!("Failed to read reference image for compression: {err}"))?;
    if has_alpha(&image) {
        return Ok(None);
    }

    let (width, height) = dimensions(&image);
    let resized = if width.max(height) > MAX_LONG_EDGE {
        image.resize(MAX_LONG_EDGE, MAX_LONG_EDGE, FilterType::Lanczos3)
    } else {
        image
    };

    let mut optimized = Vec::new();
    let rgb = resized.to_rgb8();
    let mut encoder = JpegEncoder::new_with_quality(&mut optimized, JPEG_QUALITY);
    encoder
        .encode(&rgb, rgb.width(), rgb.height(), ColorType::Rgb8.into())
        .map_err(|err| format!("Failed to encode compressed reference image: {err}"))?;

    if optimized.len() >= original.len() {
        return Ok(None);
    }

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create reference image cache: {err}"))?;
    }
    fs::write(&cache_path, &optimized)
        .map_err(|err| format!("Failed to write reference image cache: {err}"))?;

    Ok(Some(optimized))
}

fn is_jpeg_or_png(mime_type: &str) -> bool {
    matches!(
        mime_type.to_ascii_lowercase().as_str(),
        "image/png" | "image/jpeg" | "image/jpg"
    )
}

fn has_alpha(image: &DynamicImage) -> bool {
    matches!(
        image.color(),
        ColorType::La8
            | ColorType::La16
            | ColorType::Rgba8
            | ColorType::Rgba16
            | ColorType::Rgba32F
    )
}

fn dimensions(image: &DynamicImage) -> (u32, u32) {
    (image.width(), image.height())
}

fn cache_file_path(original: &[u8]) -> PathBuf {
    cache_dir().join(format!(
        "{CACHE_VERSION}-q{JPEG_QUALITY}-max{MAX_LONG_EDGE}-{}-{:016x}.jpg",
        original.len(),
        fnv1a64(original)
    ))
}

fn cache_dir() -> PathBuf {
    local_config::config_dir()
        .join("cache")
        .join("reference-images")
}

fn jpeg_name(name: &str) -> String {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("reference");
    format!("{stem}.jpg")
}

fn cleanup_old_cache_files() {
    let Ok(entries) = fs::read_dir(cache_dir()) else {
        return;
    };
    let now = SystemTime::now();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jpg") {
            continue;
        }
        let should_remove = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .map(|age| age > CACHE_TTL)
            .unwrap_or(false);
        if should_remove {
            let _ = fs::remove_file(path);
        }
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReferenceImageInput;

    #[test]
    fn skips_webp_without_error() {
        let mut request = request_with_reference("image/webp", "abc");

        optimize_request_reference_images(&mut request).unwrap();

        let image = request.reference_images.unwrap().remove(0);
        assert_eq!(image.mime_type, "image/webp");
        assert_eq!(image.data, "abc");
    }

    #[test]
    fn cache_key_changes_with_content() {
        let first = cache_file_path(b"first");
        let second = cache_file_path(b"second");

        assert_ne!(first, second);
    }

    fn request_with_reference(mime_type: &str, data: &str) -> GenerationRequest {
        GenerationRequest {
            task_id: None,
            provider: "nano-banana".to_string(),
            model: "gemini-3-pro-image-preview".to_string(),
            prompt: "Render a test image".to_string(),
            prompt_snapshot: None,
            batch_count: 1,
            reference_images: Some(vec![ReferenceImageInput {
                name: "reference.webp".to_string(),
                mime_type: mime_type.to_string(),
                data: data.to_string(),
            }]),
            output_template: "{id}.{extension}".to_string(),
            options: Default::default(),
            base_url: None,
        }
    }
}
