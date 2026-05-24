# Generation Flow

## Flow

1. `src/App.tsx` asks `usePromptLibrary`/`renderPromptSource` for the rendered prompt and keeps the markdown source as the prompt snapshot.
2. `src/hooks/useGeneration.ts` builds a `GenerationRequest`, saves the prompt snapshot, queues the task, persists the task settings, calls `generateImages`, and updates local batches from the returned `GenerationBatch`.
3. `src/api.ts` maps `generateImages` and cancellation to Tauri command names.
4. `src-tauri/src/lib.rs` receives `generate_images`, installs a cancellation watch channel, calls `GenerationRequest::validate`, optimizes reference images, dispatches the provider call, writes PNG files, embeds `prompt` and `sozocraft` metadata, appends the batch to local state, and logs sanitized provider failures.
5. Provider-specific request/response details live in `gemini.rs`, `openai_image.rs`, `xai_image.rs`, and `higgsfield.rs`.

## Common Change Points

- New generation option: frontend state/defaults in `useGeneration.ts`, control visibility in `GenerationPanel.tsx`, request types in `src/types.ts`, Rust request types/defaults/validation in `models.rs`, metadata write in `lib.rs`, and provider payload builders.
- New provider/model or task to add a new image provider: frontend provider catalog and settings platform controls, backend supported-model arrays and dispatch, local config/defaults, docs, and tests.
- Output-path behavior: `src/utils/outputTemplate.ts` for UI validation and `src-tauri/src/filename_template.rs` for authoritative path resolution.
