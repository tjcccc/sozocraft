import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AppState,
  ConfigStatus,
  CreatePromptRequest,
  GenerationBatch,
  GenerationRequest,
  PromptDocument,
  PromptListItem,
  RenderPromptResult,
  SavePromptRequest,
  UpdatePromptMetadataRequest,
} from "./types";

export function loadAppState() {
  return invoke<AppState>("load_app_state");
}

export function saveAppSettings(settings: AppSettings) {
  return invoke<AppState>("save_app_settings", { settings });
}

export function saveCurrentPrompt(prompt: string) {
  return invoke<AppState>("save_current_prompt", { prompt });
}

export function saveCurrentPromptId(promptId: string | null) {
  return invoke<AppState>("save_current_prompt_id", { promptId });
}

export function setGeminiApiKey(apiKey: string) {
  return invoke<boolean>("set_gemini_api_key", { apiKey });
}

export function setOpenaiApiKey(apiKey: string) {
  return invoke<boolean>("set_openai_api_key", { apiKey });
}

export function setXaiApiKey(apiKey: string) {
  return invoke<boolean>("set_xai_api_key", { apiKey });
}

export function hasGeminiApiKey() {
  return invoke<boolean>("has_gemini_api_key");
}

export function hasOpenaiApiKey() {
  return invoke<boolean>("has_openai_api_key");
}

export function hasXaiApiKey() {
  return invoke<boolean>("has_xai_api_key");
}

export function generateImages(request: GenerationRequest) {
  return invoke<GenerationBatch>("generate_images", { request });
}

export function readImageDataUrl(path: string) {
  return invoke<string>("read_image_data_url", { path });
}

export function saveOutputTemplate(template: string) {
  return invoke<void>("save_output_template", { template });
}

export function getConfigStatus() {
  return invoke<ConfigStatus>("get_config_status");
}

export function listPrompts(promptDirectory: string, query?: string) {
  return invoke<PromptListItem[]>("list_prompts", {
    promptDirectory,
    query: query || null,
  });
}

export function rescanPromptLibrary(promptDirectory: string) {
  return invoke<PromptListItem[]>("rescan_prompt_library", { promptDirectory });
}

export function createPrompt(promptDirectory: string, request: CreatePromptRequest) {
  return invoke<PromptDocument>("create_prompt", { promptDirectory, request });
}

export function readPrompt(promptDirectory: string, id: string) {
  return invoke<PromptDocument>("read_prompt", { promptDirectory, id });
}

export function savePrompt(promptDirectory: string, request: SavePromptRequest) {
  return invoke<PromptDocument>("save_prompt", { promptDirectory, request });
}

export function updatePromptMetadata(promptDirectory: string, request: UpdatePromptMetadataRequest) {
  return invoke<PromptListItem>("update_prompt_metadata", { promptDirectory, request });
}

export function deletePrompt(promptDirectory: string, id: string) {
  return invoke<void>("delete_prompt", { promptDirectory, id });
}

export function renderPromptSource(
  source: string,
  promptDirectory?: string,
  currentPromptId?: string | null,
) {
  return invoke<RenderPromptResult>("render_prompt_source", {
    source,
    promptDirectory: promptDirectory ?? null,
    currentPromptId: currentPromptId ?? null,
  });
}
