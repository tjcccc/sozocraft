# SozoCraft

SozoCraft is a macOS-first, cross-platform desktop AI visual generation studio and prompt management workspace.

The name comes from Japanese `sōzō` / `souzou`, evoking both imagination
(`想像`) and creation (`創造`).

This repository currently targets the `0.1.0` MVP:

- Tauri 2 desktop shell
- React + TypeScript frontend
- Rust backend
- Gemini image generation for Nano Banana / Nano Banana Pro
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

Open the settings button in the top toolbar and configure:

- Gemini API key
- default Gemini image model
- output directory
- optional Gemini-compatible base URL
- optional proxy URL, for example `http://127.0.0.1:7890`
- timeout

SozoCraft stores local configuration in:

```text
~/.sozocraft/config.toml
```

Example:

```toml
[gemini]
api_key = "your_gemini_api_key_here"
default_model = "gemini-3-pro-image-preview"
base_url = "https://generativelanguage.googleapis.com/v1beta/models"
proxy_url = "http://127.0.0.1:7890"
timeout_seconds = 180

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
- GPT-Image provider
- Grok Imagine provider
- future video generation backend
