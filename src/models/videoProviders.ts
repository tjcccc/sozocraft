import type { VideoInputMode, VideoProviderId } from "../types";

export type VideoProviderConfig = {
  id: VideoProviderId;
  label: string;
  platformLabel: string;
  models: readonly VideoModelConfig[];
  aspectRatios: readonly string[];
  maxInputImages: number;
  defaultAspectRatio: string;
  supportsAudioControl: boolean;
  audioAlwaysGenerated: boolean;
};

export type VideoModelConfig = {
  id: string;
  productName: string;
  resolutions: readonly string[];
  durations: readonly number[];
  referenceDurations: readonly number[];
  defaultDuration: number;
  defaultResolution: string;
};

export const VIDEO_PROVIDER_IDS = ["seedance", "grok-imagine", "google-veo"] as const;

const VIDEO_PROVIDERS: Record<VideoProviderId, VideoProviderConfig> = {
  seedance: {
    id: "seedance",
    label: "Seedance",
    platformLabel: "Volcengine Ark",
    models: [
      {
        id: "doubao-seedance-2-0-260128",
        productName: "Seedance 2.0",
        resolutions: ["480p", "720p", "1080p"],
        durations: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        referenceDurations: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        defaultDuration: 5,
        defaultResolution: "720p",
      },
      {
        id: "doubao-seedance-2-0-fast-260128",
        productName: "Seedance 2.0 Fast",
        resolutions: ["480p", "720p"],
        durations: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        referenceDurations: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        defaultDuration: 5,
        defaultResolution: "720p",
      },
      {
        id: "doubao-seedance-2-0-mini-260615",
        productName: "Seedance 2.0 Mini",
        resolutions: ["480p", "720p"],
        durations: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        referenceDurations: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        defaultDuration: 5,
        defaultResolution: "720p",
      },
    ],
    aspectRatios: ["16:9", "9:16", "4:3", "3:4", "1:1", "21:9"],
    maxInputImages: 9,
    defaultAspectRatio: "16:9",
    supportsAudioControl: true,
    audioAlwaysGenerated: false,
  },
  "grok-imagine": {
    id: "grok-imagine",
    label: "Grok Imagine",
    platformLabel: "xAI",
    models: [{
      id: "grok-imagine-video",
      productName: "Grok Imagine Video",
      resolutions: ["480p", "720p"],
      durations: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
      referenceDurations: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
      defaultDuration: 5,
      defaultResolution: "480p",
    }],
    aspectRatios: ["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"],
    maxInputImages: 7,
    defaultAspectRatio: "16:9",
    supportsAudioControl: false,
    audioAlwaysGenerated: false,
  },
  "google-veo": {
    id: "google-veo",
    label: "Google Veo",
    platformLabel: "Google Gemini API",
    models: [{
      id: "veo-3.1-generate-preview",
      productName: "Veo 3.1",
      resolutions: ["720p", "1080p", "4k"],
      durations: [4, 6, 8],
      referenceDurations: [8],
      defaultDuration: 8,
      defaultResolution: "720p",
    }],
    aspectRatios: ["16:9", "9:16"],
    maxInputImages: 3,
    defaultAspectRatio: "16:9",
    supportsAudioControl: false,
    audioAlwaysGenerated: true,
  },
};

export function getVideoProviderConfig(provider: VideoProviderId): VideoProviderConfig {
  return VIDEO_PROVIDERS[provider];
}

export function getVideoModelConfig(
  provider: VideoProviderId,
  modelId?: string,
): VideoModelConfig {
  const config = getVideoProviderConfig(provider);
  return config.models.find((model) => model.id === modelId) ?? config.models[0];
}

export function getVideoDurations(
  provider: VideoProviderId,
  modelId: string,
  inputMode: VideoInputMode,
  resolution: string,
): readonly number[] {
  const config = getVideoModelConfig(provider, modelId);
  if (provider === "google-veo" && (inputMode === "reference" || resolution !== "720p")) {
    return [8];
  }
  return inputMode === "reference" ? config.referenceDurations : config.durations;
}

export function nearestVideoDuration(value: number, allowed: readonly number[]): number {
  return allowed.reduce((nearest, candidate) =>
    Math.abs(candidate - value) < Math.abs(nearest - value) ? candidate : nearest,
  );
}
