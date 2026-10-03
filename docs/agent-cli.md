# SozoCraft image, video, and PixelLab CLI for agents

Use `sozocraft-cli image generate` for PNG images through the app's existing
image providers and configured platform. Use `sozocraft-cli video generate`
for MP4 videos. Both run headlessly and reuse the existing Rust adapters.

Video supports the app's Google Veo/Omni, Seedance (Ark or Higgsfield), and Grok
Imagine Video (xAI) models. The default remains `veo-3.1-generate-preview`.

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
pnpm cli:install
sozocraft-cli --help
sozocraft-cli --version
```

`--version` (or `-V`) prints `sozocraft-cli <package version>` and exits without
loading config or contacting providers. The CLI shares the app's package version.

`npm run cli:install` works too. The installer builds a release binary with
`--locked --force`, verifies the installed executable with `--help`, and reports
missing/stale PATH entries. It uses `CARGO_INSTALL_ROOT`, then `CARGO_HOME`, then
`~/.cargo` as the install root; it passes that root explicitly to Cargo rather
than using Cargo's `install.root` configuration. It does not edit shell startup
files, app config, or credentials and does not call generation APIs.

For a custom install root:

```bash
pnpm cli:install --root ./local-cli
# With npm: npm run cli:install -- --root ./local-cli
```

Without pnpm/npm, run `node /path/to/sozocraft/scripts/install-cli.mjs` from any
directory. Relative `--root` paths resolve against your working directory.
The script requires Node, Rust/Cargo, and the app's native build prerequisites;
it uses only Node built-ins and needs no JavaScript dependency installation.

For Rust-only setup without Node, Cargo can install directly:

```bash
cargo install --path src-tauri --bin sozocraft-cli --locked --force
sozocraft-cli --help
```

Cargo builds a release binary and installs it into its bin directory (normally
`~/.cargo/bin`). That directory must be on PATH. Building with `pnpm cli:build`
updates only the repository's debug binary, not a previously installed copy.

## Configuration and credentials

The CLI automatically reads `~/.sozocraft/config.toml`. Configure the selected
provider credentials in the desktop app; the app can remain open or closed.
Google video commands read `[gemini].api_key`, `base_url`, `proxy_url`, `proxy_enabled`,
and `timeout_seconds`, plus `[output].directory` and `[output].template`. An image-style base URL ending
in `/models` is normalized to the API root for video calls.

All three Google video models use the Gemini key even when the app's image API platform is set to
Higgsfield. Video model selection is explicit and does not change app settings.
Image/video commands have no API-key command-line flags or environment overrides.
PixelLab uses its own environment token, described below. Do not print,
copy into prompts, or commit the config file. Missing credentials or invalid
config return a JSON error. `--dry-run` does not require credentials.

For video, if `--output` is omitted, the command uses the desktop's configured output
filename template, with `{provider}` = `google`, `volcengine`, `xai`, or
`higgsfield` for the selected route; `{model}` = the selected API
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

## Image generation

```bash
sozocraft-cli image generate \
  --model gemini-3.1-flash-image-preview \
  --prompt-file ./prompts/contact-pose.txt \
  --reference ./assets/character.png \
  --aspect-ratio 1:1 --size 1K --thinking-level minimal \
  --output ./outputs/contact.png --dry-run
```

Remove `--dry-run` to generate. The command requests one image; there is no batch
flag in this first image workflow. Prompts follow the same plain-text/file/stdin
rules as video. PNG/JPEG references are decoded locally to reject corrupt files,
with a 100 MiB file limit and 256 MiB decoder allocation limit. Reference bytes
are preserved, without desktop JPEG optimization; WebP is unsupported here.

Image commands read the provider/platform settings from the same local config.
Without `--provider` or `--model`, they use `[app].default_provider` and its
configured default model. `--model` infers the provider; an explicit `--provider`
must match. Selecting a different provider without a model uses Nano Banana Pro,
GPT Image 2, or Grok Imagine 2.0, with the corresponding configured platform ID.
The selected provider's platform still comes from config; incompatible model
IDs are rejected. CLI flags do not change config or desktop state.

| Provider | Configured platform | Accepted model IDs | Reference limit |
| --- | --- | --- | --- |
| `nano-banana` | Gemini | `gemini-3-pro-image-preview`, `gemini-3.1-flash-image-preview`, `gemini-2.5-flash-image` | 14 for Pro/3.1 Flash; 3 for 2.5 Flash |
| `nano-banana` | Higgsfield | `nano_banana_2`, `nano_banana_flash`, `nano_banana` | 8 |
| `gpt-image` | OpenAI or OpenRouter | `gpt-image-2`, `gpt-image-2.5-flare`, `gpt-image-2.5-sunburst` | 16 |
| `gpt-image` | OpenRouter | `openai/gpt-image-2`, `openai/gpt-image-2.5-flare`, `openai/gpt-image-2.5-sunburst` | 16 |
| `gpt-image` | Higgsfield | `gpt_image_2`, `gpt_image_2_5_flare`, `gpt_image_2_5_sunburst` | 16 |
| `grok-imagine` | xAI | `grok-imagine-image-2.0` | 5 |
| `grok-imagine` | Higgsfield | `grok_image` | 5 |

Gemini uses `[gemini].api_key`; OpenAI uses `[openai].api_key`; OpenRouter uses
`[openrouter].api_key`; xAI uses `[xai].api_key`. The configured base URL, proxy
toggle/URL, and provider timeout are honored. Higgsfield uses the configured CLI
path, its existing authenticated workspace, and Higgsfield proxy settings.
Image `--dry-run` reads settings but does not require API keys, start provider
processes, make network requests, or create outputs.

Optional image flags are checked against the selected model:

- `--aspect-ratio`: supported model ratios; native GPT uses `--size` instead.
- `--size`: Gemini Pro `1K/2K/4K`; 3.1 Flash also `512`; absent for 2.5 Flash.
  Native GPT accepts `auto` or validated `WIDTHxHEIGHT`, such as `1536x1024`.
  OpenRouter's `openai/...` IDs use aspect ratio and reject size. xAI accepts
  `1k/2k`; Higgsfield accepts `1k/2k/4k` except `nano_banana` and `grok_image`.
- `--quality`: GPT `auto/low/medium/high`, with `xhigh/max` for 2.5 variants.
  Higgsfield GPT excludes `auto`. xAI accepts `auto/low/medium`;
  Higgsfield Grok accepts `std/pro`. Nano Banana has no quality flag.
- `--thinking-level`: `minimal/high` for Gemini 3.1 Flash only.

Omitted image options are left to the provider's defaults; temporary desktop
generation controls are not loaded. Unsupported options fail before generation.

An explicit `--output` must be a new `.png` file. Otherwise, image output uses
the desktop filename template with its image provider/model naming (for example,
`gemini` and `nano-banana-2`), `{id}=001`, a six-character local batch ID, and
`{extension}=png`. Date folders and collision suffixes follow that template.
Output parent/write access is checked before the provider call. Generated PNG
or JPEG data is decoded and re-encoded to PNG before atomic publication.
If the provider returns additional images, all are saved: default names use
successive IDs, and explicit names add `_002`, `_003`, etc. Existing files and
symlinks are never overwritten. No desktop history, PNG metadata, sidecars,
prompt index, or separate Higgsfield archive is written by this workflow.

Image JSON events are:

```json
{"status":"started","mediaType":"image","provider":"nano-banana","model":"gemini-3.1-flash-image-preview","platform":"gemini","requestId":"local-uuid"}
{"status":"image","mediaType":"image","provider":"nano-banana","model":"gemini-3.1-flash-image-preview","requestId":"local-uuid","index":1,"outputPath":"/absolute/path/contact.png","bytes":12345,"width":1024,"height":1024}
{"status":"completed","mediaType":"image","provider":"nano-banana","model":"gemini-3.1-flash-image-preview","platform":"gemini","requestId":"local-uuid","outputPaths":["/absolute/path/contact.png"],"imageCount":1}
```

`started` is emitted before calling the provider; it records a local attempt,
not confirmed provider acceptance. The request ID is not resumable. Image
generation is synchronous and has no `status`, `wait`, or `--no-wait` command.
Save each `image` event: files already published remain if a later image fails.
Dry runs emit one `validated` event with provider, platform, model, options,
reference count, and requested count. Errors use the common JSON/exit-1 format.
Known credentials, prompt text, and reference payloads are redacted from bounded
provider error messages; do not share diagnostics without reviewing them.

The CLI adds no generation retries. The shared Higgsfield adapter retains its
existing bounded create retry policy; direct API image routes submit once.
After an interruption or ambiguous provider error, inspect the provider account
before manually rerunning a paid request. No paid generation is performed by
`--dry-run`; it checks local validation, not remote access or API acceptance.

## PixelLab characters and animation (CLI only)

This integration is intended primarily for AI agents that need scriptable
character creation, rotations, animation, and job recovery. Human users can
create and manage artwork on [the official PixelLab website](https://pixellab.ai).
PixelLab is available only through the headless CLI; SozoCraft's desktop UI has
no PixelLab provider selection or controls.

`sozocraft-cli pixellab` ports the Python workbench into Rust; no Python process,
MCP tool invocation, desktop settings, history, or game assets are involved.
The API origin is fixed to `https://api.pixellab.ai/v2`. Export the **raw**
`PIXELLAB_API_KEY` token in the calling environment; the client adds `Bearer `
only to authenticated request headers. Tokens containing whitespace or an existing
Bearer prefix are rejected. There is no API-key argument or persisted PixelLab
config. The CLI does not source shell files or load `.env` files.

```bash
sozocraft-cli pixellab balance
sozocraft-cli pixellab characters --limit 50 --offset 0
sozocraft-cli pixellab character CHARACTER_UUID
sozocraft-cli pixellab job JOB_UUID
sozocraft-cli pixellab download CHARACTER_UUID --output ./outputs/character.zip
```

`characters` defaults to 50 items at offset 0; limit is 1–100. Character/job IDs
must be UUIDs and are normalized locally. `job` performs one read, returning
`pending`, `completed`, or `failed`. Failed jobs emit their sanitized result and
then the common JSON error with exit code 1. A completed job means remote
completion; use `download` separately to retrieve the ZIP.

Every PixelLab command accepts `--dry-run`. Dry runs validate local inputs, emit
one `validated` JSON event with method, route, model, and sanitized options, and
require no credentials, config, network, or output writes. They do not confirm
account access or model availability. Request files must be regular JSON-object
files, at most 8 MiB; unknown fields and invalid types/ranges are rejected using
a bundled subset of the official REST schema verified on 2026-10-03. Update
`src-tauri/src/pixellab/request_schemas.json` when the supported API changes.
MCP parameters such as `reference_image_url`, `confirm_cost`, and `ai_freedom`
are not REST request fields for these routes.

### Create eight rotations

Save a request file such as `character.json`:

```json
{
  "name": "Warrior rotation trial",
  "description": "An adult swordswoman in a navy robe holding a drawn katana",
  "image_size": {"width": 128, "height": 128},
  "view": "low top-down",
  "template_id": "mannequin",
  "no_background": true,
  "seed": 42
}
```

```bash
sozocraft-cli pixellab create-character --request character.json --dry-run
sozocraft-cli pixellab create-character \
  --request character.json --reference ./assets/south.png --dry-run
```

Creation calls `/create-character-v3`, producing eight directional rotations.
Without a reference, `image_size` ranges from 32–256 pixels on each axis,
with a provider default of 64×64. Non-square sprites are padded to a square.
References must be straight-on, south-facing PNGs, at most 256×256 and 2 MiB.
The CLI decodes them with bounded memory and preserves the original bytes.
Facing is an artistic requirement and cannot be checked locally. Reference-mode
`image_size` is advisory; output geometry follows the uploaded image. Use either
`--reference` or `reference_image` in JSON, never both. Inline images use
`{"base64":"..."}` (raw base64 or a PNG data URL); the same PNG restrictions
apply. Prompt enhancement cannot be combined with a character reference.

### Animate the trial character

Use the actual character UUID in `walk.json`:

```json
{
  "character_id": "123e4567-e89b-12d3-a456-426614174000",
  "animation_name": "Alert walk east",
  "mode": "v3",
  "action_description": "An alert walk in place, preserving costume and sword shape",
  "directions": ["east"],
  "frame_count": 8,
  "keep_first_frame": false
}
```

For the trial's skeleton walk, use:

```json
{
  "character_id": "123e4567-e89b-12d3-a456-426614174000",
  "animation_name": "Template walk east",
  "mode": "skeleton-v3",
  "template_animation_id": "walking-8-frames",
  "directions": ["east"]
}
```

```bash
sozocraft-cli pixellab animate --request walk.json --dry-run
```

Animation calls `/characters/animations`. Supported REST modes are `v3`,
`skeleton-v3` (documented beta), `template`, and `pro`; remote entitlement and
availability still depend on PixelLab. Omitted mode selects `template` when
`template_animation_id` is provided, otherwise `v3`. Template/skeleton modes
require an available template ID and use its frame count. Custom v3/pro modes
require nonempty `action_description` and no template ID. V3 frame count is an
even integer from 4–16, default 8. `keep_first_frame: false` stores exactly the
generated frame count; its default true adds the reference frame. Frame count,
custom start/end frames, `keep_first_frame`, and prompt enhancement are accepted
only for v3. Pro has provider-controlled frame counts and higher costs; this
command does not add an MCP cost-confirmation mechanism.

`directions` accepts distinct compass names: south, north, east, west,
south-east, south-west, north-east, north-west. Omission defaults to south for
custom animation and all character directions for templates. V3
`custom_start_frame` and `end_frame` use inline base64 PNGs and require a single
direction (omission means south). Locally supplied start/end dimensions must
match; matching a stored server rotation is checked by the provider. PNG palette
references (`color_image`) use the same 2 MiB/256×256 CLI limit. Skeleton data
estimation and arbitrary frame-skeleton submission are outside this command group.

### JSON events, recovery, and downloads

Remove `--dry-run` only when a paid submission is intended. Submissions return
immediately without polling or downloading. Save stdout as JSON lines. Creation
emits an event like this (additional sanitized `data` fields may be present):

```json
{"status":"submitted","provider":"pixellab","platform":"pixellab","model":"v3","characterId":"CHARACTER_UUID","operation":"JOB_UUID","operations":null,"animationGroupId":null,"directions":null,"data":{"character_id":"CHARACTER_UUID","background_job_id":"JOB_UUID","status":"processing"}}
```

Animation emits `operations` with **all** per-direction `background_job_ids`,
`directions`, `characterId`, `model`, and `animationGroupId` when returned;
`operation` is null for that multi-job response. Preserve the IDs and check each
with `pixellab job UUID`. To add directions to an existing animation, pass its
`animation_group_id` and repeat its `animation_name` in a new reviewed request;
this is another paid submission, whereas status/download are read-only.
The nested `data` preserves the API's snake_case fields. Read commands emit
`completed` with `data`; job reads also include `operation` and normalize the
provider job state into the event's `status`. Errors use the shared JSON/exit-1
protocol. Credentials, prompt fields, and base64 payloads are removed/redacted
from successful output; provider error bodies are never printed.

Verified HTTPS, a 60-second per-request timeout, disabled redirects, and disabled
retries apply to every PixelLab request. A timeout or interruption during create
may leave accepted jobs running. No request is automatically resubmitted.
If no ID was returned, inspect `characters` and the PixelLab account before
manually submitting again. Multi-direction requests can partially succeed;
inspect the account for accepted directions/jobs even if the HTTP call fails.

ZIP downloads use the public `/characters/{UUID}/zip` endpoint, require no key,
and **never send Authorization**, even when a token is available. HTTP 423 means
assets are still processing; check jobs and retry the download later. A new
explicit `.zip` output path is required. Downloads stream into a hidden temporary
file, enforce a 512 MiB cap and conventional ZIP header/end-record checks, then
publish atomically without overwriting any file or symlink. Parent folders are
created only for real downloads. Normal errors remove the temporary file;
forced termination may leave a `.sozocraft-cli-*.part` file. Publication requires
hard-link support. Archives are saved without extraction or decompression;
these structural checks do not verify every entry's CRC or artistic validity.

The export contains rotation PNGs, animation frames grouped by animation and
direction, and metadata. Preserve canvas sizes and use a fixed pivot/offset when
consuming frames: the reference Godot trial has 128×128 rotations and 192×192
animation canvases. Its 12 FPS is a preview choice, not provider metadata. This
CLI does not normalize sprites, choose timing, or install assets into Godot.

Official references: [REST OpenAPI schema](https://api.pixellab.ai/v2/openapi.json)
and [PixelLab MCP documentation](https://api.pixellab.ai/mcp/docs). REST schemas
are authoritative for request files; MCP descriptions supply workflow context.

## Seedance and Grok video

The model ID selects the video provider. Seedance uses `[ark].api_platform`
(`ark` or `higgsfield`) by default. Optional `--platform` overrides the route
for that command without changing config. Google only accepts `gemini`, and
Grok video only accepts `xai`; the Grok image platform setting does not route
Grok video through Higgsfield.

Ark reads `[ark].api_key`, `base_url`, `proxy_enabled`, `proxy_url`, and
`timeout_seconds`. Grok uses the equivalent `[xai]` settings and key. Seedance
through Higgsfield uses `[higgsfield].cli_path`, its authenticated workspace,
Higgsfield proxy settings, and the app's Ark/Seedance timeout. Dry runs do not
require credentials or execute Higgsfield; Seedance dry runs read settings to
report the configured route.

| Model ID | Duration | Resolution | Maximum references |
| --- | --- | --- | --- |
| `doubao-seedance-2-0-260128` | 4–15 s | 480p/720p/1080p | 9 |
| `doubao-seedance-2-0-fast-260128` | 4–15 s | 480p/720p | 9 |
| `doubao-seedance-2-0-mini-260615` | 4–15 s | 480p/720p | 9 |
| `doubao-seedance-2-5-260628` | 4–30 s | 480p/720p/1080p | 30 |
| `grok-imagine-video-1.5` | 1–15 s | 480p/720p/1080p | 7 |

All five models default to five seconds at 16:9. Seedance defaults to 720p;
Grok defaults to 480p. Seedance ratios: `16:9`, `9:16`, `4:3`, `3:4`, `1:1`,
`21:9`. Grok ratios: `1:1`, `16:9`, `9:16`, `4:3`, `3:4`, `3:2`, `2:3`.
Grok reference and start/end-frame modes support at most 720p. Both providers
accept PNG/JPEG/WebP images and text, starting-frame, frame-pair, or reference
input modes; references cannot be mixed with frames. Ark asset IDs and
video/audio reference inputs are not CLI options in this workflow.

`--generate-audio true|false` controls audio for Seedance/Grok. Omission keeps
the adapter/provider default. Google models reject this flag.

```bash
sozocraft-cli video generate \
  --model doubao-seedance-2-5-260628 \
  --prompt-file ./prompts/pixel-walk.txt \
  --start-image ./assets/contact.png --end-image ./assets/contact.png \
  --duration 5 --resolution 720p --generate-audio false --dry-run

sozocraft-cli video generate \
  --model grok-imagine-video-1.5 \
  --prompt-file ./prompts/pixel-walk.txt \
  --start-image ./assets/contact.png --end-image ./assets/contact.png \
  --duration 5 --resolution 720p --generate-audio false --dry-run
```

Generation emits `submitted` with `provider`, `platform`, `model`, and
`operation`. Store that tuple immediately. For example:

```json
{"status":"submitted","provider":"seedance","platform":"ark","model":"doubao-seedance-2-5-260628","operation":"provider-task-id"}
```

Seedance `status` and `wait` require `--platform ark|higgsfield` from that event.
This keeps recovery on the original provider if config changes later. Grok
status/wait use `xai`; Google uses `gemini`. Use the same model and operation:

```bash
sozocraft-cli video status \
  --model doubao-seedance-2-5-260628 --platform ark \
  --operation 'provider-task-id'
sozocraft-cli video wait \
  --model doubao-seedance-2-5-260628 --platform ark \
  --operation 'provider-task-id' --output ./outputs/walk.mp4
```

Use `--no-wait` to submit once without polling/downloading. Native Ark and xAI
return task/request IDs; Higgsfield returns a job ID. Status and wait never
create a new generation. The existing Higgsfield video adapter may inspect
recent history to recover an ambiguous create result; it does not retry create.
Downloads retain the app's URL, redirect, size, and MP4 validation. A failed
download leaves the provider job available for a later wait; partial output is
removed on normal errors, and existing destinations are never overwritten.

## Video agent workflow

1. Prepare a plain UTF-8 prompt and optional PNG/JPEG images.
2. Run the intended command with `--dry-run` and inspect the JSON result.
3. Submit one generation and capture stdout as JSON lines.
4. Save `model`, `platform`, and `operation` from `submitted` immediately.
5. Use the completed `outputPath` to inspect the MP4 or extract test frames.
6. If local monitoring fails, use `video status` or `video wait` for that same
   tuple. Seedance requires `--platform`. Do not automatically submit again.

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

For Omni sprite keyframes, combine frame images with character/style references.
References retain separate asset roles; the six-reference limit excludes frame
images. This example validates locally without submitting a generation:

```bash
sozocraft-cli video generate \
  --model gemini-omni-1.1-flash \
  --prompt "Animate a seamless pixel walk loop, preserving the reference character." \
  --start-image ./assets/contact.png \
  --end-image ./assets/contact.png \
  --reference ./assets/character-sheet.png \
  --duration 8 --resolution 720p --dry-run
```

### Google input modes and supported options

| Input | Flags | Constraints |
| --- | --- | --- |
| Text to video | No image flags | All models |
| Image to video | `--start-image FILE` | One starting frame |
| Frame pair | `--start-image FILE --end-image FILE` | Ending frame requires starting frame |
| Asset references | Repeat `--reference FILE` | Veo standard: 1–3, cannot mix with frame flags; Omni: 1–6, can mix with starting/ending frames; unsupported by Lite |

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

Except for help (`--help`/`-h`) and version (`--version`/`-V`), stdout contains newline-delimited JSON, flushed after each
event. Parse each line independently; do not treat all stdout as one JSON object.
Exit code 0 means the command succeeded, including a pending `status` result or
a submitted `--no-wait` job. Exit code 1 means an argument, validation, config,
provider, monitoring, or download error. Provider failure is also an error.

Normal blocking generation emits:

```json
{"status":"submitted","provider":"google-veo","platform":"gemini","model":"veo-3.1-generate-preview","operation":"models/veo-3.1-generate-preview/operations/example"}
{"status":"completed","provider":"google-veo","platform":"gemini","model":"veo-3.1-generate-preview","operation":"models/veo-3.1-generate-preview/operations/example","outputPath":"/absolute/path/pixel-walk.mp4","bytes":123456}
```

Video `--dry-run` emits one `validated` event containing `provider`, `platform`, `model`, `inputMode`,
`options`, and `inputImageCount`, with no network requests or output writes.
`status` emits `pending` or `completed` with the model and operation;
`completed` from `status` means the provider has finished, not that a local
file has been downloaded. Errors emit:

```json
{"status":"error","error":"Human-readable error message"}
```

Capture events even when the process exits nonzero: a submitted operation may
still be running. Successful events do not contain prompts, image payloads,
credentials, or temporary video URLs. The selected provider key is redacted from
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
