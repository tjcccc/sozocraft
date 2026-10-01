use serde_json::Value;
use std::process::Command;

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
