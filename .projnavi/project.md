# Project Notes

## Overview

SozoCraft is a Tauri 2 desktop app with a React/TypeScript renderer and a Rust backend. The product is an AI visual generation studio plus local prompt-management workspace for Nano Banana/Gemini, GPT-Image/OpenAI/OpenRouter, Grok Imagine/xAI, and selected Higgsfield CLI routes.

Use projnavi as navigation advice only. Verify source files before editing, especially where frontend provider catalogs mirror backend model validation.

## Stack And Commands

- Package: `sozocraft`
- Frontend: React 19, TypeScript, Vite, Vitest, lucide-react.
- Desktop/backend: Tauri 2, Rust, Tokio, rusqlite, reqwest-like provider clients.
- Frontend checks: `pnpm test:frontend`, `pnpm build`.
- Rust checks: run `cargo test` from `src-tauri`.
- Always run `git diff --check` before committing.

## First-Pass Map

- App shell and cross-panel wiring live in `src/App.tsx`.
- Tauri invoke wrappers live in `src/api.ts`; matching command handlers are in `src-tauri/src/lib.rs`.
- Provider/model capability UX lives in `src/models/imageProviders.ts` and `src/models/geminiImageModels.ts`.
- Generation orchestration state lives in `src/hooks/useGeneration.ts`; backend request validation and save/history behavior live in `src-tauri/src/models.rs` and `src-tauri/src/lib.rs`.
- Prompt library UI state lives in `src/hooks/usePromptLibrary.ts` and `src/components/PromptColumn.tsx`; prompt persistence, search, include resolution, and rendering live in `src-tauri/src/prompt_library.rs`.
- Settings UI lives in `src/components/SettingsPanel.tsx`; persisted settings defaults and schema live in `src-tauri/src/models.rs` and `src-tauri/src/local_config.rs`.
- Architecture notes in `docs/architecture.md` are useful but may lag active code; treat Rust validation as authoritative for provider payload safety.

## Boundaries To Preserve

- Renderer state is UX support; Rust commands remain the trust boundary for filesystem paths, provider names, model IDs, batch sizes, reference images, and output template handling.
- Avoid growing catchall files called out in `AGENTS.md`: `App.tsx`, `PromptColumn.tsx`, `SettingsPanel.tsx`, `src-tauri/src/lib.rs`, `higgsfield.rs`, and `prompt_library.rs`.
- Secret material belongs in local config under `~/.sozocraft`, never in repo docs, app state snapshots, generated metadata, logs, or projnavi notes.
