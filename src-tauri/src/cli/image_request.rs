use super::{
    image_arguments::GenerateImage,
    input::{read_image, read_prompt},
};
use crate::{
    experimental_image, file_access,
    models::{
        AppSettings, GenerationOptions, GenerationRequest, NANO_BANANA_MODELS, OPENAI_IMAGE_MODELS,
        XAI_IMAGE_MODELS,
    },
};
use base64::{engine::general_purpose, Engine as _};
use std::io::Cursor;

pub(super) fn build(
    options: &GenerateImage,
    settings: &AppSettings,
) -> Result<GenerationRequest, String> {
    let inferred = options.model.as_deref().map(model_provider).transpose()?;
    let provider = options
        .provider
        .as_deref()
        .or(inferred)
        .unwrap_or(&settings.default_provider);
    if inferred.is_some_and(|value| value != provider) {
        return Err("Model does not belong to the selected image provider.".to_string());
    }
    let platform =
        match provider {
            "nano-banana" => &settings.nano_banana_api_platform,
            "gpt-image" => &settings.openai_api_platform,
            "grok-imagine" => &settings.grok_api_platform,
            experimental_image::PROVIDER_ID => experimental_image::PLATFORM,
            _ => return Err(
                "Supported image providers: nano-banana, gpt-image, grok-imagine, experimental."
                    .to_string(),
            ),
        };
    let fallback = match (provider, platform) {
        ("nano-banana", "higgsfield") => "nano_banana_2",
        ("nano-banana", _) => "gemini-3-pro-image-preview",
        ("gpt-image", "higgsfield") => "gpt_image_2",
        ("gpt-image", "openrouter") => "openai/gpt-image-2",
        ("gpt-image", _) => "gpt-image-2",
        (experimental_image::PROVIDER_ID, _) => experimental_image::DEFAULT_MODEL,
        (_, "higgsfield") => "grok_image",
        _ => "grok-imagine-image-2.0",
    };
    let model = options.model.as_deref().unwrap_or_else(|| {
        if provider == settings.default_provider {
            &settings.default_model
        } else {
            fallback
        }
    });
    let higgsfield_model = model.contains('_');
    if higgsfield_model != (platform == "higgsfield")
        || (model.starts_with("openai/") && platform != "openrouter")
    {
        return Err(format!("Model {model} does not match the configured {platform} platform. Select a model for that platform or change the desktop settings."));
    }
    let prompt = match (&options.prompt, &options.prompt_file) {
        (Some(prompt), None) => prompt.clone(),
        (None, Some(path)) if path == "-" => read_prompt(std::io::stdin().lock())?,
        (None, Some(path)) => file_access::read_text_file(path)?,
        _ => return Err("Supply exactly one prompt source.".to_string()),
    };
    if prompt.trim().is_empty() || prompt.len() > 50_000 {
        return Err("Image prompt must be nonempty and at most 50,000 UTF-8 bytes.".to_string());
    }
    let request = GenerationRequest {
        task_id: None,
        provider: provider.to_string(),
        model: model.to_string(),
        prompt,
        prompt_snapshot: None,
        batch_count: 1,
        reference_images: Some(
            options
                .references
                .iter()
                .map(|path| read_image(path))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        output_template: settings.output_template.clone(),
        base_url: None,
        options: GenerationOptions {
            aspect_ratio: options.aspect_ratio.clone(),
            image_size: options.size.clone(),
            quality: options.quality.clone(),
            thinking_level: options.thinking_level.clone(),
            temperature: None,
            top_p: None,
            unlimited: None,
        },
    };
    request.validate()?;
    validate_options(&request, platform)?;
    for reference in request.reference_images.as_deref().unwrap_or_default() {
        if !["image/png", "image/jpeg"].contains(&reference.mime_type.as_str()) {
            return Err("CLI image references must be PNG or JPEG.".to_string());
        }
        let bytes = general_purpose::STANDARD
            .decode(&reference.data)
            .map_err(|_| "Invalid reference encoding.")?;
        let format = image::guess_format(&bytes).map_err(|_| "Invalid reference image.")?;
        if (reference.mime_type == "image/png" && format != image::ImageFormat::Png)
            || (reference.mime_type == "image/jpeg" && format != image::ImageFormat::Jpeg)
        {
            return Err("Reference image data does not match its file type.".to_string());
        }
        decode_image(&bytes)?;
    }
    Ok(request)
}

fn model_provider(model: &str) -> Result<&'static str, String> {
    if NANO_BANANA_MODELS.contains(&model) {
        Ok("nano-banana")
    } else if OPENAI_IMAGE_MODELS.contains(&model) {
        Ok("gpt-image")
    } else if XAI_IMAGE_MODELS.contains(&model) {
        Ok("grok-imagine")
    } else if experimental_image::model_ids().any(|id| id == model) {
        Ok(experimental_image::PROVIDER_ID)
    } else {
        Err("Unsupported image model. See docs/agent-cli.md.".to_string())
    }
}

fn validate_options(request: &GenerationRequest, platform: &str) -> Result<(), String> {
    if request.provider == experimental_image::PROVIDER_ID {
        // GenerationRequest::validate already applied the experimental model table.
        return experimental_image::validate_request(request);
    }
    let options = &request.options;
    let model = request.model.as_str();
    if platform == "higgsfield" {
        crate::higgsfield::validate_request_options(request)?;
    } else if let Some(ratio) = options.aspect_ratio.as_deref() {
        let valid = match request.provider.as_str() {
            "nano-banana" => {
                ratio == "auto" || crate::gemini::supported_aspect_ratio(request).is_some()
            }
            "gpt-image" if platform == "openrouter" => [
                "auto", "1:1", "3:2", "2:3", "4:3", "3:4", "16:9", "9:16", "21:9",
            ]
            .contains(&ratio),
            "gpt-image" => false,
            _ => [
                "auto", "1:1", "3:2", "2:3", "4:3", "3:4", "4:5", "5:4", "9:16", "16:9", "21:9",
                "1:2", "2:1",
            ]
            .contains(&ratio),
        };
        if !valid {
            return Err("Unsupported --aspect-ratio for this model/platform; native GPT images use --size WIDTHxHEIGHT.".to_string());
        }
    }
    if let Some(size) = options.image_size.as_deref() {
        let valid = match (request.provider.as_str(), platform) {
            (_, "higgsfield") => {
                model != "nano_banana"
                    && model != "grok_image"
                    && ["1k", "2k", "4k"].contains(&size)
            }
            ("nano-banana", _) => crate::gemini::supported_image_size(request).is_some(),
            ("gpt-image", "openrouter") if model.starts_with("openai/") => false,
            ("gpt-image", _) => crate::openai_image::normalize_openai_image_size(size).is_some(),
            _ => ["1k", "2k"].contains(&size),
        };
        if !valid {
            return Err("Unsupported --size for this model/platform.".to_string());
        }
    }
    if options.thinking_level.is_some()
        && crate::gemini::supported_thinking_level(request).is_none()
    {
        return Err("--thinking-level requires Gemini Flash 3.1 and minimal or high.".to_string());
    }
    if let Some(quality) = options.quality.as_deref() {
        let valid = if request.provider == "nano-banana" {
            false
        } else if model == "grok_image" {
            ["std", "pro"].contains(&quality)
        } else if model == "gpt_image_2" {
            ["low", "medium", "high"].contains(&quality)
        } else if model.starts_with("gpt_image_") {
            ["low", "medium", "high", "xhigh", "max"].contains(&quality)
        } else {
            true
        };
        if !valid {
            return Err("Unsupported --quality for this model/platform.".to_string());
        }
    }
    Ok(())
}

pub(super) fn decode_image(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    if bytes.len() > 100 * 1024 * 1024 {
        return Err("Image exceeds the 100 MiB limit.".to_string());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "Invalid image data.")?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err("Image must contain PNG or JPEG data.".to_string());
    }
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| "Image is corrupt or exceeds decoding limits.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(model: &str) -> GenerateImage {
        GenerateImage {
            model: Some(model.to_string()),
            prompt: Some("pixel sprite".to_string()),
            ..Default::default()
        }
    }
    #[test]
    fn respects_model_platform_and_provider_boundaries() {
        let mut settings = AppSettings::default();
        let mut value = options("gemini-3.1-flash-image-preview");
        value.size = Some("512".to_string());
        value.thinking_level = Some("minimal".to_string());
        assert!(build(&value, &settings).is_ok());
        value.model = Some("gemini-3-pro-image-preview".to_string());
        assert!(build(&value, &settings).is_err());
        settings.nano_banana_api_platform = "higgsfield".to_string();
        assert!(build(&value, &settings).is_err());
        let mut value = options("gpt-image-2");
        assert_eq!(build(&value, &settings).unwrap().provider, "gpt-image");
        value.provider = Some("nano-banana".to_string());
        assert!(build(&value, &settings).is_err());
        value.provider = None;
        value.size = Some("1536x1024".to_string());
        assert!(build(&value, &settings).is_ok());
        value.aspect_ratio = Some("16:9".to_string());
        assert!(build(&value, &settings).is_err());
        value.aspect_ratio = None;
        value.size = Some("1K".to_string());
        assert!(build(&value, &settings).is_err());
        settings.openai_api_platform = "openrouter".to_string();
        value = options("openai/gpt-image-2");
        value.size = Some("1536x1024".to_string());
        assert!(build(&value, &settings).is_err());
        value.size = None;
        value.aspect_ratio = Some("16:9".to_string());
        assert!(build(&value, &settings).is_ok());
        value = options("grok-imagine-image-2.0");
        value.quality = Some("high".to_string());
        assert!(build(&value, &settings).is_err());
        value.quality = Some("medium".to_string());
        value.size = Some("2k".to_string());
        assert!(build(&value, &settings).is_ok());
    }
    #[test]
    fn experimental_models_route_to_openrouter_and_reject_foreign_options() {
        // Other providers' platforms must not affect the experimental route.
        let mut settings = AppSettings::default();
        settings.grok_api_platform = "higgsfield".to_string();
        let mut value = options("meta/muse-image");
        let request = build(&value, &settings).unwrap();
        assert_eq!(request.provider, "experimental");
        assert_eq!(
            crate::generation_platform(&request, &settings),
            "openrouter"
        );

        value.model = None;
        value.provider = Some("experimental".to_string());
        assert_eq!(build(&value, &settings).unwrap().model, "meta/muse-image");

        value.aspect_ratio = Some("9:21".to_string());
        assert!(build(&value, &settings).is_ok());
        value.aspect_ratio = Some("auto".to_string());
        assert!(build(&value, &settings).is_ok());
        value.aspect_ratio = Some("5:4".to_string());
        assert!(build(&value, &settings).is_err());
        value.aspect_ratio = None;
        value.size = Some("2k".to_string());
        assert!(build(&value, &settings).is_err());
        value.size = None;
        value.quality = Some("high".to_string());
        assert!(build(&value, &settings).is_err());
        value.quality = None;
        value.provider = Some("gpt-image".to_string());
        value.model = Some("meta/muse-image".to_string());
        assert!(build(&value, &settings).is_err());
    }
    #[test]
    fn rejects_corrupt_references_and_excess_model_reference_count() {
        let dir =
            std::env::temp_dir().join(format!("sozocraft-cli-image-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("sprite.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
        let mut value = options("gemini-2.5-flash-image");
        value.references.push(path.to_str().unwrap().to_string());
        assert!(build(&value, &AppSettings::default())
            .unwrap_err()
            .contains("corrupt"));
        value.references = vec![path.to_str().unwrap().to_string(); 4];
        assert!(build(&value, &AppSettings::default())
            .unwrap_err()
            .contains("at most 3"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
