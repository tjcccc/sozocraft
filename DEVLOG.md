# DEVLOG

## 2026-04-26

- Scaffolded VisionCraft `0.1.0` MVP plan into a Tauri 2 + React + TypeScript + Rust desktop app.
- Added Rust backend modules for state persistence, local Gemini config, Gemini generation, filename template resolution, and output image saving.
- Added a three-column desktop UI for prompt editing, Nano Banana generation controls, and output gallery/history.
- Kept PromptCraft DSL, Prompt Bridge, ComfyUI, GPT-Image, Grok Imagine, reference image upload, and video generation as roadmap items.
- Switched local configuration to `~/.visioncraft/config.toml`, including Gemini API key, model, output path, proxy, and timeout.
- Added resizable workspace columns and a resizable prompt preview pane.
- Updated filename handling to use Unicode-style date tokens such as `yyyyMMdd_HHmmss`, local-time output names, batch-local image ids like `001`, and Gemini-friendly filename aliases.
- Added `TODO.md` with the next stabilization and PromptCraft integration plan.
