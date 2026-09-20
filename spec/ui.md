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
- system-synchronized light and deep blue-gray dark surfaces with subtle blue accents
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
- General Settings keeps the primary Output Directory and Filename Template,
  followed by a default-off Higgsfield Output toggle. Its dependent directory
  and filename-template fields are disabled while the toggle is off. The
  Higgsfield template supports `{higgsfield_filename}` and `{id:N}` alongside
  the shared output tokens.
- The Higgsfield CLI provider fieldset includes Use proxy and Proxy URL. The URL
  field is disabled with the toggle off; when left empty, its placeholder
  explains that the General Proxy URL is used as the fallback.
- Use icon buttons for compact toolbar/tool actions when the icon is familiar.
- The toolbar brand mark uses the light/dark layered-picture PNG with an apricot
  play triangle, selected by the system color scheme, on a transparent canvas
  without an enclosing background or CSS mask.
  Approved transparent foreground masters live in `assets/app-icon/`.
  `python3 scripts/build_app_icons.py` (requires Pillow) composes a subtle
  white-to-silver vertical gradient for light mode and solid charcoal for dark
  mode for native icons, shared artwork sizing, and padded rounded native icons.
  It regenerates toolbar PNGs, native light/dark PNGs, ICNS, ICO, and bundle sizes.
  Backgrounds and native masks are defined in code, separate from the artwork.
- The complete application follows the macOS light/dark appearance through
  `prefers-color-scheme`; dark mode retains the same information hierarchy,
  uses blue-gray surfaces instead of pure black, and lightens monochrome
  provider marks for contrast.
- Keep text labels on primary run/save actions.
- Provider tabs should be active only when the backend can generate through that
  provider. Keep provider-specific controls hidden when the active provider does
  not support them in the current implementation.
- Reference-image thumbnails wrap onto additional rows within the generation
  panel; do not introduce a horizontal thumbnail scrollbar.
- Video provider tabs appear as Seedance, Grok Imagine, and Google Video. The
  existing Grok selection remains the initial active provider for compatibility.
  Each provider preserves its own input images and control values while tabs
  switch provider-specific models, duration rules, ratios, resolutions, limits,
  and audio behavior.
- Seedance's Model control offers Seedance 2.0, Seedance 2.0 Fast, and Seedance
  2.0 Mini, plus Seedance 2.5. Changing models preserves valid settings and falls back to the
  selected model's defaults when a resolution is unavailable.
- Settings exposes a Seedance API Platform selector for Volcengine Ark and
  Higgsfield CLI plus a persisted Default Video Model selector with the same
  Standard, Fast, Mini, and 2.5 choices. The selected default initializes Seedance's
  main-panel model control.
- Video mode uses one wrapping Input Images field whose count limit follows the
  active provider. Its output panel uses native video playback and keeps video
  history separate from image history.
- Video input mode follows per-thumbnail roles. New uploads default to Reference;
  a hover/focus menu assigns Reference or Start frame for every provider and End
  frame for every video model. Reference thumbnails have no badge, while explicit
  Start and End assignments show compact bottom labels. Assigning either frame
  in a two-image set creates a valid start/end pair; reference
  and frame workflows remain mutually exclusive. Duration sliders use the
  active provider's official discrete range and clamp when reference or
  resolution constraints narrow it.
- Keep Ark virtual-identity asset controls hidden from Settings and Seedance
  generation while Volcengine limits activation to enterprise-verified
  accounts. The native asset client remains available for a future entitled
  workflow; ordinary uploaded Seedance references remain visible.
- Video Stop messaging must state that it stops local monitoring and may not
  cancel the paid provider job. Disable Stop for every active Higgsfield CLI
  image or video task because the CLI exposes no cancellation command, and
  explain that the remote job continues to completion.

## Typography And Color

- Use the existing system sans-serif stack.
- Keep compact panel headings around the current scale; do not introduce hero-sized type inside tool surfaces.
- Maintain the neutral background and blue action/accent palette in both system
  appearances unless a broader redesign is requested.

## Open Decisions

- No mobile layout is currently defined; the desktop shell has a minimum 1100px width.
- Theme values remain centralized in the light rules and the system dark-mode
  override in `src/styles.css`; no runtime theme preference is persisted yet.
- PromptCraft DSL-specific UI states are placeholders until the DSL renderer exists.

- Google Video groups Veo 3.1 and Gemini Omni Flash 1.1 under the existing Gemini
  API settings. Omni uses 3–10-second clips and supports six image references;
  its 1080p/4K choices are upscaled. Grok Imagine Video 1.5 enables audio control
  and 1080p only for text/single-frame input, with 720p maximum for references
  and start/end pairs. Input-mode changes immediately constrain resolution.

### Quick prompt workspace

- Quick sits beside DSL in the Prompt Editor header; DSL is disabled while Quick
  or its pending plain-text library draft is shown. The saved DSL preference is retained.
- Quick hides the library sidebar and name/tag fields. A tab strip with
  previous/next arrows provides eight numbered tabs, add, and close controls.
  The selected tab stays visible without a scrollbar. Nonempty closes
  require confirmation; closing the last tab leaves an empty Prompt 1.
- Quick tab labels and footer actions use compact 12px text.
- Turning Quick off opens name/tag fields and an explicit Save to Library action.
  No library file or database row is created before that action. Back to Library
  restores the existing library context without discarding Quick text.
- Autosave status and retry remain visible. Drafts flush on tab/mode changes,
  window blur, and window close. Generation uses the active text without DSL parsing.

- Image imports in Quick mode select a new temporary tab; at eight tabs they
  replace numbered Prompt 8. Session-local per-tab undo/redo includes imports
  and text edits, with Cmd/Ctrl+Z and Shift+Cmd/Ctrl+Z editor shortcuts.
