# Frontend Module

## Purpose

The renderer owns UI composition, local view state, provider controls, prompt editing affordances, and Tauri command calls. It should not be treated as the final security boundary.

## Read First

- `src/App.tsx` for top-level layout, toolbar actions, settings switching, drag/drop routing, lightbox handling, and cross-hook composition.
- `src/api.ts` for the exact Tauri command names and TypeScript return types used by the renderer.
- `src/types.ts` for shared frontend request/state shapes.
- `src/models/imageProviders.ts` and `src/models/geminiImageModels.ts` for provider/model catalogs, per-platform default models, control options, and option normalization.
- `src/hooks/useAppState.ts` for loading/saving settings, API-key state, status/message state, and app batches.
- `src/hooks/useGeneration.ts` for queued generation tasks, request construction, reference-image state, cancellation, and batch insertion.
- `src/hooks/usePromptLibrary.ts` for prompt list bootstrapping, autosave, metadata save, render preview, imports, and deletes.

## Editing Notes

- Adding or renaming provider/model controls usually touches `src/models/imageProviders.ts`, `src/components/GenerationPanel.tsx`, `src/components/SettingsPanel.tsx`, `src/types.ts`, and backend validation/provider dispatch.
- To add a new image provider, start with the frontend provider catalog, then follow the settings panel and generation hook wiring before crossing into Rust.
- Prompt editor behavior has a focused Vitest file at `src/components/PromptEditor.test.tsx`; add similarly focused tests for editor parsing/search regressions.
- Keep heavy orchestration in hooks and typed helpers rather than expanding `App.tsx` with unrelated responsibilities.
