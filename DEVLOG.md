# DEVLOG

## 2026-04-27 (session 3)

- Fixed status bar Provider/Model alignment: switched `.statusbar` to `display: grid; grid-template-columns: 1fr auto 1fr` so the center item is geometrically centered.
- Removed `−1 = model default` hint below the Temperature field.
- Removed live template example preview from the Output panel (validation warnings still shown).
- Refactored Output panel layout: preview area is `flex: 0 0 40%` fixed height; history section is `flex: 1` with `overflow-y: auto` inside `.history-section` wrapper — expanding rows no longer shift the gallery.
- History item click now expands inline thumbnails within the row instead of updating the top preview; `failedImagePaths` tracked in App shows "File not found" for missing files.
- Batch title format changed to `2026-04-27 23:44:23 - Gemini / Nano Banana 2 - bc6ae5` (with `(batch)` suffix for multi-image results).
- History list defaults to today's tasks only; date navigation bar (`‹ Today ›`) lets users browse other days.
- History row text reduced to `11.5px`/`32px` rows; list scrolls within its section.
- Lifted `historyDate` to App state so `previewBatch = filteredBatches[0]`; preview and history always show the same date's data.
- Gallery images centered via `flex-wrap: wrap; justify-content: center; align-content: center`.
- Model select field spans full width of the 2-column form grid (`grid-column: 1 / -1`).

## 2026-04-27 (session 2)

- Added `save_output_template` Tauri command that write-modify-writes only the `output.template` key in `~/.visioncraft/config.toml`; hooked to `onBlur` on the filename template input in the Output panel.
- Restructured toolbar to a 3-column grid (`toolbar-left` / `toolbar-center` / `toolbar-right`): brand stays left, Run/Stop are centered, Image/Video mode switch + Settings button are right-aligned.
- Removed batch stepper and StatusPill from the toolbar; StatusPill and Provider/Model info now live exclusively in the status bar.
- Added Image/Video mode switch (Video disabled, placeholder for v0.2) to the right side of the toolbar.
- Replaced flat `MODELS` / `ASPECT_RATIOS` / `IMAGE_SIZES` constants with per-model `MODEL_CONFIGS` record that controls available aspect ratios, image sizes (null = hidden), and whether the Thinking field is shown.
- Changed generation option defaults: aspect ratio → 4:3, image size → 2K, temperature → −1 (model default / not sent to API), thinking → HIGH.
- Fixed temperature guard in `gemini.rs`: values < 0 are no longer forwarded to the Gemini API.
- Auto-generate random seed when seed input is empty at generation time; populated value shown in the seed field after run.
- Added `src-tauri/src/image_meta.rs`: PNG tEXt chunk embedding with inline CRC32 (IEEE 802.3, no external deps); sidecar JSON is now also embedded in PNG output files as a `visioncraft` tEXt chunk for future drag-drop metadata loading (ComfyUI-style).
- Added `field-hint` style shown below Temperature input when value is −1.

## 2026-04-27

- Made `{extension}` optional in output filename templates; the MIME-detected extension is appended automatically when the token is absent.
- Removed the corresponding validation gate that previously rejected templates without `{extension}`.
- Added per-image sidecar JSON files (`<image>.png.json`) saved alongside each output image, containing prompt source/snapshot, provider, model, generation options, filename template, batch/image timestamps, and Gemini response metadata.
- Added `get_config_status` Tauri command returning the active `~/.visioncraft/config.toml` path, Gemini API key presence, and proxy configuration status.
- Added config status panel inside SettingsPanel showing the three config indicators as coloured badges.
- Added live template validation feedback in the Output Images panel: flags unsupported/uppercase date tokens and shows a rendered example of the current template.
- Made failed batch history entries expandable to show the full provider error text inline.

## 2026-04-26

- Scaffolded VisionCraft `0.1.0` MVP plan into a Tauri 2 + React + TypeScript + Rust desktop app.
- Added Rust backend modules for state persistence, local Gemini config, Gemini generation, filename template resolution, and output image saving.
- Added a three-column desktop UI for prompt editing, Nano Banana generation controls, and output gallery/history.
- Kept PromptCraft DSL, Prompt Bridge, ComfyUI, GPT-Image, Grok Imagine, reference image upload, and video generation as roadmap items.
- Switched local configuration to `~/.visioncraft/config.toml`, including Gemini API key, model, output path, proxy, and timeout.
- Added resizable workspace columns and a resizable prompt preview pane.
- Updated filename handling to use Unicode-style date tokens such as `yyyyMMdd_HHmmss`, local-time output names, batch-local image ids like `001`, and Gemini-friendly filename aliases.
- Added `TODO.md` with the next stabilization and PromptCraft integration plan.
