# Prompt Library Flow

## Flow

1. `load_app_state` rescans the configured prompt directory as a best-effort startup refresh.
2. `usePromptLibrary` calls `rescanPromptLibrary`, opens the last current prompt when possible, otherwise opens the newest prompt or creates one.
3. Source edits autosave through `savePrompt`; metadata edits commit through `updatePromptMetadata`; tag folder renames call `renamePromptTag`.
4. Rendering preview uses `renderPromptSource` with the prompt directory and current prompt ID so Rust can resolve includes while avoiding self-recursion.
5. Generation uses the rendered prompt for provider calls and keeps the markdown source as the batch/image prompt snapshot.

## Common Change Points

- Prompt list/search/tag UI: `src/hooks/usePromptLibrary.ts` and `src/components/PromptColumn.tsx`.
- Editor interactions and keyboard behavior: `src/components/PromptEditor.tsx` and `src/components/PromptEditor.test.tsx`.
- Rendering grammar, include resolution, legacy frontmatter, SQLite schema, and path rules: `src-tauri/src/prompt_library.rs`.
