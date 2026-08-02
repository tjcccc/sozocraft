# SozoCraft

SozoCraft is a macOS-first, cross-platform desktop AI visual generation studio and prompt management workspace.

The name comes from Japanese `sōzō` / `souzou`, evoking both imagination
(`想像`) and creation (`創造`).

SozoCraft is now beyond its early MVP stage. The project is in active pre-1.0
product hardening, with the remaining work focused on reliability, packaging,
workflow polish, and completing the video-generation track.

The current app includes:

- Tauri 2 desktop shell
- React + TypeScript frontend
- Rust backend
- Gemini and Higgsfield CLI image generation for Nano Banana / Nano Banana Pro
- OpenAI GPT-Image text-to-image generation
- xAI Grok Imagine text-to-image generation
- Seedance, Grok Imagine, and Google Veo text-, image-, and reference-to-video
  generation with local MP4 output
- local output saving
- recent generation history
- plain prompt editing with a future-ready PromptCraft DSL path

## Requirements

- Node.js 24+
- pnpm 10+
- Rust 1.94+
- Tauri desktop prerequisites for your OS

Linux support is intended to target Fedora Wayland first.

On Fedora, install the native Tauri/WebKitGTK development packages before
running `pnpm tauri:dev`:

```bash
sudo dnf install webkit2gtk4.1-devel \
  openssl-devel \
  curl \
  wget \
  file \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  libxdo-devel
sudo dnf group install "c-development"
```

## Run Locally

Install dependencies:

```bash
pnpm install
```

Run the desktop app:

```bash
pnpm tauri:dev
```

Build a macOS `.app` bundle:

```bash
pnpm tauri:build:app
```

Frontend-only development:

```bash
pnpm dev
```

## Configuration

Open the settings button in the top toolbar to switch to the full-page settings view and configure:

- Gemini API key
- OpenAI API key
- xAI API key
- Seedance video platform and default model (Volcengine Ark or Higgsfield CLI)
- Volcengine Ark API key when Seedance uses Ark
- default image model per provider
- output directory
- optional Gemini-compatible base URL
- optional OpenAI-compatible base URL
- optional xAI-compatible base URL
- optional Volcengine Ark base URL
- optional Higgsfield CLI path and API platform routing for supported image and Seedance video providers
- optional proxy URL, for example `http://127.0.0.1:7890`
- provider proxy toggles
- provider timeouts
- prompt library directory
- prompt editor-only mode and prompt preview placement

SozoCraft stores local configuration in:

```text
~/.sozocraft/config.toml
```

Generation failures are appended to:

```text
~/.sozocraft/error.log
```

The log is JSON-lines formatted and is local-only. It records timestamps, batch
ids, model/proxy settings, failure class, and sanitized provider error details;
it does not record API keys or prompt text.

Example:

```toml
[gemini]
api_key = "your_gemini_api_key_here"
default_model = "gemini-3-pro-image-preview"
base_url = "https://generativelanguage.googleapis.com/v1beta/models"
proxy_url = "http://127.0.0.1:7890"
proxy_enabled = true
timeout_seconds = 180

[openai]
api_key = "your_openai_api_key_here"
api_platform = "openai"
default_model = "gpt-image-2"
base_url = "https://api.openai.com/v1"
proxy_enabled = true
timeout_seconds = 180

[openrouter]
api_key = "your_openrouter_api_key_here"
base_url = "https://openrouter.ai/api/v1"

[xai]
api_key = "your_xai_api_key_here"
default_model = "grok-imagine-image-quality"
base_url = "https://api.x.ai/v1"
proxy_enabled = true
timeout_seconds = 180

[ark]
api_key = "your_ark_model_api_key_here"
api_platform = "ark"
default_model = "doubao-seedance-2-0-260128"
base_url = "https://ark.cn-beijing.volces.com/api/v3"
proxy_enabled = true
timeout_seconds = 180

[higgsfield]
cli_path = "higgsfield"
proxy_enabled = true
proxy_url = "http://127.0.0.1:7890"
output_enabled = false
output_directory = "/Users/you/Pictures/Higgsfield"
output_template = "{yyMMdd} {id:3} {higgsfield_filename}"

[output]
directory = "/Users/you/Pictures/SozoCraft"
template = "{yyMMdd}/{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}"

[prompts]
directory = "/Users/you/.sozocraft/prompts"
dsl_enabled = true
editor_only = false
preview_placement = "bottom"
```

The config file is local-only and must not be committed. API keys are never stored in project files.

Non-secret app state is stored in the platform local data directory under `SozoCraft/state.json`. Generated images and videos are saved to the configured output directory.

## Prompt Library

SozoCraft can work as a standalone local prompt editor. Prompt files are
markdown files stored in the configured prompt directory, which defaults to:

```text
~/.sozocraft/prompts
```

Prompt files are stored as UUID-named markdown files, for example
`3e2f...a91.md`. The markdown file contains the editable prompt body only.
Title, tags, timestamps, and search metadata are stored in:

```text
~/.sozocraft/prompts.sqlite
```

The database can be rebuilt by rescanning the prompt folder. Legacy prompt files
with YAML frontmatter are still readable; SozoCraft strips that frontmatter from
the editor and imports its metadata into the database when possible.

```markdown
character = cinematic portrait of a young woman
location = neon street at night
style = soft rim light, shallow depth of field, highly detailed

prompt = {
A realistic photo.
{character}. She is in a {location}.
{style}.
}
```

The preview and generation path render the first `prompt = { ... }` or
`prompt { ... }` block, replacing simple `{variable}` placeholders from
`name = value` lines above it. If a file has no prompt block, the markdown body
is used as plain prompt text. Generated PNG metadata stores the rendered model
prompt in `prompt` and the markdown source in `sozocraft.promptSnapshot`.

DSL mode also supports prompt includes with `{# ... }` references:

```text
{#identify_ref}
{# title with spaces}
{# nano-banana/identity:identify_ref}
{# "nano banana"/sub1/sub2:"ai girl"}
```

The text after `#` is trimmed. Slash-separated references such as
`{# nano-banana/identity}` match an exact prompt tag path. Scoped references use
a colon between the tag path and prompt title, such as
`{# nano-banana/identity:identify_ref}`. Use single or double quotes around tag
segments or titles that contain spaces. Plain references first match prompt
titles, then tag leaf names, then the first library search result. If multiple
prompts match, the most recently updated one is used. If no prompt matches, the
include marker is left unchanged. The older `{# tag/path/title}` scoped form is
still accepted as a deprecated fallback.

DSL mode omits `//` line comments from rendered prompts, so
`// {# identify_ref}` comments out the include directive.

Prompt tags support slash-separated nesting such as
`#nano-banana/aigirl/outdoor-photo`, which the library displays as expandable
folders.

The Prompt Editor header includes a DSL toggle. When DSL is on, the preview and
generation path use the variable/block renderer described above. When DSL is
off, SozoCraft sends the editor text exactly as written.

While editing a prompt, `Cmd+F` / `Ctrl+F` opens editor search and `Cmd+R` /
`Ctrl+R` opens replace. Prompt editor search, replace, title, and prompt-list
search fields disable autocorrect, autocapitalize, and spellcheck.

## Gemini Models

The Gemini provider enables these models:

- `gemini-3-pro-image-preview`
- `gemini-3.1-flash-image-preview`
- `gemini-2.5-flash-image`

The backend uses the Gemini `generateContent` REST API and extracts image bytes from inline image response parts.

Generation controls are model-aware:

- Gemini 3.1 Flash Image Preview supports aspect ratios `1:1`, `1:4`, `1:8`,
  `2:3`, `3:2`, `3:4`, `4:1`, `4:3`, `4:5`, `5:4`, `8:1`, `9:16`, `16:9`,
  and `21:9`; image sizes `512`, `1K`, `2K`, and `4K`; and thinking levels
  `minimal` or `high`. The UI accepts up to 14 reference images.
- Gemini 3 Pro Image Preview supports aspect ratios `1:1`, `2:3`, `3:2`,
  `3:4`, `4:3`, `4:5`, `5:4`, `9:16`, `16:9`, and `21:9`; image sizes
  `1K`, `2K`, and `4K`; thinking is model-managed. The UI accepts up to 14
  reference images.
- Gemini 2.5 Flash Image supports aspect ratios `1:1`, `2:3`, `3:2`, `3:4`,
  `4:3`, `4:5`, `5:4`, `9:16`, `16:9`, and `21:9`; image size is
  model-managed. The UI accepts up to 3 reference images.

Reference images are selected with the native file picker and sent to Gemini as
inline image data. Supported picker formats are PNG, JPEG, and WebP.

When the Nano Banana API platform is set to Higgsfield CLI, SozoCraft routes
generation through the installed `higgsfield` command and follows the
CLI-reported model options. The app forwards its configured provider proxy to
the CLI process, saves returned result images locally, and keeps generated PNG
metadata compatible with the rest of the output workflow.

Before provider requests, SozoCraft optimizes large PNG/JPEG reference images
into high-quality JPEGs when the image has no alpha channel. The default keeps a
maximum long edge of 1600 px and JPEG quality 85, which reduces upload size and
timeout risk while preserving recognizability for photo references. WebP and
transparent PNG inputs are left unchanged. Optimized reference images are cached
under:

```text
~/.sozocraft/cache/reference-images
```

The cache is content-addressed, so reusing the same reference image can reuse
the same optimized bytes across generation runs. Cache entries older than about
30 days are cleaned up opportunistically.

## GPT-Image Models

The GPT-Image tab is wired for OpenAI text-to-image and reference-image
generation:

- `gpt-image-2`

The current implementation enables `gpt-image-2` only, requests PNG output, and
offers OpenAI's documented popular generation `size` values: `auto`,
`1024x1024`, `1536x1024`, `1024x1536`, `2048x2048`, `2048x1152`,
`3840x2160`, and `2160x3840`. The backend accepts any `gpt-image-2`
resolution that fits OpenAI's documented constraints: maximum edge up to
3840px, edges divisible by 16, long-to-short edge ratio no greater than 3:1,
and total pixels between 655,360 and 8,294,400. The UI labels fixed sizes with
their aspect ratios, such as `2:3 (1024x1536)`. It accepts OpenAI Image API `data[].b64_json`
responses, OpenAI Responses API `image_generation_call.result` image payloads,
and OpenRouter chat image outputs under `choices[].message.images[]`. When an
OpenRouter model API page URL such as
`https://openrouter.ai/openai/gpt-5.4-image-2/api` is configured as the OpenAI
base URL, SozoCraft routes the request through OpenRouter's
`/api/v1/chat/completions` endpoint with image modalities, maps selected OpenAI
sizes to OpenRouter `image_config` aspect-ratio and 1K/2K/4K size buckets, and
uses the model slug from that page URL.

GPT-Image reference-image runs support up to 16 PNG, JPEG, or WebP inputs. For
OpenAI-compatible Image API endpoints, SozoCraft sends reference-image runs to
`/images/edits` as multipart `image[]` inputs. For OpenRouter endpoints, it
sends reference images as chat message `image_url` data URLs.

GPT-Image can also be routed through Higgsfield CLI from the provider platform
setting. Higgsfield CLI requests use CLI-supported aspect ratios, 1K/2K/4K
resolution buckets, low/medium/high quality options, and app-configured proxy
environment forwarding.

## Grok Imagine Models

The Grok Imagine tab enables xAI text-to-image generation through
`/v1/images/generations`:

- `grok-imagine-image-quality`
- `grok-imagine-image`

The current implementation requests `b64_json` responses so generated images
can be saved locally with the same SozoCraft PNG metadata path as other
providers. The Grok Imagine tab supports xAI aspect ratios, `1k`/`2k`
resolution, the documented `quality` field, and up to 5 uploaded reference
images through xAI's JSON image edit endpoint. Mask editing is intentionally not
implemented yet.

Grok Imagine can also be routed through Higgsfield CLI. In that mode SozoCraft
uses the CLI-supported Grok Image aspect ratios and maps Standard/Quality to the
CLI `std`/`pro` modes.

## Video Providers

Video mode reuses the existing prompt library/editor and presents providers in
the order Seedance, Grok Imagine, and Google Veo. Each provider keeps its own
controls and input-image selection when switching tabs. One shared Input Images
interaction uses per-thumbnail roles: uploads default to Reference, and a hover
or keyboard-focus menu can assign Start frame for every provider plus End frame
for Seedance and Veo. Reference thumbnails remain unlabelled; explicit Start and
End assignments receive compact badges. Start/end-frame pairs and reference
workflows stay mutually exclusive.

Seedance can use Volcengine Ark directly or route through the authenticated
Higgsfield CLI. The Settings view persists both the Seedance API platform and
the default video model. The UI model ids are `doubao-seedance-2-0-260128`,
`doubao-seedance-2-0-fast-260128`, and `doubao-seedance-2-0-mini-260615`.
All three support up to nine reference images or a strict start/end-frame pair,
4–15 second output, six aspect ratios, and an audio-generation toggle. The
flagship model offers 480p/720p/1080p; Fast and Mini offer 480p/720p. Configure
the separate Ark API key in Settings for direct Ark use. Higgsfield maps these
choices to `seedance_2_0 --mode std`, `seedance_2_0 --mode fast`, and
`seedance_2_0_mini`; local Start, End, and Reference images are auto-uploaded by
the CLI. Higgsfield requires a selected billing workspace (`higgsfield workspace set`).
The direct Ark, xAI, and Gemini API video routes form the supported common-API
checkpoint. Higgsfield Seedance video routing is currently experimental: the
Standard path and CLI job/result handling have initial coverage, while broader
end-to-end Standard/Fast/Mini and recovery-path testing remains.
If Higgsfield accepts a video create but its CLI loses the response, SozoCraft
does not retry the paid create. It searches recent video jobs and resumes only
when one newly created job exactly matches the request signature. Stop is
disabled for every running Higgsfield CLI image or video job because the CLI has
no cancellation command.
[Higgsfield CLI](https://github.com/higgsfield-ai/cli) documents the command
workflow and [its model schemas](https://github.com/higgsfield-ai/cli/blob/main/MODELS.md)
document the Seedance media inputs. Soul IDs are currently image-model inputs,
not Seedance video parameters.
[Seedance 2.0's official API launch notes](https://developer.volcengine.com/articles/7628567056649125942)
describe its multimodal reference workflow; real-person inputs may additionally
require a [trusted, authorized Ark asset](https://www.volcengine.com/docs/82379/2315856?lang=zh).

Ark asset-management support remains implemented behind the native boundary,
but its Settings controls and Seedance picker are currently hidden because
virtual identity assets require an enterprise-verified Volcengine account.
Regular uploaded Seedance references remain available.

Google Veo reuses the configured Gemini API key and base URL. The first model is
`veo-3.1-generate-preview`, with 4/6/8-second output at 720p, 8-second output at
1080p or 4K, landscape or portrait ratios, one starting image, a start/end-frame
pair, and up to three reference images. Reference-image runs are fixed to eight
seconds, and Veo's native audio is always enabled. See Google's
[Veo 3.1 Gemini API guide](https://ai.google.dev/gemini-api/docs/veo).

Grok Imagine uses the xAI provider configuration and model
`grok-imagine-video`. Its one-image and reference behavior remains unchanged.

xAI's [image-to-video documentation](https://docs.x.ai/developers/model-capabilities/video/image-to-video)
defines one source image, while its
[reference-to-video documentation](https://docs.x.ai/developers/model-capabilities/video/reference-to-video)
defines up to seven reference images and a maximum duration of 10 seconds.
Text and starting-frame modes support durations from 1 to 15 seconds. All modes
offer aspect ratios `1:1`, `16:9`, `9:16`, `4:3`, `3:4`, `3:2`, and `2:3`, and
`480p` or `720p` resolution.

Video generation is asynchronous. SozoCraft starts the provider job, polls its
status, downloads the temporary result URL immediately, and saves the MP4 to
the configured output directory. The shared generation queue serializes image
and video tasks. Stopping a video task ends local monitoring only; the remote
provider job may keep processing and charging.

For playback, SozoCraft grants Tauri's asset protocol access only to the exact
generated MP4 selected from local history. The generated video is accompanied
by an `.mp4.json` metadata file containing the input mode, input-image filenames
and MIME types when applicable, prompt snapshot, rendered prompt, model options,
ids, timestamps, and sanitized provider metadata. Input-image bytes
are not persisted in the metadata.

## Output Metadata

Generated images are always saved as PNG files. Non-PNG provider responses are
converted before writing so SozoCraft can embed stable UTF-8 PNG `iTXt`
metadata:

- `prompt`: final rendered prompt sent to the image model
- `sozocraft`: JSON metadata with `schemaVersion`, `promptSnapshot`,
  `renderedPrompt`, provider/model/options, batch/image ids, timestamps, and
  provider response metadata

Generated videos are saved as MP4 files with adjacent `.mp4.json` metadata.
Temporary provider URLs and API credentials are not retained in video metadata.

`promptSnapshot` is the original SozoCraft source prompt. Today it matches the
plain prompt text; when the PromptCraft DSL is added, it will store the DSL
source while `prompt` and `renderedPrompt` store the rendered output prompt.

## Output Filename Template

Default:

```text
{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}
```

Supported variables:

- `{provider}`
- `{model}`
- `{datetime}` or `{datetime:yyyyMMdd_HHmmss}`
- date folder tokens such as `{yyMMdd}` and `{yyyyMMdd}`
- `{id}` or a custom numeric width such as `{id:2}` or `{id:4}`
- `{batch_id}`
- `{extension}`

Provider and model values are sanitized for filesystem safety. For Gemini generation, filename aliases use `gemini` as provider and names such as `nano-banana-2` as model. `{id}` is the batch-local image order, for example `001`. Existing files are not overwritten; SozoCraft appends a numeric suffix when needed.

General Settings also offers an optional Higgsfield Output archive. When enabled,
successful Higgsfield CLI image and video jobs keep their normal SozoCraft output
and write a second copy in the configured Higgsfield directory. Image archives
preserve the raw provider bytes before SozoCraft converts them to PNG; video
archives preserve the downloaded MP4. Its template supports the variables above
plus `{higgsfield_filename}`, which is the sanitized basename supplied by
Higgsfield, for example:

```text
{yyMMdd} {id:3} {higgsfield_filename}
```

The secondary directory must be absolute and its template cannot escape that
directory. A secondary-copy failure is logged and recorded in output metadata but
does not turn an otherwise successful generation into a failure.
For templates containing `{id}` or `{id:N}`, SozoCraft scans matching files in
the rendered date/folder scope and continues after the highest existing serial;
for example, an existing `004` makes the next archive use `005`.

The Higgsfield CLI settings include their own Use proxy toggle and optional
Proxy URL. When enabled with an empty dedicated URL, Higgsfield falls back to
the General Proxy URL. The effective proxy is applied only to Higgsfield child
processes and result download clients; it is not exported into the parent shell.

## Validation

```bash
pnpm typecheck
pnpm test:frontend
pnpm build
cd src-tauri && cargo test
```

## Roadmap Notes

- PromptCraft DSL parsing/rendering and validation
- Prompt Bridge local HTTP server for external tools
- ComfyUI local backend
- provider-specific mask editing flows for GPT-Image and Grok Imagine
- Grok Imagine video editing and extension workflows
- restart-safe recovery for in-flight video jobs
