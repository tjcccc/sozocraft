# SozoCraft Architecture

## Frontend

- `src/App.tsx` wires app-level layout, toolbar actions, panel composition, and status footer.
- `src/components/` contains UI panels and shared primitives.
- `src/hooks/` contains reusable state/effect workflows:
  - `useAppState` loads and saves app settings/state.
- `useGeneration` owns generation option state and run orchestration.
  - `useHistoryDate` filters batches by selected history date.
  - `useImagePreviews` loads saved image files into data URLs.
  - `useModelOptions` clamps generation options to the selected model.
- `src/models/geminiImageModels.ts` is the frontend Gemini image model capability catalog.
- `src/utils/` contains pure formatting, validation, and numeric helpers.

## Backend

- `src-tauri/src/models.rs` contains serializable app/request/history types.
- `src-tauri/src/gemini_models.rs` contains Gemini image model capability constants.
- `src-tauri/src/gemini.rs` builds Gemini requests, parses responses, and guards model-specific options.
- `src-tauri/src/image_meta.rs` handles PNG conversion and metadata embedding.
- `src-tauri/src/local_config.rs` manages `~/.sozocraft/config.toml`.
- `src-tauri/src/app_state.rs` manages local app state under the platform data directory.

## Data Contract

- Generated outputs are saved as PNG files.
- PNG metadata uses a plain `prompt` text chunk for the rendered model prompt.
- PNG metadata also stores a `sozocraft` JSON chunk with `schemaVersion`, `promptSnapshot`, `renderedPrompt`, provider/model/options, ids, timestamps, and response metadata.
- Until PromptCraft DSL rendering exists, `promptSnapshot` and `renderedPrompt` carry the same prompt text.

## Boundaries

- Frontend option clamping improves UX, but backend request building is the source of safety for API payloads.
- Reference images are selected in the frontend, stored as browser data URLs/base64 payloads for the current run, and sent to the backend as inline image data.
- Keep frontend and backend Gemini model capability catalogs synchronized when the official API changes.
- Avoid adding global state libraries, generated schemas, or routing until the app has a concrete need.
