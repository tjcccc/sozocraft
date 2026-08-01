use crate::local_config;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::{Client, Proxy, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{net::IpAddr, time::Duration};

const ENDPOINT: &str = "https://open.volcengineapi.com/";
const HOST: &str = "open.volcengineapi.com";
const API_VERSION: &str = "2024-01-01";
const REGION: &str = "cn-beijing";
const SERVICE: &str = "ark";
const PROJECT_NAME: &str = "default";
const DEFAULT_GROUP_NAME: &str = "SozoCraft";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArkAsset {
    pub id: String,
    pub name: String,
    pub status: String,
    pub group_id: String,
    pub asset_type: String,
    pub create_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArkAssetRequest {
    pub name: String,
    pub source_url: String,
}

#[derive(Debug, Clone)]
pub struct ArkAssetClient {
    access_key: String,
    secret_key: String,
    client: Client,
}

pub fn client_from_local_config() -> Result<ArkAssetClient, String> {
    let (access_key, secret_key) =
        local_config::get_ark_asset_credentials().map_err(|error| error.to_string())?;
    let settings = local_config::load_settings(crate::models::AppSettings::default());
    let proxy_url = if settings.ark_proxy_enabled {
        settings.proxy_url
    } else {
        None
    };
    ArkAssetClient::new(
        access_key,
        secret_key,
        proxy_url,
        settings.ark_timeout_seconds,
    )
}

impl ArkAssetClient {
    fn new(
        access_key: String,
        secret_key: String,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, String> {
        validate_credential(&access_key, "access key")?;
        validate_credential(&secret_key, "secret key")?;
        let mut builder = Client::builder().timeout(Duration::from_secs(timeout_seconds.max(10)));
        if let Some(proxy_url) = proxy_url.filter(|value| !value.trim().is_empty()) {
            builder =
                builder.proxy(Proxy::all(proxy_url.trim()).map_err(|error| error.to_string())?);
        }
        Ok(Self {
            access_key: access_key.trim().to_string(),
            secret_key: secret_key.trim().to_string(),
            client: builder.build().map_err(|error| error.to_string())?,
        })
    }

    pub async fn list_assets(&self) -> Result<Vec<ArkAsset>, String> {
        let response = self
            .post(
                "ListAssets",
                json!({
                    "PageNumber": 1,
                    "PageSize": 100,
                    "Filter": {
                        "GroupType": "AIGC",
                        "ProjectName": PROJECT_NAME
                    }
                }),
            )
            .await?;
        parse_asset_items(&response)
    }

    pub async fn create_asset(&self, request: CreateArkAssetRequest) -> Result<ArkAsset, String> {
        let name = validate_asset_name(&request.name)?;
        let source_url = validate_source_url(&request.source_url)?;
        let group_id = self.find_or_create_default_group().await?;
        let response = self
            .post(
                "CreateAsset",
                json!({
                    "GroupId": group_id,
                    "URL": source_url.as_str(),
                    "AssetType": "Image",
                    "Name": name,
                    "ProjectName": PROJECT_NAME
                }),
            )
            .await?;
        let id = result_id(&response, "asset")?;
        match self.get_asset(&id).await {
            Ok(asset) => Ok(asset),
            Err(_) => Ok(ArkAsset {
                id,
                name: name.to_string(),
                status: "Processing".to_string(),
                group_id,
                asset_type: "Image".to_string(),
                create_time: None,
            }),
        }
    }

    async fn get_asset(&self, id: &str) -> Result<ArkAsset, String> {
        validate_asset_id(id)?;
        let response = self
            .post("GetAsset", json!({ "Id": id, "ProjectName": PROJECT_NAME }))
            .await?;
        parse_asset(result(&response)?)
    }

    async fn find_or_create_default_group(&self) -> Result<String, String> {
        let response = self
            .post(
                "ListAssetGroups",
                json!({
                    "PageNumber": 1,
                    "PageSize": 100,
                    "Filter": {
                        "GroupType": "AIGC",
                        "Name": DEFAULT_GROUP_NAME,
                        "ProjectName": PROJECT_NAME
                    }
                }),
            )
            .await?;
        if let Some(id) = result(&response)?
            .get("Items")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find_map(|item| {
                    let name = string_field(item, "Name")?;
                    let id = string_field(item, "Id")?;
                    (name == DEFAULT_GROUP_NAME && validate_group_id(id).is_ok())
                        .then(|| id.to_string())
                })
            })
        {
            return Ok(id);
        }

        let response = self
            .post(
                "CreateAssetGroup",
                json!({
                    "GroupType": "AIGC",
                    "Name": DEFAULT_GROUP_NAME,
                    "Description": "Seedance assets imported by SozoCraft",
                    "ProjectName": PROJECT_NAME
                }),
            )
            .await?;
        result_id(&response, "group")
    }

    async fn post(&self, action: &str, body: Value) -> Result<Value, String> {
        validate_action(action)?;
        let payload = serde_json::to_vec(&body)
            .map_err(|error| format!("Failed to encode Ark asset request: {error}"))?;
        let timestamp = Utc::now();
        let signed = sign_request(
            &self.access_key,
            &self.secret_key,
            action,
            &payload,
            timestamp,
        )?;
        let url = format!("{ENDPOINT}?Action={action}&Version={API_VERSION}");
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("X-Date", signed.timestamp)
            .header("X-Content-Sha256", signed.payload_hash)
            .header("Authorization", signed.authorization)
            .body(payload)
            .send()
            .await
            .map_err(|error| format!("Volcengine Ark asset request failed: {error}"))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        response_json(status, &bytes)
    }
}

struct SignedRequest {
    timestamp: String,
    payload_hash: String,
    authorization: String,
}

fn sign_request(
    access_key: &str,
    secret_key: &str,
    action: &str,
    payload: &[u8],
    timestamp: DateTime<Utc>,
) -> Result<SignedRequest, String> {
    validate_action(action)?;
    let timestamp = timestamp.format("%Y%m%dT%H%M%SZ").to_string();
    let date = &timestamp[..8];
    let payload_hash = sha256_hex(payload);
    let canonical_query = format!("Action={action}&Version={API_VERSION}");
    let signed_headers = "host;x-content-sha256;x-date";
    let canonical_headers =
        format!("host:{HOST}\nx-content-sha256:{payload_hash}\nx-date:{timestamp}\n");
    let canonical_request = format!(
        "POST\n/\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let scope = format!("{date}/{REGION}/{SERVICE}/request");
    let string_to_sign = format!(
        "HMAC-SHA256\n{timestamp}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let date_key = hmac_sha256(secret_key.as_bytes(), date)?;
    let region_key = hmac_sha256(&date_key, REGION)?;
    let service_key = hmac_sha256(&region_key, SERVICE)?;
    let signing_key = hmac_sha256(&service_key, "request")?;
    let signature = hex_bytes(&hmac_sha256(&signing_key, &string_to_sign)?);
    let authorization = format!(
        "HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );
    Ok(SignedRequest {
        timestamp,
        payload_hash,
        authorization,
    })
}

fn hmac_sha256(key: &[u8], value: &str) -> Result<Vec<u8>, String> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| "Failed to initialize Ark request signing.".to_string())?;
    mac.update(value.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(value: &[u8]) -> String {
    hex_bytes(&Sha256::digest(value))
}

fn hex_bytes(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn response_json(status: StatusCode, bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err("Volcengine Ark asset response was too large.".to_string());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| {
        format!("Volcengine Ark asset API returned an unreadable response ({status}).")
    })?;
    if !status.is_success() || value.get("Result").is_none() {
        return Err(format!(
            "Volcengine Ark asset API returned an error ({status}): {}",
            sanitized_error(&value)
        ));
    }
    Ok(value)
}

fn sanitized_error(value: &Value) -> String {
    value
        .pointer("/ResponseMetadata/Error/Message")
        .or_else(|| value.pointer("/error/message"))
        .and_then(Value::as_str)
        .map(|message| message.chars().take(500).collect())
        .unwrap_or_else(|| "Request failed without an error message.".to_string())
}

fn result(value: &Value) -> Result<&Value, String> {
    value
        .get("Result")
        .ok_or_else(|| "Volcengine Ark asset response did not include Result.".to_string())
}

fn result_id(value: &Value, prefix: &str) -> Result<String, String> {
    let id = string_field(result(value)?, "Id")
        .ok_or_else(|| format!("Volcengine Ark did not return a {prefix} id."))?;
    if prefix == "asset" {
        validate_asset_id(id)?;
    } else {
        validate_group_id(id)?;
    }
    Ok(id.to_string())
}

fn parse_asset_items(value: &Value) -> Result<Vec<ArkAsset>, String> {
    result(value)?
        .get("Items")
        .and_then(Value::as_array)
        .ok_or_else(|| "Volcengine Ark asset list did not include Items.".to_string())?
        .iter()
        .map(parse_asset)
        .collect()
}

fn parse_asset(value: &Value) -> Result<ArkAsset, String> {
    let id = string_field(value, "Id").ok_or_else(|| "Ark asset is missing an id.".to_string())?;
    validate_asset_id(id)?;
    Ok(ArkAsset {
        id: id.to_string(),
        name: string_field(value, "Name").unwrap_or(id).to_string(),
        status: string_field(value, "Status")
            .unwrap_or("Unknown")
            .to_string(),
        group_id: string_field(value, "GroupId")
            .unwrap_or_default()
            .to_string(),
        asset_type: string_field(value, "AssetType")
            .unwrap_or("Image")
            .to_string(),
        create_time: string_field(value, "CreateTime").map(str::to_string),
    })
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn validate_credential(value: &str, label: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 512 || value.chars().any(char::is_whitespace) {
        return Err(format!("Volcengine Ark asset {label} is invalid."));
    }
    Ok(())
}

fn validate_action(action: &str) -> Result<(), String> {
    if !matches!(
        action,
        "ListAssets" | "GetAsset" | "ListAssetGroups" | "CreateAssetGroup" | "CreateAsset"
    ) {
        return Err("Unsupported Volcengine Ark asset action.".to_string());
    }
    Ok(())
}

fn validate_asset_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err("Ark asset name must contain 1 to 64 printable characters.".to_string());
    }
    Ok(name)
}

fn validate_source_url(value: &str) -> Result<Url, String> {
    if value.len() > 2048 {
        return Err("Ark asset source URL is too long.".to_string());
    }
    let url = Url::parse(value.trim())
        .map_err(|_| "Ark asset source must be a valid public HTTPS URL.".to_string())?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err("Ark asset source must be a public HTTPS URL without credentials.".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Ark asset source URL is missing a host.".to_string())?;
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || !host.contains('.')
        || host.parse::<IpAddr>().is_ok_and(is_non_public_ip)
    {
        return Err("Ark asset source must use a public host.".to_string());
    }
    Ok(url)
}

fn is_non_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

pub fn validate_asset_id(id: &str) -> Result<(), String> {
    validate_resource_id(id, "asset")
}

fn validate_group_id(id: &str) -> Result<(), String> {
    validate_resource_id(id, "group")
}

fn validate_resource_id(id: &str, prefix: &str) -> Result<(), String> {
    let expected = format!("{prefix}-");
    if !id.starts_with(&expected)
        || id.len() > 128
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(format!("Volcengine Ark returned an invalid {prefix} id."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn signs_with_volcengine_v4_scope_and_headers() {
        let signed = sign_request(
            "AKLTexample",
            "secret",
            "ListAssets",
            br#"{"PageNumber":1}"#,
            Utc.with_ymd_and_hms(2026, 8, 1, 4, 5, 6).unwrap(),
        )
        .unwrap();

        assert_eq!(signed.timestamp, "20260801T040506Z");
        assert!(signed
            .authorization
            .contains("Credential=AKLTexample/20260801/cn-beijing/ark/request"));
        assert!(signed
            .authorization
            .contains("SignedHeaders=host;x-content-sha256;x-date"));
        assert_eq!(signed.payload_hash.len(), 64);
        assert_eq!(signed.authorization.rsplit('=').next().unwrap().len(), 64);
    }

    #[test]
    fn rejects_non_public_source_urls() {
        for value in [
            "http://example.com/image.png",
            "https://localhost/image.png",
            "https://127.0.0.1/image.png",
            "https://user:pass@example.com/image.png",
        ] {
            assert!(validate_source_url(value).is_err(), "accepted {value}");
        }
        assert!(validate_source_url("https://images.example.com/image.png").is_ok());
    }

    #[test]
    fn parses_asset_list_result() {
        let assets = parse_asset_items(&json!({
            "Result": {
                "Items": [{
                    "Id": "asset-20260801-example",
                    "Name": "portrait",
                    "Status": "Active",
                    "GroupId": "group-20260801-example",
                    "AssetType": "Image",
                    "URL": "https://example.com/portrait.png"
                }]
            }
        }))
        .unwrap();

        assert_eq!(assets[0].id, "asset-20260801-example");
        assert_eq!(assets[0].status, "Active");
    }
}
