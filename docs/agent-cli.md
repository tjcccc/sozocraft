# SozoCraft video CLI for agents

Use `sozocraft-cli video generate` to generate an MP4 without opening the desktop
app. Supported models are `veo-3.1-generate-preview` (default),
`veo-3.1-lite-generate-preview`, and `gemini-omni-1.1-flash`. Generation uses the existing Rust provider adapters.

## Build and run

From the repository root:

```bash
pnpm cli:build
./src-tauri/target/debug/sozocraft-cli --help
```

The standalone binary is `src-tauri/target/debug/sozocraft-cli` (add `.exe` on
Windows). It can run from another working directory; use its absolute path.
Relative prompt, image, and output paths resolve against the working directory.
The build requires the same Rust and native Tauri prerequisites as the app, but
running the CLI does not create a window. `pnpm cli video generate ...` also
builds and runs the command from this repository.

For an optimized binary:

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml --bin sozocraft-cli
./src-tauri/target/release/sozocraft-cli --help
```

To install or update the command on PATH (including a stale existing binary):

```bash
cargo install --path src-tauri --bin sozocraft-cli --locked --force
sozocraft-cli --help
```

Cargo builds a release binary and installs it into its bin directory (normally
`~/.cargo/bin`). That directory must be on PATH. Building with `pnpm cli:build`
updates only the repository's debug binary, not a previously installed copy.

## Configuration and credentials

The CLI automatically reads `~/.sozocraft/config.toml`. Configure the Gemini
API key in the desktop app, then close or leave the app open as desired.
The CLI reads `[gemini].api_key`, `base_url`, `proxy_url`, `proxy_enabled`,
and `timeout_seconds`, plus `[output].directory` and `[output].template`. An image-style base URL ending
in `/models` is normalized to the API root for video calls.

All three models use the Gemini key even when the app's image API platform is set to
Higgsfield. CLI model selection is explicit and does not change app settings.
There are no API-key command-line flags or environment overrides. Do not print,
copy into prompts, or commit the config file. Missing credentials or invalid
config return a JSON error. `--dry-run` does not load credentials.

If `--output` is omitted, the command uses the desktop's configured output
filename template, with `{provider}` = `google`, `{model}` = the selected API
model ID, `{batch_id}` = a six-character local batch ID, `{id}` = `001`,
`{extension}` = `mp4`, and local time for date tokens. For example, the template:

```text
{yyMMdd}/{datetime:yyyyMMdd_HHmmss}_{provider}_{model}_{batch_id}_{id}
```

produces `<output dir>/261001/20261001_140000_google_gemini-omni-1.1-flash_a1b2c3_001.mp4`.
The extension is appended if the template omits `{extension}`. Date folders
follow the configured template; custom templates are honored without adding an
extra folder. Existing default filenames receive the same numeric collision
suffixes as the desktop. An explicit `--output` bypasses the template and must
have an `.mp4` extension. Parent directories are created and write
access is checked before submitting a generation. Existing files and symlinks
are never overwritten. The final file is published only after a validated
download; the output filesystem must support hard links.

CLI generation does not change desktop history, prompt files, or the SQLite
index. It returns output metadata as JSON events and does not create the
desktop `.mp4.json` sidecar. Save stdout if you need a durable job/result record.

## Agent workflow

1. Prepare a plain UTF-8 prompt and optional PNG/JPEG images.
2. Run the intended command with `--dry-run` and inspect the JSON result.
3. Submit one generation and capture stdout as JSON lines.
4. Save the `model` and `operation` from the `submitted` event immediately.
5. Use the completed `outputPath` to inspect the MP4 or extract test frames.
6. If local monitoring fails, use `video status` or `video wait` for that same
   model and operation. Do not automatically submit the generation again.

The CLI sends prompt text as written. It does not expand PromptCraft DSL or
library includes. Choose exactly one of `--prompt TEXT` and `--prompt-file FILE`.
Prompt files must be `.txt`, `.md`, or `.markdown`; `--prompt-file -` reads stdin.
The prompt must be nonempty and at most 50,000 UTF-8 bytes. Use a prompt file to
avoid shell quoting issues with longer text.

### Pixel-animation test with Veo 3.1

```bash
./src-tauri/target/debug/sozocraft-cli video generate \
  --model veo-3.1-generate-preview \
  --prompt "Animate this pixel-art character walking in place. Keep the camera fixed, preserve the character design and crisp pixel edges, use a solid background, and keep the first and last pose consistent for a loop." \
  --start-image ./assets/character.png \
  --duration 8 \
  --resolution 720p \
  --aspect-ratio 16:9 \
  --output ./outputs/pixel-walk-veo.mp4 \
  --dry-run
```

Remove `--dry-run` to submit the paid generation. A video generation is a visual
test artifact; inspect its frames before using them as a game animation. The
CLI does not enforce pixel-grid alignment, extract sprites, or remove backgrounds.

### Lower-cost test with Veo 3.1 Lite

```bash
./src-tauri/target/debug/sozocraft-cli video generate \
  --model veo-3.1-lite-generate-preview \
  --prompt-file ./prompts/pixel-walk.txt \
  --start-image ./assets/character.png \
  --end-image ./assets/character.png \
  --duration 4 \
  --resolution 720p \
  --output ./outputs/pixel-walk-lite.mp4
```

Lite supports text, a starting frame, or a start/end-frame pair; `--reference`
and 4K are rejected. It supports 4/6/8 seconds at 720p and eight seconds at
1080p, with the same aspect ratios and native audio as standard Veo. Its default
is eight seconds at 720p. Passing the same frame at both ends can guide a loop,
but the generated motion still needs visual inspection.

### Same test with Gemini Omni Flash 1.1

```bash
./src-tauri/target/debug/sozocraft-cli video generate \
  --model gemini-omni-1.1-flash \
  --prompt-file ./prompts/pixel-walk.txt \
  --start-image ./assets/character.png \
  --duration 5 \
  --resolution 720p \
  --output ./outputs/pixel-walk-omni.mp4
```

### Input modes and supported options

| Input | Flags | Constraints |
| --- | --- | --- |
| Text to video | No image flags | All models |
| Image to video | `--start-image FILE` | One starting frame |
| Frame pair | `--start-image FILE --end-image FILE` | Ending frame requires starting frame |
| Asset references | Repeat `--reference FILE` | Veo standard: 1–3; Omni: 1–6; unsupported by Lite; cannot mix with frame flags |

| Option | Veo 3.1 | Omni Flash 1.1 |
| --- | --- | --- |
| Duration | 4, 6, or 8 seconds; default 8 | 3–10 seconds; default 5 |
| Resolution | 720p, 1080p, 4k; default 720p | 360p, 720p, 1080p, 4k; default 720p |
| Aspect ratio | 16:9 or 9:16; default 16:9 | Same |
| Additional limits | 1080p/4k and asset references require 8 seconds | No additional duration restriction |
| Audio | Native audio; no CLI toggle | Native audio; no CLI toggle |

Images must be regular PNG/JPEG files with matching signatures and pass the
backend's size limits. WebP is rejected by the Google video boundary. Input
images are sent without the desktop's automatic JPEG optimization, preserving
the uploaded pixel-art bytes.

## JSON protocol and exit codes

Except for `--help`, stdout contains newline-delimited JSON, flushed after each
event. Parse each line independently; do not treat all stdout as one JSON object.
Exit code 0 means the command succeeded, including a pending `status` result or
a submitted `--no-wait` job. Exit code 1 means an argument, validation, config,
provider, monitoring, or download error. Provider failure is also an error.

Normal blocking generation emits:

```json
{"status":"submitted","model":"veo-3.1-generate-preview","operation":"models/veo-3.1-generate-preview/operations/example"}
{"status":"completed","model":"veo-3.1-generate-preview","operation":"models/veo-3.1-generate-preview/operations/example","outputPath":"/absolute/path/pixel-walk.mp4","bytes":123456}
```

`--dry-run` emits one `validated` event containing `model`, `inputMode`,
`options`, and `inputImageCount`, with no network requests or output writes.
`status` emits `pending` or `completed` with the model and operation;
`completed` from `status` means the provider has finished, not that a local
file has been downloaded. Errors emit:

```json
{"status":"error","error":"Human-readable error message"}
```

Capture events even when the process exits nonzero: a submitted operation may
still be running. Successful events do not contain prompts, image payloads,
credentials, or temporary video URLs. The local Gemini key is redacted from
provider errors. Avoid sharing provider error text without reviewing it.

## Submit, inspect, and resume

For an agent with short tool deadlines, submit Veo without local monitoring:

```bash
./src-tauri/target/debug/sozocraft-cli video generate \
  --model veo-3.1-generate-preview \
  --prompt-file ./prompts/pixel-walk.txt \
  --start-image ./assets/character.png \
  --no-wait
```

`--no-wait` emits only `submitted`; it cannot be combined with `--output` or
`--max-wait`. Omni's Interactions adapter completes generation synchronously
before returning a file ID, so Omni `--no-wait` can still take several minutes.
It skips local file polling/download after the ID is returned.

Pass the exact returned model and operation to both commands:

```bash
./src-tauri/target/debug/sozocraft-cli video status \
  --model veo-3.1-generate-preview \
  --operation 'models/veo-3.1-generate-preview/operations/example'

./src-tauri/target/debug/sozocraft-cli video wait \
  --model veo-3.1-generate-preview \
  --operation 'models/veo-3.1-generate-preview/operations/example' \
  --output ./outputs/pixel-walk.mp4 \
  --max-wait 900
```

For Omni, the returned operation is a Google file ID. Use that ID unchanged with
`--model gemini-omni-1.1-flash`. These commands use the same config and never
create a new generation. `wait` emits one `completed` event after downloading.

The default local monitoring deadline is 900 seconds, configurable from 1 to
3600 with `--max-wait`. It starts after the provider returns an operation/file
ID, bounds the polling loop, and excludes submission and download time. Each
Veo request/download also uses the configured Gemini timeout. Omni submission
uses the existing adapter's minimum 900-second request timeout.

There are no automatic create retries or provider cancellation commands. A
timeout or process interruption can leave a paid provider job running. If the
submission fails before returning an ID, the provider may still have accepted
it; inspect the provider account before manually retrying. If the process is
forcibly terminated, a hidden `.sozocraft-cli-*.part` file may remain in the
output directory. Normal errors remove that temporary file.

Model/API references: [Veo video generation](https://ai.google.dev/gemini-api/docs/veo)
and [Gemini Omni Flash](https://ai.google.dev/gemini-api/docs/models/gemini-omni-flash).

## Troubleshooting Omni HTTP errors

Omni errors include Google's structured status, message, and any field violations
when supplied. The CLI redacts the local API key and known prompt/image payloads,
limits the message length, and does not dump the full provider response. A proxy
returning HTML or unstructured text produces a generic error instead.

HTTP 400 alone does not identify the cause. Read the returned diagnostic before
changing duration, image inputs, or config; supported defaults can still fail
because of account access, input validation, or a gateway's API compatibility.
`--dry-run` checks local validation only, not remote API access or acceptance.
Capture the failing command's JSON error and model/options; avoid repeated
generation attempts while diagnosing. Rebuild with `pnpm cli:build` after
updating source. If using a separately installed binary, reinstall it as well.

URI delivery requires `store=true` for Omni. SozoCraft enables interaction
storage while keeping `background=false` and `stream=false`; prompts and input
media therefore belong to a stored Google interaction. The reported API error
confirmed this requirement even though Google's general synchronous-generation
recommendation says `store=false`. `--duration N` remains mapped to the documented
`response_format.duration` string `"Ns"`; it is not appended to the prompt.
See the [Interactions API video response format](https://ai.google.dev/api/interactions-api).
