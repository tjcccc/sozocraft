# Backend Module

## Purpose

The Rust backend is the trust boundary for Tauri commands, provider request construction, generation output saving, local config/state, prompt library persistence, and image metadata.

## Read First

- `src-tauri/src/lib.rs` registers commands, validates generation requests before provider dispatch, handles cancellation, writes generated PNGs, embeds metadata, records history, and routes provider calls.
- `src-tauri/src/models.rs` defines serialized app state/settings, generation request/response types, defaults, supported provider IDs, supported model IDs, and core request validation.
- Provider clients are split by route: `gemini.rs`, `openai_image.rs`, `xai_image.rs`, and `higgsfield.rs`.
- `src-tauri/src/local_config.rs` stores local config and API keys under `~/.sozocraft/config.toml`.
- `src-tauri/src/app_state.rs` stores non-secret app state under the platform data directory.
- `src-tauri/src/file_access.rs`, `filename_template.rs`, `reference_image_cache.rs`, and `image_meta.rs` are boundary helpers for local files, output paths, reference images, and PNG metadata.

## Editing Notes

- Treat `GenerationRequest::validate` in `src-tauri/src/models.rs` as the first stop for new generation request fields or provider/model constraints.
- To add a new image provider, add backend model validation, platform settings/defaults, dispatch selection, provider client logic, output naming, and sanitized error logging together.
- Keep provider-specific payload logic inside the provider module instead of adding large branches to `src-tauri/src/lib.rs`.
- Changes to supported model IDs must stay aligned with frontend model catalogs and settings defaults.
- For Rust/provider changes, run `cargo test` in `src-tauri`; add targeted tests around validation, parsing, persistence, and security-boundary behavior where practical.
