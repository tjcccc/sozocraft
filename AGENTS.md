# AGENTS

SozoCraft is a Tauri 2 + React/TypeScript desktop app with a Rust backend.

## Quality Bar

- Build production-grade product code, not throwaway prototype code.
- Keep React UI, hooks, provider capability tables, and Rust native/provider logic separated by clear ownership.
- Split files before they become mixed-responsibility catchalls. In particular, avoid adding more unrelated responsibilities to `App.tsx`, `PromptColumn.tsx`, `SettingsPanel.tsx`, `src-tauri/src/lib.rs`, `higgsfield.rs`, or `prompt_library.rs`.
- Prefer small targeted refactors during related feature work over late broad rewrites.

## Tauri Boundaries

- Treat the renderer as less trusted than Rust. Tauri commands must validate paths, file kinds, sizes, process paths, URLs, and provider options.
- Do not expose broad filesystem, shell, or network behavior without a narrow command contract and explicit validation.
- Keep CSP and capabilities restrictive; document any relaxation.
- Store secrets only in local config, never in app state, docs, generated metadata, or logs.

## Validation

- Frontend/UI changes: run `pnpm test:frontend` and `pnpm build`.
- Rust/native/provider changes: run `cargo test` in `src-tauri`.
- Always run `git diff --check` before committing.
- Add focused tests for parsing, persistence, security-boundary validation, provider behavior, and bug fixes when practical.

## Docs

- Update `README.md`, `DEVLOG.md`, `spec/ui.md`, or this file when behavior, setup, architecture, or security expectations change.
