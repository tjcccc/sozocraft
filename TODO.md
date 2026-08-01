# SozoCraft Next TODOs

## v0.1.x Stabilization

- Save per-image sidecar metadata JSON next to each output image, including prompt source, final prompt snapshot, provider, model, generation options, filename template, timestamps, and response metadata.

## Prompt Workflow

- Integrate `@promptcraft/core` behind the existing DSL toggle.
- Show PromptCraft parse/render validation errors inline in the prompt editor.

## Output Workflow

- Add image context actions: reveal in Finder, copy path, copy prompt, and open metadata.
- Add output gallery filtering by provider/model/status/date.
- Add safe cleanup controls for failed batches and missing local files.
- Replace base64 output previews with safe file-backed preview URLs, likely by copying generated preview assets into a scoped temp/cache directory and serving only that directory through a narrow Tauri asset/protocol path.

## Architecture Roadmap

- Continue architecture work opportunistically before or during major feature work, not as a broad standalone rewrite.
- Split `prompt_library.rs` by responsibility: storage/indexing, source parsing/rendering, include resolution, metadata/frontmatter migration, and filesystem safety.
- Split `higgsfield.rs` into focused units for CLI status/auth, model option mapping, job/result parsing, downloading, and temporary reference-image handling.
- Keep provider integrations behind narrow adapter contracts with focused tests for request mapping, option validation, result parsing, proxy behavior, and failure diagnostics.
- Add regression tests when extracting a boundary first; refactor only the code needed for the feature or risk being handled.

## Video Roadmap

- Complete Higgsfield CLI Seedance video qualification across Standard, Fast,
  and Mini, including reference inputs, result polling/download, and recovery
  from interrupted or previously misclassified jobs.
- Add restart-safe recovery for in-flight provider video request ids.
- Add progress reporting from provider polling responses.
- Add video editing and video extension as separate milestones.
- Add provider-side cancellation where a video API exposes a supported endpoint.
