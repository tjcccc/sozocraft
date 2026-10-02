use crate::{
    google_omni::{GoogleOmniClient, OmniStatus},
    google_veo::{GoogleVeoClient, GoogleVeoStatus},
    higgsfield_video::{HiggsfieldVideoClient, HiggsfieldVideoStatus},
    local_config,
    models::{AppSettings, VideoGenerationRequest, VideoProvider},
    seedance_video::{SeedanceVideoClient, SeedanceVideoStatus},
    xai_video::{XaiVideoClient, XaiVideoStatus},
};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) enum ProviderClient {
    Seedance(SeedanceVideoClient),
    Higgsfield(HiggsfieldVideoClient),
    Grok(XaiVideoClient),
    Veo(GoogleVeoClient),
    Omni(GoogleOmniClient),
}

pub(crate) enum PollStatus {
    Pending,
    Done { url: String, duration: Option<u8> },
    Failed(String),
}

pub(crate) struct PollResponse {
    pub(crate) status: PollStatus,
    pub(crate) metadata: Value,
}

#[derive(Clone)]
pub(crate) struct ProviderRuntime {
    pub(crate) base_url: Option<String>,
    pub(crate) proxy_url: Option<String>,
    pub(crate) timeout_seconds: u64,
    pub(crate) higgsfield_cli_path: Option<String>,
}

impl ProviderClient {
    pub(crate) fn is_higgsfield(&self) -> bool {
        matches!(self, Self::Higgsfield(_))
    }

    pub(crate) async fn start(&self, request: &VideoGenerationRequest) -> Result<String, String> {
        match self {
            Self::Seedance(client) => client.start(request).await,
            Self::Higgsfield(client) => client.start(request).await,
            Self::Grok(client) => client
                .start(request)
                .await
                .map_err(|error| error.to_string()),
            Self::Veo(client) => client.start(request).await,
            Self::Omni(client) => client.start(request).await,
        }
    }

    pub(crate) async fn poll(&self, request_id: &str) -> Result<PollResponse, String> {
        match self {
            Self::Seedance(client) => {
                let response = client.poll(request_id).await?;
                let status = match response.status {
                    SeedanceVideoStatus::Pending => PollStatus::Pending,
                    SeedanceVideoStatus::Done { url, duration } => {
                        PollStatus::Done { url, duration }
                    }
                    SeedanceVideoStatus::Failed(message) => PollStatus::Failed(message),
                    SeedanceVideoStatus::Cancelled => PollStatus::Failed(
                        "Volcengine Ark cancelled the Seedance task.".to_string(),
                    ),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
            Self::Higgsfield(client) => {
                let response = client.poll(request_id).await?;
                let status = match response.status {
                    HiggsfieldVideoStatus::Pending => PollStatus::Pending,
                    HiggsfieldVideoStatus::Done { url } => PollStatus::Done {
                        url,
                        duration: None,
                    },
                    HiggsfieldVideoStatus::Failed(message) => PollStatus::Failed(message),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
            Self::Grok(client) => {
                let response = client
                    .poll(request_id)
                    .await
                    .map_err(|error| error.to_string())?;
                let status = match response.status {
                    XaiVideoStatus::Pending => PollStatus::Pending,
                    XaiVideoStatus::Done { url, duration } => PollStatus::Done { url, duration },
                    XaiVideoStatus::Failed(message) => PollStatus::Failed(message),
                    XaiVideoStatus::Expired => PollStatus::Failed(
                        "xAI video request expired before completion.".to_string(),
                    ),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
            Self::Omni(client) => {
                let status = match client.poll(request_id).await? {
                    OmniStatus::Pending => PollStatus::Pending,
                    OmniStatus::Done(url) => PollStatus::Done {
                        url,
                        duration: None,
                    },
                    OmniStatus::Failed => {
                        PollStatus::Failed("Gemini Omni video processing failed.".to_string())
                    }
                };
                Ok(PollResponse {
                    status,
                    metadata: json!({"fileId": request_id}),
                })
            }
            Self::Veo(client) => {
                let response = client.poll(request_id).await?;
                let status = match response.status {
                    GoogleVeoStatus::Pending => PollStatus::Pending,
                    GoogleVeoStatus::Done { url } => PollStatus::Done {
                        url,
                        duration: None,
                    },
                    GoogleVeoStatus::Failed(message) => PollStatus::Failed(message),
                };
                Ok(PollResponse {
                    status,
                    metadata: response.metadata,
                })
            }
        }
    }

    pub(crate) async fn download_to(
        &self,
        video_url: &str,
        output_path: &Path,
    ) -> Result<u64, String> {
        match self {
            Self::Seedance(client) => client.download_to(video_url, output_path).await,
            Self::Higgsfield(client) => client.download_to(video_url, output_path).await,
            Self::Grok(client) => client
                .download_to(video_url, output_path)
                .await
                .map_err(|error| error.to_string()),
            Self::Veo(client) => client.download_to(video_url, output_path).await,
            Self::Omni(client) => client.download_to(video_url, output_path).await,
        }
    }

    pub(crate) fn platform(&self) -> &'static str {
        match self {
            Self::Higgsfield(_) => "higgsfield",
            Self::Seedance(_) => "volcengine-ark",
            Self::Grok(_) => "xai",
            Self::Veo(_) | Self::Omni(_) => "google-gemini-api",
        }
    }

    pub(crate) fn filename_provider(&self) -> &'static str {
        match self {
            Self::Higgsfield(_) => "higgsfield",
            Self::Seedance(_) => "volcengine",
            Self::Grok(_) => "xai",
            Self::Veo(_) | Self::Omni(_) => "google",
        }
    }
}

pub(crate) fn provider_runtime(settings: &AppSettings, provider: VideoProvider) -> ProviderRuntime {
    match provider {
        VideoProvider::Seedance => ProviderRuntime {
            base_url: (settings.seedance_api_platform != "higgsfield")
                .then(|| settings.ark_base_url.clone())
                .flatten(),
            proxy_url: if settings.seedance_api_platform == "higgsfield" {
                settings
                    .effective_higgsfield_proxy_url()
                    .map(str::to_string)
            } else {
                settings
                    .ark_proxy_enabled
                    .then(|| settings.proxy_url.clone())
                    .flatten()
            },
            timeout_seconds: settings.ark_timeout_seconds,
            higgsfield_cli_path: settings.higgsfield_cli_path.clone(),
        },
        VideoProvider::GrokImagine => ProviderRuntime {
            base_url: settings.xai_base_url.clone(),
            proxy_url: settings
                .xai_proxy_enabled
                .then(|| settings.proxy_url.clone())
                .flatten(),
            timeout_seconds: settings.xai_timeout_seconds,
            higgsfield_cli_path: None,
        },
        VideoProvider::GoogleVeo => ProviderRuntime {
            base_url: settings.optional_base_url.clone(),
            proxy_url: settings
                .gemini_proxy_enabled
                .then(|| settings.proxy_url.clone())
                .flatten(),
            timeout_seconds: settings.gemini_timeout_seconds,
            higgsfield_cli_path: None,
        },
    }
}

pub(crate) fn create_client_with_key(
    provider: VideoProvider,
    model: &str,
    runtime: &ProviderRuntime,
    settings: &AppSettings,
    key: &str,
) -> Result<ProviderClient, String> {
    match provider {
        VideoProvider::Seedance if settings.seedance_api_platform == "higgsfield" => {
            Ok(ProviderClient::Higgsfield(HiggsfieldVideoClient::new(
                runtime.higgsfield_cli_path.clone(),
                runtime.proxy_url.clone(),
                runtime.timeout_seconds,
            )?))
        }
        VideoProvider::Seedance => Ok(ProviderClient::Seedance(SeedanceVideoClient::new(
            key.to_string(),
            runtime.base_url.clone(),
            runtime.proxy_url.clone(),
            runtime.timeout_seconds,
        )?)),
        VideoProvider::GrokImagine => Ok(ProviderClient::Grok(
            XaiVideoClient::new(
                key.to_string(),
                runtime.base_url.clone(),
                runtime.proxy_url.clone(),
                runtime.timeout_seconds,
            )
            .map_err(|error| error.to_string())?,
        )),
        VideoProvider::GoogleVeo if model == "gemini-omni-1.1-flash" => {
            Ok(ProviderClient::Omni(GoogleOmniClient::new(
                key.to_string(),
                runtime.base_url.clone(),
                runtime.proxy_url.clone(),
                runtime.timeout_seconds,
            )?))
        }
        VideoProvider::GoogleVeo => Ok(ProviderClient::Veo(GoogleVeoClient::new(
            key.to_string(),
            runtime.base_url.clone(),
            runtime.proxy_url.clone(),
            runtime.timeout_seconds,
        )?)),
    }
}

pub(crate) fn load_key(provider: VideoProvider, settings: &AppSettings) -> Result<String, String> {
    let (platform, result) = match provider {
        VideoProvider::Seedance if settings.seedance_api_platform == "higgsfield" => {
            return Ok(String::new())
        }
        VideoProvider::Seedance => ("Ark", local_config::get_ark_api_key()),
        VideoProvider::GrokImagine => ("xAI", local_config::get_xai_api_key()),
        VideoProvider::GoogleVeo => ("Gemini", local_config::get_gemini_api_key()),
    };
    result.map_err(|_| format!("Could not load {platform} API key from ~/.sozocraft/config.toml. Configure it in SozoCraft and check that the file is valid."))
}

pub(crate) fn create_client(
    provider: VideoProvider,
    model: &str,
    runtime: &ProviderRuntime,
    settings: &AppSettings,
) -> Result<ProviderClient, String> {
    let key = load_key(provider, settings)?;
    create_client_with_key(provider, model, runtime, settings, &key)
}
