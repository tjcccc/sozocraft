import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ArkAsset,
  AppState,
  ConfigStatus,
  CreatePromptRequest,
  CreateArkAssetRequest,
  GenerationBatch,
  GenerationRequest,
  HiggsfieldStatus,
  ImageTextMetadata,
  PromptDocument,
  PromptListItem,
  RenamePromptTagRequest,
  RenderPromptResult,
  SavePromptRequest,
  UpdatePromptMetadataRequest,
  VideoGenerationRequest,
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

export function setOpenrouterApiKey(apiKey: string) {
  return invoke<boolean>("set_openrouter_api_key", { apiKey });
}

export function setXaiApiKey(apiKey: string) {
  return invoke<boolean>("set_xai_api_key", { apiKey });
}

export function setArkApiKey(apiKey: string) {
  return invoke<boolean>("set_ark_api_key", { apiKey });
}

export function setArkAssetCredentials(accessKey: string, secretKey: string) {
  return invoke<boolean>("set_ark_asset_credentials", { accessKey, secretKey });
}

export function hasGeminiApiKey() {
  return invoke<boolean>("has_gemini_api_key");
}

export function hasOpenaiApiKey() {
  return invoke<boolean>("has_openai_api_key");
}

export function hasOpenrouterApiKey() {
  return invoke<boolean>("has_openrouter_api_key");
}

export function hasXaiApiKey() {
  return invoke<boolean>("has_xai_api_key");
}

export function hasArkApiKey() {
  return invoke<boolean>("has_ark_api_key");
}

export function hasArkAssetCredentials() {
  return invoke<boolean>("has_ark_asset_credentials");
}

export function listArkAssets() {
  return invoke<ArkAsset[]>("list_ark_assets");
}

export function createArkAsset(request: CreateArkAssetRequest) {
  return invoke<ArkAsset>("create_ark_asset", { request });
}

export function generateImages(request: GenerationRequest) {
  return invoke<GenerationBatch>("generate_images", { request });
}

export function generateVideo(request: VideoGenerationRequest) {
  return invoke<GenerationBatch>("generate_video", { request });
}

export function cancelGenerationTask(taskId: string) {
  return invoke<boolean>("cancel_generation_task", { taskId });
}

export function readImageDataUrl(path: string) {
  return invoke<string>("read_image_data_url", { path });
}

export function prepareVideoPreview(path: string) {
  return invoke<string>("prepare_video_preview", { path });
}

export function readTextFile(path: string) {
  return invoke<string>("read_text_file", { path });
}

export function readImageTextMetadata(path: string) {
  return invoke<ImageTextMetadata>("read_image_text_metadata", { path });
}

export function exportRenderedPrompt(outputPath: string, renderedPrompt: string) {
  return invoke<string>("export_rendered_prompt", { outputPath, renderedPrompt });
}

export function saveOutputTemplate(template: string) {
  return invoke<void>("save_output_template", { template });
}

export function getConfigStatus() {
  return invoke<ConfigStatus>("get_config_status");
}

export function checkHiggsfieldStatus(cliPath?: string | null, proxyUrl?: string | null) {
  return invoke<HiggsfieldStatus>("check_higgsfield_status", {
    cliPath: cliPath ?? null,
    proxyUrl: proxyUrl ?? null,
  });
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

export function renamePromptTag(promptDirectory: string, request: RenamePromptTagRequest) {
  return invoke<PromptListItem[]>("rename_prompt_tag", { promptDirectory, request });
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
