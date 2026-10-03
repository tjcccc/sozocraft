use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::{fs::File, io::Read, path::Path};

pub const CREATE_ROUTE: &str = "/create-character-v3";
pub const ANIMATE_ROUTE: &str = "/characters/animations";
const MAX_REQUEST: u64 = 8 * 1024 * 1024;
const MAX_IMAGE: u64 = 2 * 1024 * 1024;

pub fn checked_id(id: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(id)
        .map(|id| id.to_string())
        .map_err(|_| "Expected a character/job UUID.".to_string())
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path).map_err(|_| "Cannot inspect PixelLab input file.")?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("PixelLab input must be a regular file within its size limit.".into());
    }
    let file = File::open(path).map_err(|_| "Cannot open PixelLab input file.")?;
    if !file
        .metadata()
        .map_err(|_| "Cannot inspect PixelLab input file.")?
        .is_file()
    {
        return Err("PixelLab inputs must be regular files.".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read PixelLab input file.")?;
    if bytes.len() as u64 > limit {
        return Err("PixelLab input file exceeds its size limit.".into());
    }
    Ok(bytes)
}

fn png_size(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() as u64 > MAX_IMAGE || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("PixelLab reference must be a PNG, at most 2 MiB and 256x256 pixels.".into());
    }
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(256);
    limits.max_image_height = Some(256);
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "Invalid PixelLab PNG or dimensions exceed 256x256.")?;
    if image.width() == 0 || image.height() == 0 {
        return Err("Empty PixelLab PNG.".into());
    }
    Ok((image.width(), image.height()))
}

fn encoded_size(value: &Value) -> Result<(u32, u32), String> {
    let text = value["base64"]
        .as_str()
        .ok_or("Expected a base64 PNG image.")?;
    let text = text.strip_prefix("data:image/png;base64,").unwrap_or(text);
    if text.len() > MAX_IMAGE as usize * 4 / 3 + 4 {
        return Err("PixelLab image exceeds 2 MiB.".into());
    }
    let bytes = STANDARD
        .decode(text)
        .map_err(|_| "Invalid PixelLab image base64.")?;
    if value.get("format").is_some_and(|v| v != "png") {
        return Err("PixelLab CLI images must use PNG format.".into());
    }
    png_size(&bytes)
}

pub fn load(path: &Path, reference: Option<&Path>, create: bool) -> Result<Value, String> {
    let mut value: Value = serde_json::from_slice(&read_file(path, MAX_REQUEST)?)
        .map_err(|_| "Request must contain valid JSON.")?;
    let object = value
        .as_object_mut()
        .ok_or("Request JSON must be an object.")?;
    if let Some(reference) = reference {
        if !create {
            return Err("--reference is only supported for character creation.".into());
        }
        if object.contains_key("reference_image") {
            return Err("Use --reference or reference_image, not both.".into());
        }
        let bytes = read_file(reference, MAX_IMAGE)?;
        png_size(&bytes)?;
        object.insert(
            "reference_image".into(),
            json!({"base64": STANDARD.encode(bytes)}),
        );
    }
    validate(&value, create)?;
    Ok(value)
}

pub fn mode(value: &Value) -> &str {
    value["mode"].as_str().unwrap_or_else(|| {
        if value["template_animation_id"].is_string() {
            "template"
        } else {
            "v3"
        }
    })
}

pub fn validate(value: &Value, create: bool) -> Result<(), String> {
    super::schema::validate(
        value,
        if create {
            "CreateCharacterV3Request"
        } else {
            "CreateCharacterAnimationRequest"
        },
    )?;
    if create {
        if value["description"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
        {
            return Err("Character description must be nonempty.".into());
        }
        if !value["reference_image"].is_null() {
            encoded_size(&value["reference_image"])?;
            if value["enhance_prompt"] == true {
                return Err("enhance_prompt requires creation without a reference.".into());
            }
        }
        return Ok(());
    }
    checked_id(
        value["character_id"]
            .as_str()
            .ok_or("Supply character_id.")?,
    )?;
    let mode = mode(value);
    let template = value["template_animation_id"]
        .as_str()
        .filter(|s| !s.trim().is_empty());
    if ["template", "skeleton-v3"].contains(&mode) && template.is_none() {
        return Err("Template and skeleton-v3 modes require template_animation_id.".into());
    }
    if ["v3", "pro"].contains(&mode)
        && (template.is_some()
            || value["action_description"]
                .as_str()
                .is_none_or(|s| s.trim().is_empty()))
    {
        return Err("Custom modes require action_description and no template_animation_id.".into());
    }
    if let Some(directions) = value["directions"].as_array() {
        let mut seen = std::collections::HashSet::new();
        if directions.is_empty()
            || directions.iter().any(|d| {
                let d = d.as_str().unwrap_or_default();
                ![
                    "south",
                    "north",
                    "east",
                    "west",
                    "south-east",
                    "south-west",
                    "north-east",
                    "north-west",
                ]
                .contains(&d)
                    || !seen.insert(d)
            })
        {
            return Err(
                "Directions must be a nonempty list of distinct compass directions.".into(),
            );
        }
    }
    if let Some(frames) = value["frame_count"].as_u64() {
        if frames % 2 != 0 {
            return Err("frame_count must be even (4-16).".into());
        }
    }
    let mut start = None;
    let mut end = None;
    for field in ["custom_start_frame", "end_frame", "color_image"] {
        if !value[field].is_null() {
            let size = encoded_size(&value[field])?;
            if field == "custom_start_frame" {
                start = Some(size);
            }
            if field == "end_frame" {
                end = Some(size);
            }
        }
    }
    if start.is_some() && end.is_some() && start != end {
        return Err("Start and end frames must have matching dimensions.".into());
    }
    let frame_input = start.is_some() || end.is_some();
    if mode != "v3"
        && (frame_input
            || value.get("keep_first_frame").is_some()
            || !value["frame_count"].is_null()
            || value["enhance_prompt"] == true)
    {
        return Err(
            "Frame inputs, frame_count, keep_first_frame and prompt enhancement require v3 mode."
                .into(),
        );
    }
    if frame_input && value["directions"].as_array().is_some_and(|d| d.len() != 1) {
        return Err("Custom frame inputs require exactly one direction (default south).".into());
    }
    Ok(())
}
