use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

struct Workspace(PathBuf);
impl Workspace {
    fn new(config: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("sozocraft-cli-images-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(path.join(".sozocraft")).unwrap();
        fs::write(path.join(".sozocraft/config.toml"), config).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sozocraft-cli"))
            .env("HOME", &self.0)
            .env("NO_PROXY", "*")
            .env("no_proxy", "*")
            .current_dir(&self.0)
            .args(["image", "generate"])
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn events(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn image_dry_run_uses_config_without_keys_or_output_writes() {
    let workspace = Workspace::new("[app]\ndefault_provider='gpt-image'\n[openai]\napi_platform='openrouter'\ndefault_model='openai/gpt-image-2.5-flare'\n");
    let output = workspace.run(&[
        "--prompt",
        "sprite",
        "--quality",
        "max",
        "--output",
        "new/sprite.png",
        "--dry-run",
    ]);
    assert!(output.status.success(), "{:?}", events(&output));
    let result = &events(&output)[0];
    assert_eq!(result["platform"], "openrouter");
    assert_eq!(result["model"], "openai/gpt-image-2.5-flare");
    assert!(!workspace.0.join("new").exists());
    let output = workspace.run(&["--prompt", "sprite", "--size", "not-a-size", "--dry-run"]);
    assert_eq!(output.status.code(), Some(1));
}

fn mock_provider(response: String) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("mock did not receive a request: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 4096];
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(offset) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..offset]);
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_string)
                    })
                    .unwrap()
                    .parse()
                    .unwrap();
                if bytes.len() >= offset + 4 + length {
                    break;
                }
            }
        }
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        String::from_utf8(bytes).unwrap()
    });
    (address, handle)
}

#[test]
fn native_image_routes_generate_and_publish_png_using_shared_config() {
    let mut png = Vec::new();
    image::DynamicImage::new_rgba8(2, 3)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let encoded = general_purpose::STANDARD.encode(png);
    for (section, platform, model, suffix) in [
        (
            "gemini",
            "gemini",
            "gemini-3.1-flash-image-preview",
            "/models/gemini-3.1-flash-image-preview:generateContent",
        ),
        ("openai", "openai", "gpt-image-2", "/images/generations"),
        (
            "openrouter",
            "openrouter",
            "openai/gpt-image-2",
            "/api/v1/images",
        ),
        (
            "xai",
            "xai",
            "grok-imagine-image-2.0",
            "/images/generations",
        ),
    ] {
        let body = if section == "gemini" {
            json!({"candidates":[{"content":{"parts":[{"inlineData":{"mimeType":"image/png","data":encoded}}]}}]})
        } else {
            json!({"data":[{"b64_json":encoded}]})
        };
        let (endpoint, server) = mock_provider(body.to_string());
        let base = if section == "gemini" {
            format!("{endpoint}/models")
        } else {
            endpoint
        };
        let mut config = format!(
            "[{section}]\napi_key='test-only-key'\nbase_url='{base}'\nproxy_enabled=false\n"
        );
        if platform == "openrouter" {
            config.push_str("[openai]\napi_platform='openrouter'\nproxy_enabled=false\n");
        }
        let workspace = Workspace::new(&config);
        let mut args = vec![
            "--model",
            model,
            "--prompt",
            "pixel sprite",
            "--output",
            "images/sprite.png",
        ];
        let reference = workspace.0.join("reference.png");
        let reference_name = reference.to_str().unwrap();
        if section == "gemini" {
            fs::write(
                &reference,
                general_purpose::STANDARD.decode(&encoded).unwrap(),
            )
            .unwrap();
            args.extend(["--reference", reference_name]);
        }
        let output = workspace.run(&args);
        assert!(
            output.status.success(),
            "{} {:?}",
            String::from_utf8_lossy(&output.stderr),
            events(&output)
        );
        let request = server.join().unwrap();
        assert!(request.starts_with(&format!("POST {suffix} ")), "{request}");
        assert!(request.contains("test-only-key"));
        if section == "gemini" {
            assert!(request.contains("inline_data"));
            assert!(request.contains(&encoded));
        }
        let result = events(&output);
        assert_eq!(result[0]["status"], "started");
        assert_eq!(result[1]["width"], 2);
        assert_eq!(result[1]["height"], 3);
        assert_eq!(result[2]["status"], "completed");
        assert_eq!(result[0]["requestId"], result[1]["requestId"]);
        assert_eq!(result[0]["requestId"], result[2]["requestId"]);
        assert!(workspace.0.join("images/sprite.png").is_file());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("test-only-key"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("pixel sprite"));
        let original = fs::read(workspace.0.join("images/sprite.png")).unwrap();
        let output = workspace.run(&[
            "--model",
            model,
            "--prompt",
            "pixel sprite",
            "--output",
            "images/sprite.png",
        ]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            fs::read(workspace.0.join("images/sprite.png")).unwrap(),
            original
        );
        assert_eq!(fs::read_dir(workspace.0.join("images")).unwrap().count(), 1);
    }
}

#[test]
fn corrupt_provider_image_leaves_no_output_or_partial_file() {
    let (endpoint, server) = mock_provider(
        json!({"data":[{"b64_json":general_purpose::STANDARD.encode(b"not an image") }]})
            .to_string(),
    );
    let workspace = Workspace::new(&format!(
        "[openai]\napi_key='test-only-key'\nbase_url='{endpoint}'\nproxy_enabled=false\n"
    ));
    let output = workspace.run(&[
        "--model",
        "gpt-image-2",
        "--prompt",
        "sprite",
        "--output",
        "images/sprite.png",
    ]);
    assert_eq!(output.status.code(), Some(1));
    server.join().unwrap();
    let result = events(&output);
    assert_eq!(result[0]["status"], "started");
    assert_eq!(result[1]["status"], "error");
    assert_eq!(fs::read_dir(workspace.0.join("images")).unwrap().count(), 0);
}

#[test]
fn experimental_muse_image_uses_openrouter_images_api_with_shared_platform_settings() {
    let mut png = Vec::new();
    image::DynamicImage::new_rgba8(4, 2)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let encoded = general_purpose::STANDARD.encode(&png);
    let body = json!({"data":[{"b64_json":encoded,"media_type":"image/png"}]});
    let (endpoint, server) = mock_provider(body.to_string());
    let workspace = Workspace::new(&format!(
        "[openrouter]\napi_key='test-only-key'\nbase_url='{endpoint}/api/v1'\nproxy_enabled=false\n"
    ));
    let reference = workspace.0.join("reference.png");
    fs::write(&reference, &png).unwrap();
    let output = workspace.run(&[
        "--model",
        "meta/muse-image",
        "--prompt",
        "pixel sprite",
        "--aspect-ratio",
        "21:9",
        "--reference",
        reference.to_str().unwrap(),
        "--output",
        "images/muse.png",
    ]);
    assert!(
        output.status.success(),
        "{} {:?}",
        String::from_utf8_lossy(&output.stderr),
        events(&output)
    );
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /api/v1/images "), "{request}");
    assert!(request.contains("test-only-key"));
    assert!(request.contains(r#""size":"2016x864""#), "{request}");
    assert!(request.contains(r#""moderation":"low""#), "{request}");
    assert!(request.contains(r#""input_references""#), "{request}");
    assert!(request.contains(&format!("data:image/png;base64,{encoded}")));
    let result = events(&output);
    assert_eq!(result[0]["provider"], "experimental");
    assert_eq!(result[0]["platform"], "openrouter");
    assert_eq!(result[1]["width"], 4);
    assert_eq!(result[2]["status"], "completed");
    assert!(workspace.0.join("images/muse.png").exists());
}
