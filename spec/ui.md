# SozoCraft UI Spec

## Stack

- React 19 + TypeScript inside a Vite/Tauri desktop shell.
- Styling is plain CSS in `src/styles.css`; no component library or utility CSS framework is used.
- Icons come from `lucide-react`.

## Design Direction

SozoCraft is a desktop creative tool, not a marketing site. The UI should stay compact, work-focused, and optimized for repeated prompt editing, generation, and image review.

The current design uses:

- three-column workspace: prompt editor, generation controls, output/history
- top toolbar for app identity, run controls, mode switch, and settings
- bottom status bar for run state and aggregate counts
- light neutral surface with subtle blue accents
- restrained panels with persistent borders and small-radius controls

## Layout Conventions

- Preserve the top toolbar, central resizable workspace, and bottom status bar.
- Keep the three main work areas as first-class panels rather than nested card stacks.
- Keep drag handles stable in width/height so resizing does not shift adjacent content.
- Output preview should remain empty on startup until the user generates or selects a history row.
- History remains a collapsible bottom drawer inside the output panel.

## Component Conventions

- Top-level orchestration belongs in `src/App.tsx`.
- Major UI areas live in `src/components/*`:
  - `PromptColumn.tsx`
  - `GenerationPanel.tsx`
  - `OutputColumn.tsx`
  - `SettingsPanel.tsx`
- Shared small UI primitives live in `src/components/common.tsx`.
- Frontend model capabilities live in `src/models/geminiImageModels.ts`.
- Reusable state/effect logic lives in `src/hooks/*`.

## Controls

- Use native form controls for settings and generation options unless a richer control is needed.
- Use icon buttons for compact toolbar/tool actions when the icon is familiar.
- Keep text labels on primary run/save actions.
- Disabled future providers or modes may stay visible when they clarify the roadmap, but should not distract from the active Gemini flow.

## Typography And Color

- Use the existing system sans-serif stack.
- Keep compact panel headings around the current scale; do not introduce hero-sized type inside tool surfaces.
- Maintain the current neutral background and blue action/accent palette unless a broader redesign is requested.

## Open Decisions

- No mobile layout is currently defined; the desktop shell has a minimum 1100px width.
- No formal design tokens exist yet beyond the CSS values in `src/styles.css`.
- PromptCraft DSL-specific UI states are placeholders until the DSL renderer exists.
