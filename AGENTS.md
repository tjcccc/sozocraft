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

<!-- projnavi-agent-codex:start -->
## projnavi

projnavi is a local navigation layer for coding agents. Humans initialize it; agents use it before broad work.

When the user says exactly or approximately:

```text
projnavi onboard
```

treat it as this task:

```text
Run projnavi onboarding for this repo. Execute `projnavi onboard`, inspect the repo, improve the `.projnavi` project notes, module notes, flow notes, glossary, and claims for future guide queries, then run `projnavi onboard` again and `projnavi verify`. Update `AGENTS.md` only if useful. Do not make unrelated code changes.
```

Before broad or ambiguous codebase work, run:

```bash
projnavi guide "<task>"
```

Use guide output as navigation advice only. Verify source files before editing. Skip projnavi for trivial single-file edits where the user already named the exact file and location.

After changing files referenced by `.projnavi/claims.jsonl`, `.projnavi/glossary.json`, or `.projnavi` notes, run:

```bash
projnavi onboard
projnavi verify
```

When the user says exactly or approximately:

```text
projnavi benchmark
```

treat it as this read-only benchmark request:

```text
Based on the current project, choose a realistic complex codebase task. Do not edit files. Dry-run investigation twice: first without projnavi using normal repo exploration, search, and file reads; then with projnavi by running `projnavi guide "<task>"` and inspecting only the recommended first-pass files. Measure wall time, command count, output bytes, output lines, approximate tokens, and qualitative relevance. Report a professional Markdown table, a compact shareable summary, whether projnavi pointed to the right files, and the caveat that approximate tokens are estimated from output bytes rather than model token accounting.
```
<!-- projnavi-agent-codex:end -->
