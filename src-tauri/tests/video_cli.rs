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
            std::env::temp_dir().join(format!("sozocraft-cli-video-{}", uuid::Uuid::new_v4()));
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
            .args(["video"])
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
fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{} {:?}",
        String::from_utf8_lossy(&output.stderr),
        events(output)
    );
    events(output)[0].clone()
}

#[test]
fn new_models_dry_run_and_enforce_provider_limits_without_keys() {
    let workspace = Workspace::new("[ark]\napi_platform='higgsfield'\n");
    let path = workspace.0.join("sprite.png");
    fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
    for model in [
        "doubao-seedance-2-0-260128",
        "doubao-seedance-2-0-fast-260128",
        "doubao-seedance-2-0-mini-260615",
        "doubao-seedance-2-5-260628",
        "grok-imagine-video-1.5",
    ] {
        let output = workspace.run(&[
            "generate",
            "--model",
            model,
            "--prompt",
            "walk",
            "--start-image",
            "sprite.png",
            "--end-image",
            "sprite.png",
            "--generate-audio",
            "false",
            "--output",
            "new/sprite.mp4",
            "--dry-run",
        ]);
        let value = success(&output);
        assert_eq!(value["options"]["duration"], 5);
        assert_eq!(value["options"]["generateAudio"], false);
        assert_eq!(
            value["platform"],
            if model.starts_with("doubao") {
                "higgsfield"
            } else {
                "xai"
            }
        );
        assert!(!workspace.0.join("new").exists());
    }
    for args in [
        vec![
            "generate",
            "--model",
            "grok-imagine-video-1.5",
            "--prompt",
            "walk",
            "--reference",
            "sprite.png",
            "--resolution",
            "1080p",
            "--dry-run",
        ],
        vec![
            "generate",
            "--model",
            "doubao-seedance-2-0-fast-260128",
            "--prompt",
            "walk",
            "--resolution",
            "1080p",
            "--dry-run",
        ],
        vec![
            "generate",
            "--model",
            "doubao-seedance-2-0-260128",
            "--prompt",
            "walk",
            "--duration",
            "30",
            "--dry-run",
        ],
    ] {
        assert_eq!(workspace.run(&args).status.code(), Some(1));
    }
    let mut args = vec![
        "generate",
        "--model",
        "doubao-seedance-2-5-260628",
        "--prompt",
        "walk",
        "--duration",
        "30",
        "--dry-run",
    ];
    for _ in 0..30 {
        args.extend(["--reference", "sprite.png"]);
    }
    let value = success(&workspace.run(&args));
    assert_eq!(value["inputImageCount"], 30);
    args.extend(["--reference", "sprite.png"]);
    assert_eq!(workspace.run(&args).status.code(), Some(1));
}

fn mock_provider(responses: Vec<Value>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
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
                        .unwrap_or_else(|| "0".to_string())
                        .parse()
                        .unwrap();
                    if bytes.len() >= offset + 4 + length {
                        break;
                    }
                }
            }
            let body = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            requests.push(String::from_utf8(bytes).unwrap());
        }
        requests
    });
    (address, handle)
}

#[test]
fn ark_and_xai_submit_once_and_resume_existing_jobs() {
    for (section, model, platform, prefix) in [
        (
            "ark",
            "doubao-seedance-2-0-260128",
            "ark",
            "/contents/generations/tasks",
        ),
        (
            "xai",
            "grok-imagine-video-1.5",
            "xai",
            "/videos/generations",
        ),
    ] {
        let id = "task-test_123";
        let start = if section == "ark" {
            json!({"id":id})
        } else {
            json!({"request_id":id})
        };
        let pending = json!({"status":"running"});
        let completed = if section == "ark" {
            json!({"status":"succeeded", "content":{"video_url":"https://tos.volces.com/test.mp4"}})
        } else {
            json!({"status":"done", "video":{"url":"https://vidgen.x.ai/test.mp4"}})
        };
        let failed = json!({"status":"failed", "error":{"message":"mock failure"}});
        let (endpoint, server) = mock_provider(vec![start, pending, completed, failed]);
        let image_platform = if section == "xai" {
            "higgsfield"
        } else {
            platform
        };
        let workspace = Workspace::new(&format!("[{section}]\napi_key='test-only-key'\nbase_url='{endpoint}'\nproxy_enabled=false\napi_platform='{image_platform}'\n"));
        let submitted = success(&workspace.run(&[
            "generate",
            "--model",
            model,
            "--prompt",
            "walk",
            "--generate-audio",
            "false",
            "--no-wait",
        ]));
        assert_eq!(submitted["operation"], id);
        assert_eq!(submitted["platform"], platform);
        for expected in ["pending", "completed"] {
            let status = success(&workspace.run(&[
                "status",
                "--model",
                model,
                "--platform",
                platform,
                "--operation",
                id,
            ]));
            assert_eq!(status["status"], expected);
            assert!(!status.to_string().contains("https://"));
        }
        let output = workspace.run(&[
            "wait",
            "--model",
            model,
            "--platform",
            platform,
            "--operation",
            id,
            "--output",
            "outputs/walk.mp4",
            "--max-wait",
            "1",
        ]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            fs::read_dir(workspace.0.join("outputs")).unwrap().count(),
            0
        );
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with(&format!("POST {prefix} ")));
        for request in &requests[1..] {
            assert!(request.starts_with("GET "));
            assert!(request.contains(id));
        }
        assert!(requests
            .iter()
            .all(|request| request.contains("test-only-key")));
        assert!(requests[0].contains("generate_audio") || requests[0].contains("audio"));
    }
}

#[cfg(unix)]
#[test]
fn higgsfield_route_uses_configured_executable_and_preserves_job_recovery() {
    use std::os::unix::fs::PermissionsExt;
    let workspace = Workspace::new("");
    let executable = workspace.0.join("higgsfield");
    fs::write(&executable, "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$HOME/calls.txt\"\ncase \"$2\" in\ncreate) printf '{\"id\":\"job-test-123\"}\\n';;\nget) cat \"$HOME/poll.json\";;\n*) exit 1;;\nesac\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let config = format!(
        "[ark]\napi_platform='ark'\n[higgsfield]\ncli_path='{}'\nproxy_enabled=false\n",
        executable.display()
    );
    fs::write(workspace.0.join(".sozocraft/config.toml"), config).unwrap();
    fs::write(workspace.0.join("poll.json"), "{\"status\":\"running\"}").unwrap();
    let submitted = success(&workspace.run(&[
        "generate",
        "--model",
        "doubao-seedance-2-5-260628",
        "--platform",
        "higgsfield",
        "--prompt",
        "walk",
        "--no-wait",
    ]));
    assert_eq!(submitted["operation"], "job-test-123");
    assert_eq!(submitted["platform"], "higgsfield");
    let result = success(&workspace.run(&[
        "status",
        "--model",
        "doubao-seedance-2-5-260628",
        "--platform",
        "higgsfield",
        "--operation",
        "job-test-123",
    ]));
    assert_eq!(result["status"], "pending");
    fs::write(
        workspace.0.join("poll.json"),
        "{\"status\":\"failed\",\"message\":\"mock failure\"}",
    )
    .unwrap();
    let output = workspace.run(&[
        "wait",
        "--model",
        "doubao-seedance-2-5-260628",
        "--platform",
        "higgsfield",
        "--operation",
        "job-test-123",
        "--output",
        "outputs/walk.mp4",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(events(&output)[0]["error"]
        .as_str()
        .unwrap()
        .contains("mock failure"));
    assert_eq!(
        fs::read_dir(workspace.0.join("outputs")).unwrap().count(),
        0
    );
    let calls = fs::read_to_string(workspace.0.join("calls.txt")).unwrap();
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("generate create "))
            .count(),
        1
    );
    assert!(calls.contains("seedance_2_5"));
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("generate get job-test-123 "))
            .count(),
        2
    );
}
