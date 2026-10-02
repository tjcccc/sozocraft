use crate::{
    models::{AppSettings, VideoProvider},
    video_provider,
};

pub(super) fn provider(model: &str) -> Result<VideoProvider, String> {
    match model {
        "veo-3.1-generate-preview" | "veo-3.1-lite-generate-preview" | "gemini-omni-1.1-flash" => {
            Ok(VideoProvider::GoogleVeo)
        }
        "doubao-seedance-2-0-260128"
        | "doubao-seedance-2-0-fast-260128"
        | "doubao-seedance-2-0-mini-260615"
        | "doubao-seedance-2-5-260628" => Ok(VideoProvider::Seedance),
        "grok-imagine-video-1.5" => Ok(VideoProvider::GrokImagine),
        _ => Err(
            "Unsupported video model. See docs/agent-cli.md for supported model IDs.".to_string(),
        ),
    }
}

pub(super) fn max_references(model: &str) -> usize {
    match model {
        "gemini-omni-1.1-flash" => 6,
        "grok-imagine-video-1.5" => 7,
        "doubao-seedance-2-5-260628" => 30,
        value if value.starts_with("doubao-seedance-") => 9,
        _ => 3,
    }
}

pub(super) fn validate_platform(model: &str, platform: &str) -> Result<(), String> {
    let supported = match provider(model)? {
        VideoProvider::GoogleVeo => platform == "gemini",
        VideoProvider::GrokImagine => platform == "xai",
        VideoProvider::Seedance => ["ark", "higgsfield"].contains(&platform),
    };
    if supported {
        Ok(())
    } else {
        Err("Video model does not support the selected --platform.".to_string())
    }
}

pub(super) fn apply_platform(
    settings: &mut AppSettings,
    model: &str,
    platform: Option<&str>,
) -> Result<(), String> {
    if let Some(platform) = platform {
        validate_platform(model, platform)?;
        if provider(model)? == VideoProvider::Seedance {
            settings.seedance_api_platform = platform.to_string();
        }
    }
    Ok(())
}

pub(super) fn platform(model: &str, settings: &AppSettings) -> Result<&'static str, String> {
    Ok(match provider(model)? {
        VideoProvider::Seedance if settings.seedance_api_platform == "higgsfield" => "higgsfield",
        VideoProvider::Seedance => "ark",
        VideoProvider::GrokImagine => "xai",
        VideoProvider::GoogleVeo => "gemini",
    })
}

pub(super) fn validate_operation(model: &str, operation: &str) -> Result<(), String> {
    match provider(model)? {
        VideoProvider::Seedance => crate::seedance_video::validate_task_id(operation),
        VideoProvider::GrokImagine => {
            crate::xai_video::validate_request_id(operation).map_err(|error| error.to_string())
        }
        VideoProvider::GoogleVeo if model == "gemini-omni-1.1-flash" => {
            crate::google_omni::validate_file_id(operation)
        }
        VideoProvider::GoogleVeo => {
            crate::google_veo::validate_operation_name(operation)?;
            if !operation.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.')
            }) {
                return Err("Invalid characters in Veo operation name.".to_string());
            }
            if !operation.starts_with(&format!("models/{model}/operations/"))
                && !operation.starts_with("operations/")
            {
                return Err("Expected a Veo operation name returned by video generate.".to_string());
            }
            Ok(())
        }
    }
}

pub(super) fn load_key(model: &str, settings: &AppSettings) -> Result<String, String> {
    video_provider::load_key(provider(model)?, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_overrides_are_scoped_and_operation_ids_are_safe() {
        let mut settings = AppSettings::default();
        apply_platform(
            &mut settings,
            "doubao-seedance-2-0-260128",
            Some("higgsfield"),
        )
        .unwrap();
        assert_eq!(
            platform("doubao-seedance-2-0-260128", &settings).unwrap(),
            "higgsfield"
        );
        assert!(
            apply_platform(&mut settings, "grok-imagine-video-1.5", Some("higgsfield")).is_err()
        );
        for model in ["doubao-seedance-2-0-260128", "grok-imagine-video-1.5"] {
            assert!(validate_operation(model, "safe-id_123").is_ok());
            for operation in ["../secret", "bad/id", "%2e%2e", "a?key=secret"] {
                assert!(validate_operation(model, operation).is_err());
            }
        }
        assert_eq!(max_references("doubao-seedance-2-5-260628"), 30);
    }
}
