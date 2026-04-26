# VisionCraft Next TODOs

## v0.1.x Stabilization

- Make `.{extension}` optional in output templates. If omitted, append the extension detected from the generated image MIME type.
- Save per-image sidecar metadata JSON next to each output image, including prompt source, final prompt snapshot, provider, model, generation options, filename template, timestamps, and response metadata.
- Add template validation feedback in the UI before generation, including unsupported date tokens, missing output directory, and examples for `yyyyMMdd_HHmmss`.
- Add a small config status panel showing the active `~/.visioncraft/config.toml` path, whether proxy is configured, and whether the Gemini API key is present.
- Improve failed batch history entries so the user can expand and inspect the exact provider error.

## Prompt Workflow

- Integrate `@promptcraft/core` behind the existing DSL toggle.
- Save both prompt source and rendered prompt snapshot for every generation.
- Show PromptCraft parse/render validation errors inline in the prompt editor.
- Add prompt library persistence for saved plain prompts and future DSL prompts.

## Output Workflow

- Add image context actions: reveal in Finder, copy path, copy prompt, and open metadata.
- Add output gallery filtering by provider/model/status/date.
- Add safe cleanup controls for failed batches and missing local files.

## Provider Roadmap

- Keep Nano Banana / Gemini as the only implemented provider until the MVP flow is stable.
- Add reference image upload for Gemini after prompt/history/output handling is reliable.
- Add GPT-Image, Grok Imagine, ComfyUI Local, and Prompt Bridge as separate provider/backend milestones.
