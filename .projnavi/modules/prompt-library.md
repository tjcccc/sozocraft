# Prompt Library Module

## Purpose

The prompt library stores prompt bodies as markdown files and prompt metadata in a SQLite index. The renderer presents library navigation, editing, metadata, preview placement, and include drag/drop; Rust owns durable persistence and prompt rendering.

## Read First

- `src/hooks/usePromptLibrary.ts` for frontend library bootstrapping, autosave, metadata commits, query state, rendered-preview updates, import, delete, and tag rename calls.
- `src/components/PromptColumn.tsx` for library tree UI, tag folders, prompt metadata fields, rendered preview placement, export controls, and include drag/drop affordances.
- `src/components/PromptEditor.tsx` for text editing, search/replace, syntax highlighting, and prompt include drop behavior.
- `src-tauri/src/prompt_library.rs` for markdown file indexing, SQLite schema, title/tag metadata, legacy frontmatter import, render grammar, include resolution, tag renames, and path validation.
- `README.md` prompt-library section for the user-facing DSL and include syntax.

## Editing Notes

- Prompt body files are UUID-named markdown files; title, tags, timestamps, and search data live in `~/.sozocraft/prompts.sqlite`.
- Rendered generation sends the rendered prompt to providers and stores the markdown source as `promptSnapshot`.
- Include resolution supports title matches, exact tag paths, scoped tag/title references, quoted segments, and a deprecated slash-scoped fallback.
- Add Rust tests for renderer/parser/include changes when practical; add frontend tests for editor interaction regressions.
