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
  maxInputImages?: number;
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
      {
        id: "doubao-seedance-2-5-260628",
        productName: "Seedance 2.5",
        resolutions: ["480p", "720p", "1080p"],
        durations: Array.from({ length: 27 }, (_, index) => index + 4),
        referenceDurations: Array.from({ length: 27 }, (_, index) => index + 4),
        defaultDuration: 5,
        defaultResolution: "720p",
        maxInputImages: 30,
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
      id: "grok-imagine-video-1.5",
      productName: "Grok Imagine Video 1.5",
      resolutions: ["480p", "720p", "1080p"],
      durations: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
      referenceDurations: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
      defaultDuration: 5,
      defaultResolution: "480p",
    }],
    aspectRatios: ["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"],
    maxInputImages: 7,
    defaultAspectRatio: "16:9",
    supportsAudioControl: true,
    audioAlwaysGenerated: false,
  },
  "google-veo": {
    id: "google-veo",
    label: "Google Video",
    platformLabel: "Google Gemini API",
    models: [{
      id: "veo-3.1-generate-preview",
      productName: "Veo 3.1",
      resolutions: ["720p", "1080p", "4k"],
      durations: [4, 6, 8],
      referenceDurations: [8],
      defaultDuration: 8,
      defaultResolution: "720p",
    }, {
      id: "gemini-omni-1.1-flash",
      productName: "Gemini Omni Flash 1.1",
      resolutions: ["360p", "720p", "1080p", "4k"],
      durations: [3, 4, 5, 6, 7, 8, 9, 10],
      referenceDurations: [3, 4, 5, 6, 7, 8, 9, 10],
      defaultDuration: 5,
      defaultResolution: "720p",
      maxInputImages: 6,
    }],
    aspectRatios: ["16:9", "9:16"],
    maxInputImages: 3,
    defaultAspectRatio: "16:9",
    supportsAudioControl: false,
    audioAlwaysGenerated: true,
  },
};

export function getVideoProviderConfig(provider: VideoProviderId, modelId?: string): VideoProviderConfig {
  const config = VIDEO_PROVIDERS[provider];
  const model = config.models.find((item) => item.id === modelId);
  return model?.maxInputImages === undefined ? config : { ...config, maxInputImages: model.maxInputImages };
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
  if (modelId === "veo-3.1-generate-preview" && (inputMode === "reference" || resolution !== "720p")) {
    return [8];
  }
  return inputMode === "reference" ? config.referenceDurations : config.durations;
}

export function nearestVideoDuration(value: number, allowed: readonly number[]): number {
  return allowed.reduce((nearest, candidate) =>
    Math.abs(candidate - value) < Math.abs(nearest - value) ? candidate : nearest,
  );
}

export function getVideoResolutions(provider: VideoProviderId, modelId: string, inputMode: VideoInputMode): readonly string[] {
  const resolutions = getVideoModelConfig(provider, modelId).resolutions;
  return provider === "grok-imagine" && (inputMode === "reference" || inputMode === "frames")
    ? resolutions.filter((value) => value !== "1080p")
    : resolutions;
}
