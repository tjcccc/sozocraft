# SozoCraft Next TODOs

## v0.1.x Stabilization

- Output metadata: desktop images are always saved as PNG with metadata embedded
  in PNG text chunks; MP4s keep a separate `.mp4.json` sidecar (no in-file MP4
  metadata). Raw Higgsfield archive copies stay unmodified. Remaining: add the
  prompt source and filename template fields to both.

## Prompt Workflow

- Integrate `@promptcraft/core` behind the existing DSL toggle.
- Show PromptCraft parse/render validation errors inline in the prompt editor.

## Output Workflow

- Add image context actions: reveal in Finder, copy path, copy prompt, and open metadata.
- Add safe cleanup controls for failed batches and missing local files.

## Architecture Roadmap

- Continue architecture work opportunistically before or during major feature work, not as a broad standalone rewrite.
- Split `prompt_library.rs` by responsibility: storage/indexing, source parsing/rendering, include resolution, metadata/frontmatter migration, and filesystem safety.
- Split `higgsfield.rs` into focused units for CLI status/auth, model option mapping, job/result parsing, downloading, and temporary reference-image handling.
- Keep provider integrations behind narrow adapter contracts with focused tests for request mapping, option validation, result parsing, proxy behavior, and failure diagnostics.
- Add regression tests when extracting a boundary first; refactor only the code needed for the feature or risk being handled.

## Video Roadmap

- Complete live Higgsfield CLI Seedance video qualification across Standard,
  Fast, and Mini, including reference inputs, result polling/download, and the
  restart/Resume recovery path (implemented and unit-tested, not yet exercised
  against a live interrupted job).
- Extend restart-safe video resume beyond Higgsfield to Ark, xAI, Veo, and Omni
  request ids. Those jobs are currently marked failed on restart with their
  provider request id recorded.
- Add progress reporting from provider polling responses.
- Add video editing and video extension as separate milestones.
- Add provider-side cancellation where a video API exposes a supported endpoint.
