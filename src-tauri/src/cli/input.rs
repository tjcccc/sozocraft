use crate::{file_access, models::ReferenceImageInput};
use std::{io::Read, path::Path};

pub(super) fn read_prompt(reader: impl Read) -> Result<String, String> {
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

pub(super) fn read_image(path: &str) -> Result<ReferenceImageInput, String> {
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
