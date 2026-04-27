import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, AppState, ConfigStatus, GenerationBatch, GenerationRequest } from "./types";

export function loadAppState() {
  return invoke<AppState>("load_app_state");
}

export function saveAppSettings(settings: AppSettings) {
  return invoke<AppState>("save_app_settings", { settings });
}

export function saveCurrentPrompt(prompt: string) {
  return invoke<AppState>("save_current_prompt", { prompt });
}

export function setGeminiApiKey(apiKey: string) {
  return invoke<boolean>("set_gemini_api_key", { apiKey });
}

export function hasGeminiApiKey() {
  return invoke<boolean>("has_gemini_api_key");
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
