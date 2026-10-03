mod arguments;
mod image;
mod image_arguments;
mod image_request;
mod input;
mod output;
mod pixellab;
mod pixellab_arguments;
mod video;
mod video_models;

use serde_json::Value;
use std::{ffi::OsString, io::Write};

const HELP: &str = "SozoCraft agent CLI (image, video, and PixelLab generation)

Usage:
  sozocraft-cli --version (or -V)
  sozocraft-cli image generate --prompt TEXT [options]
  sozocraft-cli video generate --prompt TEXT [options]
  sozocraft-cli video status --operation NAME [--model ID] [--platform ID]
  sozocraft-cli video wait --operation NAME [--model ID] [--platform ID] [--output FILE] [--max-wait SECONDS]

PixelLab (CLI-only; raw PIXELLAB_API_KEY environment token):
  pixellab balance [--dry-run]
  pixellab characters [--limit 1..100] [--offset N] [--dry-run]
  pixellab character UUID [--dry-run]
  pixellab create-character --request FILE [--reference PNG] [--dry-run]
  pixellab animate --request FILE [--dry-run]
  pixellab job UUID [--dry-run]
  pixellab download UUID --output NEW.zip [--dry-run]
  Request files use REST JSON fields. Creation: v3 (8 rotations).
  Animation modes: v3, skeleton-v3, template, pro. Submissions return immediately;
  save operation/operations and characterId. job/download never resubmit.
  Downloads are public and send no credentials. No automatic retries.

Image generate options:
  --provider ID         nano-banana, gpt-image, grok-imagine (default: app config)
  --model ID            Existing app model ID; infers provider if --provider omitted
  --prompt TEXT         Plain prompt text (no DSL expansion)
  --prompt-file FILE    UTF-8 .txt/.md file, or - for stdin; replaces --prompt
  --reference FILE      Repeat for PNG/JPEG references, subject to model limits
  --aspect-ratio RATIO  Model-supported ratio; GPT native uses --size instead
  --size SIZE           Gemini: 512/1K/2K/4K; GPT native: WIDTHxHEIGHT/auto;
                        xAI: 1k/2k; Higgsfield: 1k/2k/4k
  --quality LEVEL       Model-supported quality; Higgsfield Grok: std/pro
  --thinking-level LEVEL Gemini Flash 3.1 only: minimal/high
  --output FILE         New .png file (default: configured filename template)
  --dry-run             Validate without API keys, network, or output writes

Video generate options:
  --model ID            veo-3.1-generate-preview (default), veo-3.1-lite-generate-preview,
                        gemini-omni-1.1-flash, grok-imagine-video-1.5,
                        doubao-seedance-2-0-260128, doubao-seedance-2-0-fast-260128,
                        doubao-seedance-2-0-mini-260615, doubao-seedance-2-5-260628
  --platform ID         Seedance: ark/higgsfield (default: config); Grok: xai;
                        Google: gemini. Required for Seedance status/wait.
  --prompt TEXT          Plain prompt text (no DSL expansion)
  --prompt-file FILE     UTF-8 .txt/.md file, or - for stdin; replaces --prompt
  --start-image FILE     Starting frame; PNG/JPEG, also WebP for Seedance/Grok
  --end-image FILE       Ending frame; requires --start-image
  --reference FILE       Repeat: Veo 3, Omni 6, Grok 7, Seedance 9 (2.5: 30); no Lite
  --duration SECONDS     Veo: 4/6/8; Omni: 3..10; Grok: 1..15; Seedance: 4..15 (2.5: 30)
                        Defaults: Veo 8; all other models 5
  --aspect-ratio RATIO   Model-supported ratio (default 16:9)
  --resolution SIZE      Model-supported size (default: Grok 480p, others 720p)
  --generate-audio BOOL  true/false for Seedance/Grok; unavailable for Google
  --output FILE         New .mp4 file; never overwrites (default: configured output dir)
  --max-wait SECONDS    Local monitoring deadline, 1..3600 (default 900)
  --no-wait             Submit once and return the operation ID
  --dry-run             Validate inputs without credentials, network, or output writes

stdout: newline-delimited JSON except help/version; errors have status=error and exit code 1.
Video generate emits submitted with operation, then completed with outputPath.
Image generate emits started, image events, then completed with outputPaths.
Status emits pending/completed; wait resumes and downloads an existing operation.
Save model, platform, and operation from submitted events for recovery.
Uses ~/.sozocraft/config.toml and the selected provider credentials/platform.
Generation is paid. Stopping local monitoring does not cancel the provider job.
";

pub async fn run(args: impl IntoIterator<Item = OsString>) -> u8 {
    let result = match arguments::parse(args) {
        Ok(arguments::Command::Help) => {
            print!("{HELP}");
            return 0;
        }
        Ok(arguments::Command::Version) => {
            println!("sozocraft-cli {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        Ok(arguments::Command::Image(options)) => image::execute(options).await,
        Ok(arguments::Command::PixelLab(options)) => pixellab::execute(options).await,
        Ok(command) => video::execute(command).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = emit(&serde_json::json!({"status": "error", "error": error}));
            1
        }
    }
}

fn emit(value: &Value) -> Result<(), String> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value).map_err(|_| "Failed to write JSON output.")?;
    writeln!(stdout).map_err(|_| "Failed to write JSON output.")?;
    stdout
        .flush()
        .map_err(|_| "Failed to flush JSON output.".to_string())
}
