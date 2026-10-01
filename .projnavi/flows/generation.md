# Generation Flow

## Flow

1. `src/App.tsx` asks `usePromptLibrary`/`renderPromptSource` for the rendered prompt and keeps the markdown source as the prompt snapshot.
2. `src/hooks/useGeneration.ts` builds a `GenerationRequest`, saves the prompt snapshot, queues the task, persists the task settings, calls `generateImages`, and updates local batches from the returned `GenerationBatch`.
3. `src/api.ts` maps `generateImages` and cancellation to Tauri command names.
4. `src-tauri/src/lib.rs` receives `generate_images`, installs a cancellation watch channel, calls `GenerationRequest::validate`, optimizes reference images, dispatches the provider call, writes PNG files, embeds `prompt` and `sozocraft` metadata, appends the batch to local state, and logs sanitized provider failures.
5. Provider-specific request/response details live in `gemini.rs`, `openai_image.rs`, `xai_image.rs`, and `higgsfield.rs`.

Video tasks share the frontend queue but use `useVideoGeneration` and the
`generate_video` Tauri command. `useVideoGeneration` derives text, starting
frame, start/end frame, or reference mode from per-thumbnail roles in each
provider's image list. New uploads default to Reference except Veo Lite, which assigns Start/End
frames and does not support asset references; the hover menu can
assign Start for every provider or End for Seedance and Veo. `videoProviders.ts`
supplies the Seedance, Grok Imagine, and Google Veo capability rules.
`video_generation.rs` validates and optimizes that contract, dispatches to
`seedance_video.rs`, `xai_video.rs`, or `google_veo.rs`, then saves the MP4 plus
JSON sidecar. Grok reference mode caps duration at 10 seconds, while Veo
reference mode is fixed at 8 seconds.

## Common Change Points

Headless agents use `sozocraft-cli video generate/status/wait`, bypassing the
renderer and desktop history. `cli/arguments.rs` parses options and resume IDs;
`cli/video.rs` validates `VideoGenerationRequest`, loads the shared Gemini config,
dispatches to `google_veo.rs` or `google_omni.rs`, emits JSON events, and publishes
a local MP4 without overwriting. Start with `docs/agent-cli.md` for usage.

- New generation option: frontend state/defaults in `useGeneration.ts`, control visibility in `GenerationPanel.tsx`, request types in `src/types.ts`, Rust request types/defaults/validation in `models.rs`, metadata write in `lib.rs`, and provider payload builders.
- New provider/model or task to add a new image provider: frontend provider catalog and settings platform controls, backend supported-model arrays and dispatch, local config/defaults, docs, and tests.
- Output-path behavior: `src/utils/outputTemplate.ts` for UI validation and `src-tauri/src/filename_template.rs` for authoritative path resolution.
