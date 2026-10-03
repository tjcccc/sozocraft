use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("sozocraft-pixellab-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(path.join(".sozocraft")).unwrap();
        fs::write(path.join(".sozocraft/config.toml"), "invalid [toml").unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str], success: bool) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_sozocraft-cli"))
            .args(["pixellab"])
            .args(args)
            .current_dir(&self.0)
            .env("HOME", &self.0)
            .env_remove("PIXELLAB_API_KEY")
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(output.stderr.is_empty());
        serde_json::from_slice(&output.stdout).unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const ID: &str = "123e4567-e89b-12d3-a456-426614174000";

#[test]
fn dry_runs_cover_all_commands_without_config_credentials_or_output_writes() {
    let ws = Workspace::new();
    fs::write(
        ws.0.join("create.json"),
        json!({"description":"private knight prompt","image_size":{"width":128,"height":128}})
            .to_string(),
    )
    .unwrap();
    fs::write(ws.0.join("walk.json"),json!({"character_id":ID,"mode":"v3","action_description":"private walk prompt","directions":["east"],"frame_count":8,"keep_first_frame":false}).to_string()).unwrap();
    fs::write(ws.0.join("skeleton.json"),json!({"character_id":ID,"mode":"skeleton-v3","template_animation_id":"walking-8-frames","directions":["east"]}).to_string()).unwrap();
    for args in [
        vec!["balance", "--dry-run"],
        vec![
            "characters",
            "--limit",
            "100",
            "--offset",
            "50",
            "--dry-run",
        ],
        vec!["character", ID, "--dry-run"],
        vec!["job", ID, "--dry-run"],
        vec!["create-character", "--request", "create.json", "--dry-run"],
        vec!["animate", "--request", "walk.json", "--dry-run"],
        vec!["animate", "--request", "skeleton.json", "--dry-run"],
        vec!["download", ID, "--output", "new/character.zip", "--dry-run"],
    ] {
        let value = ws.run(&args, true);
        assert_eq!(value["status"], "validated");
        assert_eq!(value["provider"], "pixellab");
        assert!(!value.to_string().contains("private"));
    }
    assert!(!ws.0.join("new").exists());
}

#[test]
fn reference_png_is_decoded_and_never_printed() {
    let ws = Workspace::new();
    image::RgbaImage::new(128, 128)
        .save(ws.0.join("south.png"))
        .unwrap();
    fs::write(ws.0.join("create.json"), r#"{"description":"knight"}"#).unwrap();
    let value = ws.run(
        &[
            "create-character",
            "--request",
            "create.json",
            "--reference",
            "south.png",
            "--dry-run",
        ],
        true,
    );
    assert_eq!(value["referenceCount"], 1);
    assert!(!value.to_string().contains("base64"));
    image::RgbaImage::new(257, 128)
        .save(ws.0.join("large.png"))
        .unwrap();
    ws.run(
        &[
            "create-character",
            "--request",
            "create.json",
            "--reference",
            "large.png",
            "--dry-run",
        ],
        false,
    );
    fs::write(ws.0.join("broken.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    ws.run(
        &[
            "create-character",
            "--request",
            "create.json",
            "--reference",
            "broken.png",
            "--dry-run",
        ],
        false,
    );
}

#[test]
fn bad_arguments_credentials_and_existing_outputs_fail_with_json() {
    let ws = Workspace::new();
    fs::write(ws.0.join("existing.zip"), b"preserved").unwrap();
    for args in [
        vec!["balance"],
        vec!["characters", "--limit", "0", "--dry-run"],
        vec!["character", "../secret"],
        vec!["job", ID, "--output", "x.zip"],
        vec!["animate", "--request", "missing.json", "--dry-run"],
        vec!["balance", "--dry-run", "--dry-run"],
        vec!["download", ID, "--output", "existing.zip", "--dry-run"],
        vec!["download", ID, "--output", "new.png", "--dry-run"],
    ] {
        assert_eq!(ws.run(&args, false)["status"], "error");
    }
    assert_eq!(fs::read(ws.0.join("existing.zip")).unwrap(), b"preserved");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(ws.0.join("missing"), ws.0.join("link.zip")).unwrap();
        ws.run(
            &["download", ID, "--output", "link.zip", "--dry-run"],
            false,
        );
    }
}
