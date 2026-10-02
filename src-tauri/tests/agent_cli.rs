use serde_json::Value;
use std::process::Command;

#[test]
fn version_flags_report_package_version_without_loading_config() {
    let dir = std::env::temp_dir().join(format!("sozocraft-cli-version-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join(".sozocraft")).unwrap();
    std::fs::write(dir.join(".sozocraft/config.toml"), "invalid [toml").unwrap();
    for flag in ["--version", "-V"] {
        let result = Command::new(env!("CARGO_BIN_EXE_sozocraft-cli"))
            .arg(flag)
            .env("HOME", &dir)
            .env("USERPROFILE", &dir)
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            format!("sozocraft-cli {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mixed_frame_references_are_only_accepted_for_omni() {
    let dir = std::env::temp_dir().join(format!("sozocraft-cli-mixed-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("sprite.png");
    std::fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
    for model in [
        "gemini-omni-1.1-flash",
        "veo-3.1-generate-preview",
        "veo-3.1-lite-generate-preview",
    ] {
        for pair in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_sozocraft-cli"));
            command
                .args([
                    "video",
                    "generate",
                    "--model",
                    model,
                    "--prompt",
                    "walk",
                    "--duration",
                    "8",
                    "--dry-run",
                    "--start-image",
                ])
                .arg(&path)
                .arg("--reference")
                .arg(&path);
            if pair {
                command.arg("--end-image").arg(&path);
            }
            let result = command.output().unwrap();
            let value: Value = serde_json::from_slice(&result.stdout).unwrap();
            if model == "gemini-omni-1.1-flash" {
                assert!(result.status.success(), "{value}");
                assert_eq!(value["status"], "validated");
                assert_eq!(value["inputMode"], if pair { "frames" } else { "image" });
                assert_eq!(value["inputImageCount"], if pair { 3 } else { 2 });
            } else {
                assert_eq!(result.status.code(), Some(1));
                assert_eq!(value["status"], "error");
            }
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn headless_dry_run_returns_json_for_both_models() {
    for (model, duration) in [
        ("veo-3.1-generate-preview", 8),
        ("veo-3.1-lite-generate-preview", 8),
        ("gemini-omni-1.1-flash", 5),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_sozocraft-cli"))
            .args([
                "video",
                "generate",
                "--model",
                model,
                "--prompt",
                "Pixel character walks in place",
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(value["status"], "validated");
        assert_eq!(value["model"], model);
        assert_eq!(value["options"]["duration"], duration);
        assert_eq!(value["inputMode"], "text");
        assert!(!String::from_utf8_lossy(&result.stdout).contains("Pixel character"));
    }
}

#[test]
fn invalid_video_options_exit_with_json_error() {
    let result = Command::new(env!("CARGO_BIN_EXE_sozocraft-cli"))
        .args([
            "video",
            "generate",
            "--prompt",
            "walk",
            "--duration",
            "5",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["status"], "error");
    assert!(value["error"].as_str().unwrap().contains("duration"));
}
