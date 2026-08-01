# SozoCraft UI Spec

## Stack

- React 19 + TypeScript inside a Vite/Tauri desktop shell.
- Styling is plain CSS in `src/styles.css`; no component library or utility CSS framework is used.
- Icons come from `lucide-react`.

## Design Direction

SozoCraft is a desktop creative tool, not a marketing site. The UI should stay compact, work-focused, and optimized for repeated prompt editing, generation, and media review.

The current design uses:

- three-column workspace: prompt editor, generation controls, output/history
- top toolbar for app identity, run controls, mode switch, and settings
- settings opens as a full-page tool view in place of the workspace
- bottom status bar for run state and aggregate counts
- light neutral surface with subtle blue accents
- restrained panels with persistent borders and small-radius controls

## Layout Conventions

- Preserve the top toolbar, central resizable workspace, and bottom status bar.
- Keep the three main work areas as first-class panels rather than nested card stacks.
- Keep drag handles stable in width/height so resizing does not shift adjacent content.
- Output preview should remain empty on startup until the user generates or selects a history row.
- History remains a collapsible bottom drawer inside the output panel.
- Image and Video modes reuse the same prompt editor and three-column geometry;
  only generation controls and output rendering change with the mode.

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
- Provider tabs should be active only when the backend can generate through that
  provider. Keep provider-specific controls hidden when the active provider does
  not support them in the current implementation.
- Reference-image thumbnails wrap onto additional rows within the generation
  panel; do not introduce a horizontal thumbnail scrollbar.
- Video provider tabs appear as Seedance, Grok Imagine, and Google Veo. The
  existing Grok selection remains the initial active provider for compatibility.
  Each provider preserves its own input images and control values while tabs
  switch provider-specific models, duration rules, ratios, resolutions, limits,
  and audio behavior.
- Seedance's Model control offers Seedance 2.0, Seedance 2.0 Fast, and Seedance
  2.0 Mini. Changing models preserves valid settings and falls back to the
  selected model's defaults when a resolution is unavailable.
- Settings exposes a Seedance API Platform selector for Volcengine Ark and
  Higgsfield CLI plus a persisted Default Video Model selector with the same
  Standard, Fast, and Mini choices. The selected default initializes Seedance's
  main-panel model control.
- Video mode uses one wrapping Input Images field whose count limit follows the
  active provider. Its output panel uses native video playback and keeps video
  history separate from image history.
- Video input mode follows per-thumbnail roles. New uploads default to Reference;
  a hover/focus menu assigns Reference or Start frame for every provider and End
  frame for Seedance and Veo. Reference thumbnails have no badge, while explicit
  Start and End assignments show compact bottom labels. Assigning either frame
  in a two-image Seedance or Veo set creates a valid start/end pair; reference
  and frame workflows remain mutually exclusive. Duration sliders use the
  active provider's official discrete range and clamp when reference or
  resolution constraints narrow it.
- Keep Ark virtual-identity asset controls hidden from Settings and Seedance
  generation while Volcengine limits activation to enterprise-verified
  accounts. The native asset client remains available for a future entitled
  workflow; ordinary uploaded Seedance references remain visible.
- Video Stop messaging must state that it stops local monitoring and may not
  cancel the paid provider job.

## Typography And Color

- Use the existing system sans-serif stack.
- Keep compact panel headings around the current scale; do not introduce hero-sized type inside tool surfaces.
- Maintain the current neutral background and blue action/accent palette unless a broader redesign is requested.

## Open Decisions

- No mobile layout is currently defined; the desktop shell has a minimum 1100px width.
- No formal design tokens exist yet beyond the CSS values in `src/styles.css`.
- PromptCraft DSL-specific UI states are placeholders until the DSL renderer exists.
