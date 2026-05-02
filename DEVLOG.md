# DEVLOG

## 2026-05-02

- Bumped app/package/crate version metadata to `0.12.1` for the UTF-8 PNG metadata checkpoint.
- Changed generated PNG metadata embedding from Latin-1 `tEXt` chunks to UTF-8 `iTXt` chunks so Chinese and other non-Latin prompt text round-trips correctly.

## 2026-05-01

- Bumped app/package/crate version metadata to `0.12.0` for the Fedora setup, prompt metadata portability, and reference-image drag/drop checkpoint.
- Bumped app/package/crate version metadata to `0.11.0` for the prompt library and DSL editor checkpoint.
- Fixed prompt-library rescans so synced SQLite metadata survives when the prompt directory root changes across machines, such as macOS `~/.sozocraft/prompts` to Fedora `~/.sozocraft/prompts`.
- Added drag-and-drop image import on the reference-image add tile, using Tauri's native file-drop event so it works in Linux desktop webviews while reusing the existing picker validation and per-model image limits.
- Added the first real prompt-library/editor milestone: markdown prompt files live under the configurable prompt directory, defaulting to `~/.sozocraft/prompts`, with a rebuildable SQLite metadata index in app data.
- Added YAML-style prompt file frontmatter for `name`, `tags`, `description`, `createdAt`, `updatedAt`, and `schemaVersion`; prompt files intentionally do not use a user-facing version counter.
- Added ComfyUI Banana Studio-compatible prompt rendering for simple `name = value` variables and the first `prompt = { ... }` / `prompt { ... }` block, with plain markdown-body fallback.
- Replaced the placeholder prompt sidebar with create/search/select/refresh/delete actions and debounced autosave state.
- Changed generation to send the rendered prompt to providers while preserving the full markdown source in `promptSnapshot` metadata.
- Revised prompt storage so new prompt files are UUID-named markdown bodies, while title/tags/timestamps live in `~/.sozocraft/prompts.sqlite`; legacy YAML frontmatter files are stripped in the editor and indexed for migration.
- Added title and tags inputs, nested slash-tag library grouping, updated/name sort controls, a resizable prompt-library pane, last-open prompt restoration, and auto-creation of an `Untitled Prompt` file when the library is empty.
- Fixed empty `prompt = {}` blocks so they render to an empty prompt instead of falling back to the original block text.
- Added a persisted Prompt Editor DSL toggle under `[prompts].dsl_enabled`; when off, the rendered prompt is the exact editor text.
- Simplified the prompt sidebar controls by removing refresh, compacting sort controls to icon buttons, and moving delete to a hover action on prompt rows.
- Changed prompt title saving so title edits commit on blur instead of during typing.
- Removed tags from prompt list item rows and added DSL prompt includes by title with `{#prompt title}` syntax.

## 2026-04-30

- Bumped app/package/crate version metadata to `0.10.0` for the reference-image compression and prompt race fix checkpoint.
- Fixed a prompt race where pressing Run immediately after editing could send the previous prompt; the Run path now reads from a live prompt ref updated directly by the textarea change handler.

## 2026-04-29

- Added backend reference-image optimization before provider dispatch: large non-alpha PNG/JPEG uploads are cached as quality-85 JPEGs with a 1600 px max long edge under `~/.sozocraft/cache/reference-images`, with opportunistic cleanup for entries older than about 30 days. WebP and transparent inputs pass through unchanged.
- Bumped app/package/crate version metadata to `0.9.0` for the completed GPT-Image OpenRouter and reference-image checkpoint.
- Enabled GPT-Image reference images with a 16-image limit. OpenAI-compatible endpoints use multipart `/images/edits` `image[]` inputs; OpenRouter endpoints use chat message image data URLs.
- Fixed GPT-Image response parsing for OpenRouter/OpenAI shape mismatches: OpenAI Image API `data[].b64_json`, OpenAI Responses `image_generation_call.result`, and OpenRouter `choices[].message.images[]` data URLs are now accepted. OpenRouter model API page URLs are routed through `/api/v1/chat/completions` with image modalities.
- Bumped app/package/crate version metadata to `0.8.0` for the multi-provider image generation checkpoint. GPT-Image generation is wired but remains unfinished until live endpoint validation passes.
- Added active GPT-Image and Grok Imagine image provider tabs alongside Nano Banana, with text-to-image generation routed through provider-specific Rust clients.
- Added OpenAI and xAI API key/base URL settings in the full-page Settings view; settings now replaces the workspace instead of expanding inline.
- Changed `Save Settings` to also persist any non-empty provider API key fields so users do not have to click each per-key save button first.
- Fixed OpenAI/xAI API key saves by avoiding a frontend name collision where React state setters shadowed the Tauri persistence functions.
- Aligned Grok Imagine options with xAI image docs: full aspect-ratio list, `1k`/`2k` resolution, no-op `quality` selector, API `n` usage for batches, and up to 5 reference images through `/v1/images/edits` without masks.
- Changed the status bar provider label for Grok Imagine from `Grok Imagine` to `xAI` while keeping the tab/model label as `Grok Imagine`.
- Updated the output preview gallery so single images scale down to fit the available preview height instead of creating an internal vertical scrollbar.
- Removed the Seed generation option and stopped sending Gemini `seed`, since current Nano Banana image generation does not expose a supported seed control.
- Narrowed GPT-Image to `gpt-image-2` only, with documented `size` and `quality` controls; live generation remains marked unfinished until tested against the target OpenAI-compatible endpoint.
- Initially kept GPT-Image reference-image editing out of scope before enabling it in the follow-up fix.
- Added an in-app image lightbox for reference images, output gallery images, and output history thumbnails with Esc/backdrop close and subtle arrow-key/click navigation.
- Added append-only generation failure logging to `~/.sozocraft/error.log` with JSON-lines entries for client setup, request, and no-image failures.
- Added Gemini REST `safetySettings` with `BLOCK_NONE` for the adjustable sexually explicit category.
- Expanded the `Gemini returned no image data.` failure path to include prompt feedback, candidate finish reasons/messages, safety ratings, and text response snippets when Gemini returns them without inline image parts.

## 2026-04-28 (session 4)

- Bumped app/package/crate version metadata to `0.6.0` for the reference-image and architecture checkpoint.
- Implemented reference image upload in the generation panel with a compact add tile, thumbnail tiles, per-model max counts, and inline Gemini request payloads.
- Split frontend UI out of `App.tsx` into panel components, shared UI primitives, focused hooks, utilities, and a Gemini image model catalog.
- Split backend Gemini model capability constants into `gemini_models.rs`, leaving `models.rs` focused on serializable app/request/history types.
- Added `spec/ui.md` and `docs/architecture.md` to document current UI conventions and module boundaries.
- Updated Gemini image option controls from the current official Gemini image-generation docs: model changes now clamp aspect ratio, image size, and thinking options to the selected model's supported profile.
- Enforced PNG as the saved output format for generated images so metadata embedding is always available.
- Updated PNG metadata schema: `prompt` stores the rendered model prompt, and the `sozocraft` JSON chunk now includes `schemaVersion`, `promptSnapshot`, and `renderedPrompt`.
- Added `promptSnapshot` to generation requests for future PromptCraft DSL source retention; current UI sends the plain prompt as both source and rendered prompt.

## 2026-04-28 (session 3)

- Bumped app/package/crate version metadata to `0.4.0` for the SozoCraft rename checkpoint.
- Completed the repository/package/product rename to SozoCraft / `sozocraft`.
- Updated Tauri product name, window title, bundle identifier, Rust crate/lib names, npm package name, visible UI copy, docs, and config examples.
- App UI name/logo uses the stylized `SōzōCraft` spelling while repo/package identifiers remain ASCII.
- New local config/state paths use `~/.sozocraft/config.toml` and `SozoCraft/state.json`.
- New PNG metadata tEXt chunks use `sozocraft` as the app metadata key.

## 2026-04-28 (session 2)

- Preview area is blank on startup; only shows a batch after the user explicitly generates or clicks a history row. Replaced `expandedBatch ?? filteredBatches[0]` fallback with a dedicated `previewBatchId` state (null on startup). Generation sets it to the new batch; history-row click sets it to that batch.
- Resize handle for the history drawer is now `position: absolute` and transparent — no longer renders as a visible extra bar above the History toggle.
- Removed card border/border-radius from preview gallery image tiles; image fills full tile width. Gallery `align-content` changed to `flex-start`.
- Single-image batch uses `gallery-single` class: tile takes 100% gallery width instead of 50%.
- Batch info bar moved from top heading to bottom-center of the preview area with 16 px bottom padding (`batch-info-bar`).

## 2026-04-28

- Removed `mimeType: "image/png"` from `imageConfig` request — API returned 400. Client-side JPEG→PNG conversion (`image_meta::to_png`) is the only path now.
- Removed sidecar `.json` file output; metadata is embedded directly in the PNG tEXt chunks only.
- History panel converted to a collapsible bottom drawer: collapsed = 32px bar showing "History" label; click to open at 280px default; height is resizable by dragging the top handle.
- Preview area (upper section) now follows the selected history batch: clicking a row updates the gallery above. Falls back to the latest batch for the current date when nothing is selected. `expandedBatchId` is cleared on date change and after each new generation.
- Error messages from failed generation are shown inline in the output preview area (red banner) in addition to the status bar.
- Added `image` crate (JPEG/PNG codec only) for client-side format conversion.
- Output images are now always PNG: non-PNG responses (JPEG/WebP) are converted via `image_meta::to_png` before saving.
- Changed PNG tEXt embedding from a single `sozocraft` chunk to two chunks: `prompt` (raw prompt text as sent to the API) and `sozocraft` (generation metadata without `promptSnapshot`, `filename`, `outputTemplate`, `path`).

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

- Added `save_output_template` Tauri command that write-modify-writes only the `output.template` key in `~/.sozocraft/config.toml`; hooked to `onBlur` on the filename template input in the Output panel.
- Restructured toolbar to a 3-column grid (`toolbar-left` / `toolbar-center` / `toolbar-right`): brand stays left, Run/Stop are centered, Image/Video mode switch + Settings button are right-aligned.
- Removed batch stepper and StatusPill from the toolbar; StatusPill and Provider/Model info now live exclusively in the status bar.
- Added Image/Video mode switch (Video disabled, placeholder for v0.2) to the right side of the toolbar.
- Replaced flat `MODELS` / `ASPECT_RATIOS` / `IMAGE_SIZES` constants with per-model `MODEL_CONFIGS` record that controls available aspect ratios, image sizes (null = hidden), and whether the Thinking field is shown.
- Changed generation option defaults: aspect ratio → 4:3, image size → 2K, temperature → −1 (model default / not sent to API), thinking → HIGH.
- Fixed temperature guard in `gemini.rs`: values < 0 are no longer forwarded to the Gemini API.
- Auto-generate random seed when seed input is empty at generation time; populated value shown in the seed field after run.
- Added `src-tauri/src/image_meta.rs`: PNG tEXt chunk embedding with inline CRC32 (IEEE 802.3, no external deps); sidecar JSON is now also embedded in PNG output files as a `sozocraft` tEXt chunk for future drag-drop metadata loading (ComfyUI-style).
- Added `field-hint` style shown below Temperature input when value is −1.

## 2026-04-27

- Made `{extension}` optional in output filename templates; the MIME-detected extension is appended automatically when the token is absent.
- Removed the corresponding validation gate that previously rejected templates without `{extension}`.
- Added per-image sidecar JSON files (`<image>.png.json`) saved alongside each output image, containing prompt source/snapshot, provider, model, generation options, filename template, batch/image timestamps, and Gemini response metadata.
- Added `get_config_status` Tauri command returning the active `~/.sozocraft/config.toml` path, Gemini API key presence, and proxy configuration status.
- Added config status panel inside SettingsPanel showing the three config indicators as coloured badges.
- Added live template validation feedback in the Output Images panel: flags unsupported/uppercase date tokens and shows a rendered example of the current template.
- Made failed batch history entries expandable to show the full provider error text inline.

## 2026-04-26

- Scaffolded SozoCraft `0.1.0` MVP plan into a Tauri 2 + React + TypeScript + Rust desktop app.
- Added Rust backend modules for state persistence, local Gemini config, Gemini generation, filename template resolution, and output image saving.
- Added a three-column desktop UI for prompt editing, Nano Banana generation controls, and output gallery/history.
- Kept PromptCraft DSL, Prompt Bridge, ComfyUI, GPT-Image, Grok Imagine, reference image upload, and video generation as roadmap items.
- Switched local configuration to `~/.sozocraft/config.toml`, including Gemini API key, model, output path, proxy, and timeout.
- Added resizable workspace columns and a resizable prompt preview pane.
- Updated filename handling to use Unicode-style date tokens such as `yyyyMMdd_HHmmss`, local-time output names, batch-local image ids like `001`, and Gemini-friendly filename aliases.
- Added `TODO.md` with the next stabilization and PromptCraft integration plan.
