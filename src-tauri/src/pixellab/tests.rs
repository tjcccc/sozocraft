use super::*;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Instant,
};

const ID: &str = "123e4567-e89b-12d3-a456-426614174000";

fn mock(status: u16, body: Vec<u8>) -> (Client, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}/v2", listener.local_addr().unwrap());
    let redirect_base = base.clone();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    loop {
                        let mut buffer = [0; 4096];
                        let n = stream.read(&mut buffer).unwrap();
                        if n == 0 {
                            break;
                        }
                        bytes.extend_from_slice(&buffer[..n]);
                        if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                            let headers =
                                String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                            let length = headers
                                .lines()
                                .find_map(|l| {
                                    l.strip_prefix("content-length: ")
                                        .and_then(|s| s.parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if bytes.len() >= end + 4 + length {
                                break;
                            }
                        }
                    }
                    requests.push(String::from_utf8(bytes).unwrap());
                    if status == 0 {
                        thread::sleep(Duration::from_millis(250));
                        deadline = Instant::now() + Duration::from_millis(150);
                        continue;
                    }
                    let headers = format!("HTTP/1.1 {status} Mock\r\nContent-Length: {}\r\nLocation: {redirect_base}/balance\r\nConnection: close\r\n\r\n", body.len());
                    stream.write_all(headers.as_bytes()).unwrap();
                    stream.write_all(&body).unwrap();
                    deadline = Instant::now() + Duration::from_millis(150);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        }
        requests
    });
    // Only unit tests can use HTTP, a custom origin or a shorter timeout.
    (
        Client {
            http: HttpClient::builder()
                .no_proxy()
                .timeout(Duration::from_secs(2))
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .build()
                .unwrap(),
            key: Some("test-secret".into()),
            base,
        },
        handle,
    )
}

#[test]
fn raw_key_validation_and_output_redaction() {
    for key in [
        "",
        "Bearer secret",
        "bearer secret",
        " secret",
        "secret\n",
        "secret key",
    ] {
        assert!(validate_key(key).is_err());
    }
    assert!(validate_key("test-secret").is_ok());
    let mut v = json!({"token":"test-secret", "nested":{"url":"https://example/test-secret", "base64":"bytes", "test-secret":"echo"}, "description":"private prompt"});
    sanitize(&mut v, Some("test-secret"));
    let s = v.to_string();
    assert!(!s.contains("test-secret") && !s.contains("bytes") && !s.contains("private prompt"));
    assert!(s.contains("<redacted>"));
}

#[tokio::test]
async fn read_routes_authentication_and_redaction() {
    for (route, action) in [
        ("/balance", 0),
        ("/characters?limit=50&offset=2", 1),
        ("/characters/123e4567-e89b-12d3-a456-426614174000", 2),
        ("/background-jobs/123e4567-e89b-12d3-a456-426614174000", 3),
    ] {
        let (client, handle) = mock(
            200,
            br#"{"status":"completed","echo":"test-secret"}"#.to_vec(),
        );
        let result = match action {
            0 => client.balance().await,
            1 => client.characters(50, 2).await,
            2 => client.character(ID).await,
            _ => client.job(ID).await,
        }
        .unwrap();
        assert_eq!(result["echo"], "<redacted>");
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with(&format!("GET /v2{route} HTTP/1.1")));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-secret"));
    }
}

#[tokio::test]
async fn submission_preserves_all_ids_and_schema_fields() {
    for (create, payload) in [
        (
            true,
            json!({"description":"knight", "image_size":{"width":128,"height":128}}),
        ),
        (
            false,
            json!({"character_id":ID,"mode":"v3","action_description":"walk","directions":["east"],"frame_count":8,"keep_first_frame":false}),
        ),
        (
            false,
            json!({"character_id":ID,"mode":"skeleton-v3","template_animation_id":"walking-8-frames","directions":["east"]}),
        ),
    ] {
        let (client, handle) = mock(200,json!({"background_job_id":ID,"background_job_ids":[ID,ID],"character_id":ID,"animation_group_id":ID,"directions":["east","west"]}).to_string().into_bytes());
        let result = client.submit(&payload, create).await.unwrap();
        assert_eq!(result["background_job_ids"].as_array().unwrap().len(), 2);
        assert_eq!(result["animation_group_id"], ID);
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 1);
        let (headers, body) = requests[0].split_once("\r\n\r\n").unwrap();
        assert!(headers.starts_with(&format!(
            "POST /v2{}",
            if create {
                request::CREATE_ROUTE
            } else {
                request::ANIMATE_ROUTE
            }
        )));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), payload);
    }
}

#[tokio::test]
async fn failures_and_redirects_never_repeat_paid_submissions_or_echo_bodies() {
    for status in [307, 401, 402, 422, 429, 500] {
        let (client, handle) = mock(status, b"test-secret private prompt".to_vec());
        let error = client
            .submit(&json!({"description":"private prompt"}), true)
            .await
            .unwrap_err();
        assert!(!error.contains("test-secret") && !error.contains("private prompt"));
        assert_eq!(handle.join().unwrap().len(), 1);
    }
    let (client, handle) = mock(200, b"not json test-secret".to_vec());
    assert_eq!(
        client.balance().await.unwrap_err(),
        "PixelLab returned invalid JSON."
    );
    handle.join().unwrap();
}

#[tokio::test]
async fn public_zip_has_no_auth_and_rejects_invalid_or_incomplete_archives() {
    let path = std::env::temp_dir().join(format!("pixellab-{}.part", uuid::Uuid::new_v4()));
    // Empty conventional ZIP; validation does not decompress or extract assets.
    let zip = [b"PK\x05\x06".as_slice(), &[0; 18]].concat();
    for (body, success) in [
        (zip, true),
        (b"<html>test-secret</html>".to_vec(), false),
        (b"PK\x03\x04truncated".to_vec(), false),
    ] {
        std::fs::write(&path, []).unwrap();
        let (client, handle) = mock(200, body);
        assert_eq!(client.download_to(ID, &path).await.is_ok(), success);
        let requests = handle.join().unwrap();
        assert!(requests[0].starts_with(&format!("GET /v2/characters/{ID}/zip")));
        assert!(!requests[0].to_ascii_lowercase().contains("authorization"));
        assert!(!requests[0].contains("test-secret"));
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn requests_reject_invalid_types_ranges_modes_and_payloads() {
    for value in [
        json!([]),
        json!({}),
        json!({"description":""}),
        json!({"description":"x","unknown":1}),
        json!({"description":"x","image_size":{"width":257,"height":32}}),
        json!({"description":"x","seed":-1}),
        json!({"description":"x","view":"oblique"}),
        json!({"description":"x","reference_image":{"base64":"bad"}}),
    ] {
        assert!(request::validate(&value, true).is_err(), "{value}");
    }
    for patch in [
        json!({"character_id":"../secret"}),
        json!({"mode":"skeleton-v3"}),
        json!({"frame_count":7}),
        json!({"directions":["east","east"]}),
        json!({"directions":["down"]}),
        json!({"async_mode":false}),
        json!({"mode":"pro","keep_first_frame":false}),
        json!({"mode":"v3","template_animation_id":"walking"}),
    ] {
        let mut value = json!({"character_id":ID,"mode":"v3","action_description":"walking"});
        value
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(request::validate(&value, false).is_err(), "{value}");
    }
}

#[tokio::test]
async fn submission_timeout_does_not_repeat_or_expose_the_key() {
    let (mut client, handle) = mock(0, Vec::new());
    client.http = HttpClient::builder()
        .no_proxy()
        .timeout(Duration::from_millis(100))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()
        .unwrap();
    let error = client
        .submit(&json!({"description":"private prompt"}), true)
        .await
        .unwrap_err();
    assert_eq!(error, NETWORK_ERROR);
    assert_eq!(handle.join().unwrap().len(), 1);
}
