mod arguments;
mod video;

use serde_json::Value;
use std::{ffi::OsString, io::Write};

const HELP: &str = "SozoCraft agent CLI (Veo 3.1 / Veo 3.1 Lite / Gemini Omni Flash 1.1)

Usage:
  sozocraft-cli video generate --prompt TEXT [options]
  sozocraft-cli video status --operation NAME [--model ID]
  sozocraft-cli video wait --operation NAME [--model ID] [--output FILE] [--max-wait SECONDS]

Generate options:
  --model ID            veo-3.1-generate-preview (default), veo-3.1-lite-generate-preview,
                        or gemini-omni-1.1-flash
  --prompt TEXT          Plain prompt text (no DSL expansion)
  --prompt-file FILE     UTF-8 .txt/.md file, or - for stdin; replaces --prompt
  --start-image FILE     PNG/JPEG starting frame
  --end-image FILE       PNG/JPEG ending frame; requires --start-image
  --reference FILE       PNG/JPEG asset reference; up to three for Veo, six for Omni; no Lite
  --duration SECONDS     Veo: 4/6/8 (default 8); Omni: 3..10 (default 5)
  --aspect-ratio RATIO   16:9 or 9:16 (default 16:9)
  --resolution SIZE      720p, 1080p, or 4k; Lite has no 4k; Omni also 360p (default 720p)
  --output FILE         New .mp4 file; never overwrites (default: configured output dir)
  --max-wait SECONDS    Local monitoring deadline, 1..3600 (default 900)
  --no-wait             Submit once and return the operation ID
  --dry-run             Validate inputs without credentials, network, or output writes

stdout: newline-delimited JSON; errors have status=error and exit code 1.
Generate emits submitted with operation, then completed with outputPath.
Status emits pending/completed; wait resumes and downloads an existing operation.
Uses ~/.sozocraft/config.toml and configured Gemini proxy/base URL.
Generation is paid. Stopping local monitoring does not cancel the provider job.
";

pub async fn run(args: impl IntoIterator<Item = OsString>) -> u8 {
    let result = match arguments::parse(args) {
        Ok(arguments::Command::Help) => {
            print!("{HELP}");
            return 0;
        }
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
