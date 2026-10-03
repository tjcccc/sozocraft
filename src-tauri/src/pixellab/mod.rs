//! PixelLab REST adapter for the headless CLI. No desktop state or Python runtime.
pub(crate) mod request;
mod schema;
#[cfg(test)]
mod tests;

use reqwest::{Client as HttpClient, Method};
use serde_json::Value;
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

pub const BASE_URL: &str = "https://api.pixellab.ai/v2";
const MAX_JSON: usize = 8 * 1024 * 1024;
const MAX_ZIP: u64 = 512 * 1024 * 1024;
const NETWORK_ERROR: &str = "PixelLab network/TLS request failed. No automatic retry was made; inspect job/account status before resubmitting a generation.";

pub fn api_key() -> Result<String, String> {
    let key = std::env::var("PIXELLAB_API_KEY").unwrap_or_default();
    validate_key(&key)?;
    Ok(key)
}

fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || key.to_ascii_lowercase().starts_with("bearer ")
        || key.chars().any(|c| c.is_whitespace() || c.is_control())
        || reqwest::header::HeaderValue::from_str(key).is_err()
    {
        return Err(
            "Export PIXELLAB_API_KEY as the raw token, without 'Bearer ' or whitespace.".into(),
        );
    }
    Ok(())
}

pub struct Client {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl Client {
    pub fn new(key: Option<String>) -> Result<Self, String> {
        if let Some(key) = &key {
            validate_key(key)?;
        }
        Ok(Self {
            http: HttpClient::builder()
                .https_only(true)
                .timeout(Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .build()
                .map_err(|_| "Cannot initialize PixelLab HTTPS client.")?,
            key,
            base: BASE_URL.into(),
        })
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        payload: Option<&Value>,
    ) -> Result<Value, String> {
        let key = self
            .key
            .as_deref()
            .ok_or("PixelLab credentials are required.")?;
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(key);
        if let Some(payload) = payload {
            request = request.json(payload);
        }
        let mut response = request.send().await.map_err(|_| NETWORK_ERROR)?;
        check_status(response.status())?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| NETWORK_ERROR)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_JSON {
                return Err("PixelLab JSON response exceeds 8 MiB.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let mut value: Value =
            serde_json::from_slice(&bytes).map_err(|_| "PixelLab returned invalid JSON.")?;
        if !value.is_object() {
            return Err("PixelLab returned an unexpected JSON response.".into());
        }
        sanitize(&mut value, Some(key));
        Ok(value)
    }

    pub async fn balance(&self) -> Result<Value, String> {
        self.request(Method::GET, "/balance", None).await
    }
    pub async fn characters(&self, limit: u32, offset: u32) -> Result<Value, String> {
        if !(1..=100).contains(&limit) {
            return Err("Character limit must be 1-100.".into());
        }
        self.request(
            Method::GET,
            &format!("/characters?limit={limit}&offset={offset}"),
            None,
        )
        .await
    }
    pub async fn character(&self, id: &str) -> Result<Value, String> {
        let id = request::checked_id(id)?;
        self.request(Method::GET, &format!("/characters/{id}"), None)
            .await
    }
    pub async fn job(&self, id: &str) -> Result<Value, String> {
        let id = request::checked_id(id)?;
        self.request(Method::GET, &format!("/background-jobs/{id}"), None)
            .await
    }
    pub async fn submit(&self, value: &Value, create: bool) -> Result<Value, String> {
        request::validate(value, create)?;
        self.request(
            Method::POST,
            if create {
                request::CREATE_ROUTE
            } else {
                request::ANIMATE_ROUTE
            },
            Some(value),
        )
        .await
    }

    // The export endpoint is public. Even an authenticated Client never attaches its key.
    pub async fn download_to(&self, id: &str, path: &Path) -> Result<u64, String> {
        let id = request::checked_id(id)?;
        let mut response = self
            .http
            .get(format!("{}/characters/{id}/zip", self.base))
            .send()
            .await
            .map_err(|_| NETWORK_ERROR)?;
        check_status(response.status())?;
        let expected = response.content_length();
        if expected.is_some_and(|len| len > MAX_ZIP) {
            return Err("PixelLab ZIP exceeds 512 MiB.".into());
        }
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .await
            .map_err(|_| "Cannot open temporary ZIP output.")?;
        let mut total = 0;
        let mut head = Vec::new();
        let mut tail = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| NETWORK_ERROR)? {
            total += chunk.len() as u64;
            if total > MAX_ZIP {
                return Err("PixelLab ZIP exceeds 512 MiB.".into());
            }
            if head.len() < 4 {
                head.extend_from_slice(&chunk[..chunk.len().min(4 - head.len())]);
            }
            tail.extend_from_slice(&chunk);
            if tail.len() > 65_557 {
                tail.drain(..tail.len() - 65_557);
            }
            file.write_all(&chunk)
                .await
                .map_err(|_| "Cannot write temporary ZIP output.")?;
        }
        if expected.is_some_and(|len| len != total) {
            return Err("Incomplete PixelLab ZIP download.".into());
        }
        validate_zip(&head, &tail, total)?;
        file.sync_all()
            .await
            .map_err(|_| "Cannot flush ZIP output.")?;
        Ok(total)
    }
}

fn check_status(status: reqwest::StatusCode) -> Result<(), String> {
    if status.is_success() {
        return Ok(());
    }
    let hint = match status.as_u16() {
        401 => "Check PIXELLAB_API_KEY.",
        402 => "Insufficient allowance or credits.",
        403 => "Access denied.",
        404 => "Resource not found.",
        409 => "Animation direction already exists in this group.",
        422 => "Check the request against the REST OpenAPI schema.",
        423 => "Generation is still processing; check job status.",
        429 => "Rate/concurrency limit reached; check accepted jobs before resubmitting.",
        _ => "Request failed; no automatic retry was made.",
    };
    // Never echo error bodies: they may contain credentials or submitted images.
    Err(format!("PixelLab HTTP {}. {hint}", status.as_u16()))
}

fn validate_zip(head: &[u8], tail: &[u8], total: u64) -> Result<(), String> {
    let invalid = || "PixelLab download is not a complete conventional ZIP archive.".to_string();
    if head != b"PK\x03\x04" && head != b"PK\x05\x06" {
        return Err(invalid());
    }
    let end = tail
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .ok_or_else(invalid)?;
    let record = &tail[end..];
    if record.len() < 22 {
        return Err(invalid());
    }
    let u16_at = |n| u16::from_le_bytes([record[n], record[n + 1]]);
    let u32_at = |n| u32::from_le_bytes(record[n..n + 4].try_into().unwrap()) as u64;
    let offset = total - tail.len() as u64 + end as u64;
    if record.len() != 22 + u16_at(20) as usize
        || u16_at(4) != 0
        || u16_at(6) != 0
        || u16_at(8) != u16_at(10)
        || u32_at(12) + u32_at(16) != offset
    {
        return Err(invalid());
    }
    Ok(())
}

pub fn sanitize(value: &mut Value, key: Option<&str>) {
    match value {
        Value::Object(object) => {
            let old = std::mem::take(object);
            for (name, mut item) in old {
                if [
                    "api_key",
                    "apiKey",
                    "authorization",
                    "Authorization",
                    "token",
                    "secret",
                    "base64",
                    "prompt",
                    "description",
                    "action_description",
                    "enhanced_prompt",
                ]
                .contains(&name.as_str())
                {
                    continue;
                }
                sanitize(&mut item, key);
                let name = key
                    .filter(|k| !k.is_empty())
                    .map_or(name.clone(), |k| name.replace(k, "<redacted>"));
                object.insert(name, item);
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize(item, key);
            }
        }
        Value::String(text) => {
            if let Some(key) = key.filter(|k| !k.is_empty()) {
                *text = text.replace(key, "<redacted>");
            }
        }
        _ => {}
    }
}
