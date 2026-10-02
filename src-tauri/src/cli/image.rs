use super::{
    emit,
    image_arguments::GenerateImage,
    image_request,
    output::{validate_output, Output},
};
use crate::{
    filename_template::resolve_output_path,
    local_config,
    models::{AppSettings, GenerationRequest},
};
use chrono::Local;
use serde_json::json;
use std::{fs, io::Cursor, path::PathBuf};
use uuid::Uuid;

pub(super) async fn execute(options: GenerateImage) -> Result<(), String> {
    let settings = local_config::load_settings(AppSettings::default());
    let request = image_request::build(&options, &settings)?;
    if let Some(path) = &options.output {
        validate_output(path, "png")?;
    }
    let platform = crate::generation_platform(&request, &settings);
    if options.dry_run {
        return emit(
            &json!({"status":"validated", "mediaType":"image", "provider":request.provider,
            "model":request.model, "platform":platform, "options":request.options,
            "inputImageCount":request.reference_images.as_ref().map_or(0, Vec::len), "count":1}),
        );
    }
    let key = load_key(&request, &platform)?;
    let request_id = Uuid::new_v4().to_string();
    let timestamp = Local::now();
    let path = output_path(&options, &request, &settings, &request_id, timestamp, 1)?;
    let first_output = Output::for_path(path, "png")?;
    // This local ID records the attempt; it is not a resumable provider operation.
    emit(
        &json!({"status":"started", "mediaType":"image", "provider":request.provider,
        "model":request.model, "platform":platform, "requestId":request_id}),
    )?;
    let response = crate::generate_with_provider(&request, &settings)
        .await
        .map_err(|error| sanitize_error(&error, &key, &request, &settings))?;
    if response.images.is_empty() {
        return Err("Image provider returned no image data. Check the provider response or account before retrying.".to_string());
    }
    let mut first_output = Some(first_output);
    let mut paths = Vec::new();
    for (index, image) in response.images.into_iter().enumerate() {
        let decoded = image_request::decode_image(&image.bytes)?;
        let (width, height) = (decoded.width(), decoded.height());
        let mut png = Vec::new();
        decoded
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|_| "Cannot encode generated PNG.")?;
        let output = if let Some(output) = first_output.take() {
            output
        } else {
            Output::for_path(
                output_path(
                    &options,
                    &request,
                    &settings,
                    &request_id,
                    timestamp,
                    index + 1,
                )?,
                "png",
            )?
        };
        fs::write(&output.partial, &png).map_err(|_| "Cannot write generated PNG.")?;
        output.publish()?;
        emit(
            &json!({"status":"image", "mediaType":"image", "provider":request.provider,
            "model":request.model, "requestId":request_id, "index":index+1,
            "outputPath":output.final_path, "bytes":png.len(), "width":width, "height":height}),
        )?;
        paths.push(output.final_path.clone());
    }
    emit(
        &json!({"status":"completed", "mediaType":"image", "provider":request.provider,
        "model":request.model, "platform":platform, "requestId":request_id,
        "outputPaths":paths, "imageCount":paths.len()}),
    )
}

fn load_key(request: &GenerationRequest, platform: &str) -> Result<String, String> {
    let result = match platform {
        "gemini" => local_config::get_gemini_api_key(),
        "openai" => local_config::get_openai_api_key(),
        "openrouter" => local_config::get_openrouter_api_key(),
        "xai" => local_config::get_xai_api_key(),
        "higgsfield" => return Ok(String::new()),
        _ => return Err("Unsupported configured image platform.".to_string()),
    };
    result.map_err(|_| format!("Could not load {platform} API key for {} from ~/.sozocraft/config.toml. Configure it in SozoCraft.", request.provider))
}

fn output_path(
    options: &GenerateImage,
    request: &GenerationRequest,
    settings: &AppSettings,
    request_id: &str,
    timestamp: chrono::DateTime<Local>,
    index: usize,
) -> Result<PathBuf, String> {
    if let Some(path) = &options.output {
        if index == 1 {
            return Ok(path.clone());
        }
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or("Invalid output filename.")?;
        return Ok(path.with_file_name(format!("{stem}_{index:03}.png")));
    }
    resolve_output_path(
        &settings.output_directory,
        &settings.output_template,
        crate::filename_provider(request, settings),
        crate::filename_model(&request.model),
        &format!("{index:03}"),
        &request_id[..6],
        "png",
        timestamp,
    )
    .map_err(|error| error.to_string())
}

fn sanitize_error(
    error: &str,
    key: &str,
    request: &GenerationRequest,
    settings: &AppSettings,
) -> String {
    let mut message = error.to_string();
    let mut secrets = vec![key, request.prompt.as_str()];
    secrets.extend(
        request
            .reference_images
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|image| image.data.as_str()),
    );
    secrets.extend(
        [
            settings.proxy_url.as_deref(),
            settings.higgsfield_proxy_url.as_deref(),
            crate::effective_base_url(&request.provider, settings),
        ]
        .into_iter()
        .flatten(),
    );
    for secret in secrets.into_iter().filter(|value| !value.is_empty()) {
        message = message.replace(secret, "[redacted]");
        if let Ok(escaped) = serde_json::to_string(secret) {
            message = message.replace(&escaped[1..escaped.len() - 1], "[redacted]");
        }
    }
    message.chars().take(2000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_names_follow_desktop_template_and_preserve_explicit_names() {
        let settings = AppSettings {
            output_directory: "images".to_string(),
            output_template: "{yyMMdd}/{provider}_{model}_{batch_id}_{id}".to_string(),
            ..AppSettings::default()
        };
        let mut options = GenerateImage {
            model: Some("gemini-3.1-flash-image-preview".to_string()),
            prompt: Some("sprite".to_string()),
            ..Default::default()
        };
        let request = image_request::build(&options, &settings).unwrap();
        let timestamp = Local::now();
        let path = output_path(
            &options,
            &request,
            &settings,
            "abcdef-local-id",
            timestamp,
            1,
        )
        .unwrap();
        assert!(path.ends_with("gemini_nano-banana-2_abcdef_001.png"));
        assert_eq!(
            path.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            timestamp.format("%y%m%d").to_string()
        );
        options.output = Some(PathBuf::from("images/contact.png"));
        assert_eq!(
            output_path(
                &options,
                &request,
                &settings,
                "abcdef-local-id",
                timestamp,
                1
            )
            .unwrap(),
            PathBuf::from("images/contact.png")
        );
        assert_eq!(
            output_path(
                &options,
                &request,
                &settings,
                "abcdef-local-id",
                timestamp,
                2
            )
            .unwrap(),
            PathBuf::from("images/contact_002.png")
        );
    }

    #[test]
    fn diagnostics_redact_before_truncating() {
        let settings = AppSettings::default();
        let options = GenerateImage {
            prompt: Some("private\nprompt".to_string()),
            ..Default::default()
        };
        let request = image_request::build(&options, &settings).unwrap();
        let message = sanitize_error("secret private\\nprompt", "secret", &request, &settings);
        assert_eq!(message, "[redacted] [redacted]");
    }
}
