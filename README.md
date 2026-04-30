# SozoCraft

SozoCraft is a macOS-first, cross-platform desktop AI visual generation studio and prompt management workspace.

The name comes from Japanese `sōzō` / `souzou`, evoking both imagination
(`想像`) and creation (`創造`).

This repository currently targets the `0.1.0` MVP:

- Tauri 2 desktop shell
- React + TypeScript frontend
- Rust backend
- Gemini image generation for Nano Banana / Nano Banana Pro
- OpenAI GPT-Image text-to-image generation
- xAI Grok Imagine text-to-image generation
- local output saving
- recent generation history
- plain prompt editing with a future-ready PromptCraft DSL path

## Requirements

- Node.js 24+
- pnpm 10+
- Rust 1.94+
- Tauri desktop prerequisites for your OS

Linux support is intended to target Fedora Wayland first.

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
- default Gemini image model
- output directory
- optional Gemini-compatible base URL
- optional OpenAI-compatible base URL
- optional xAI-compatible base URL
- optional proxy URL, for example `http://127.0.0.1:7890`
- timeout

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
timeout_seconds = 180

[openai]
api_key = "your_openai_api_key_here"
default_model = "gpt-image-2"
base_url = "https://api.openai.com/v1"

[xai]
api_key = "your_xai_api_key_here"
default_model = "grok-imagine-image"
base_url = "https://api.x.ai/v1"

[output]
directory = "/Users/you/Pictures/SozoCraft"
template = "{yyMMdd}/{provider}_{model}_{datetime:yyyyMMdd_HHmmss}_{id}.{extension}"
```

The config file is local-only and must not be committed. API keys are never stored in project files.

Non-secret app state is stored in the platform local data directory under `SozoCraft/state.json`. Generated images are saved to the configured output directory.

## Gemini Models

The MVP enables these models:

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

## GPT-Image Models

The GPT-Image tab is wired for OpenAI text-to-image and reference-image
generation:

- `gpt-image-2`

The current implementation enables `gpt-image-2` only, requests PNG output, and
uses OpenAI's documented generation `size` values: `auto`, `1024x1024`,
`1536x1024`, and `1024x1536`. It accepts OpenAI Image API `data[].b64_json`
responses, OpenAI Responses API `image_generation_call.result` image payloads,
and OpenRouter chat image outputs under `choices[].message.images[]`. When an
OpenRouter model API page URL such as
`https://openrouter.ai/openai/gpt-5.4-image-2/api` is configured as the OpenAI
base URL, SozoCraft routes the request through OpenRouter's
`/api/v1/chat/completions` endpoint with image modalities and uses the model
slug from that page URL.

GPT-Image reference-image runs support up to 16 PNG, JPEG, or WebP inputs. For
OpenAI-compatible Image API endpoints, SozoCraft sends reference-image runs to
`/images/edits` as multipart `image[]` inputs. For OpenRouter endpoints, it
sends reference images as chat message `image_url` data URLs.

## Grok Imagine Models

The Grok Imagine tab enables xAI text-to-image generation through
`/v1/images/generations`:

- `grok-imagine-image`

The current implementation requests `b64_json` responses so generated images
can be saved locally with the same SozoCraft PNG metadata path as other
providers. The Grok Imagine tab supports xAI aspect ratios, `1k`/`2k`
resolution, the documented `quality` field, and up to 5 uploaded reference
images through xAI's JSON image edit endpoint. Mask editing is intentionally not
implemented yet.

## Output Metadata

Generated images are always saved as PNG files. Non-PNG provider responses are
converted before writing so SozoCraft can embed stable PNG `tEXt` metadata:

- `prompt`: final rendered prompt sent to the image model
- `sozocraft`: JSON metadata with `schemaVersion`, `promptSnapshot`,
  `renderedPrompt`, provider/model/options, batch/image ids, timestamps, and
  provider response metadata

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
- `{id}`
- `{batch_id}`
- `{extension}`

Provider and model values are sanitized for filesystem safety. For Gemini generation, filename aliases use `gemini` as provider and names such as `nano-banana-2` as model. `{id}` is the batch-local image order, for example `001`. Existing files are not overwritten; SozoCraft appends a numeric suffix when needed.

## Validation

```bash
pnpm typecheck
pnpm build
cd src-tauri && cargo test
```

## Roadmap Notes

- PromptCraft DSL parsing/rendering and validation
- syntax-highlighted prompt editor
- reference image upload UI
- Prompt Bridge local HTTP server for external tools
- ComfyUI local backend
- provider-specific mask editing flows for GPT-Image and Grok Imagine
- future video generation backend
