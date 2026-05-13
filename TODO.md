# SozoCraft Next TODOs

## v0.1.x Stabilization

- Make `.{extension}` optional in output templates. If omitted, append the extension detected from the generated image MIME type.
- Save per-image sidecar metadata JSON next to each output image, including prompt source, final prompt snapshot, provider, model, generation options, filename template, timestamps, and response metadata.
- Add template validation feedback in the UI before generation, including unsupported date tokens, missing output directory, and examples for `yyyyMMdd_HHmmss`.
- Add a small config status panel showing the active `~/.sozocraft/config.toml` path, whether proxy is configured, and whether the Gemini API key is present.
- Improve failed batch history entries so the user can expand and inspect the exact provider error.

## Prompt Workflow

- Integrate `@promptcraft/core` behind the existing DSL toggle.
- Save both prompt source and rendered prompt snapshot for every generation.
- Show PromptCraft parse/render validation errors inline in the prompt editor.
- Add prompt library persistence for saved plain prompts and future DSL prompts.

## Output Workflow

- Add image context actions: reveal in Finder, copy path, copy prompt, and open metadata.
- Add output gallery filtering by provider/model/status/date.
- Add safe cleanup controls for failed batches and missing local files.
- Replace base64 output previews with safe file-backed preview URLs, likely by copying generated preview assets into a scoped temp/cache directory and serving only that directory through a narrow Tauri asset/protocol path.

## Architecture Roadmap

- Continue architecture work opportunistically before or during major feature work, not as a broad standalone rewrite.
- Before video generation, split shared generation orchestration out of `App.tsx` so image and video flows can share queue/history/settings behavior without adding more top-level state.
- Split `prompt_library.rs` by responsibility: storage/indexing, source parsing/rendering, include resolution, metadata/frontmatter migration, and filesystem safety.
- Split `higgsfield.rs` into focused units for CLI status/auth, model option mapping, job/result parsing, downloading, and temporary reference-image handling.
- Keep provider integrations behind narrow adapter contracts with focused tests for request mapping, option validation, result parsing, proxy behavior, and failure diagnostics.
- Add regression tests when extracting a boundary first; refactor only the code needed for the feature or risk being handled.

## Provider Roadmap

- Keep Nano Banana / Gemini as the only implemented provider until the MVP flow is stable.
- Add reference image upload for Gemini after prompt/history/output handling is reliable.
- Add GPT-Image, Grok Imagine, ComfyUI Local, and Prompt Bridge as separate provider/backend milestones.
