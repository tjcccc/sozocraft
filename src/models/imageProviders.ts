import {
  GEMINI_IMAGE_MODEL_IDS,
  getGeminiImageModelConfig,
  normalizeGeminiImageOptions,
} from "./geminiImageModels";

export type ImageProviderId = "nano-banana" | "gpt-image" | "grok-imagine";

export type ProviderModelConfig = {
  id: string;
  label: string;
  productName: string;
  maxReferenceImages?: number;
};

export type ProviderConfig = {
  id: ImageProviderId;
  label: string;
  providerName: string;
  models: ProviderModelConfig[];
  aspectRatios: string[];
  imageSizes: string[] | null;
  qualityLevels: string[] | null;
  thinkingLevels: string[] | null;
  maxReferenceImages: number;
  defaults: {
    model: string;
    aspectRatio: string;
    imageSize: string | null;
    quality: string | null;
    thinkingLevel: string | null;
  };
};

export const IMAGE_PROVIDERS: Record<ImageProviderId, ProviderConfig> = {
  "nano-banana": {
    id: "nano-banana",
    label: "Nano Banana",
    providerName: "Gemini",
    models: GEMINI_IMAGE_MODEL_IDS.map((id) => ({
      id,
      label: getGeminiImageModelConfig(id).label,
      productName: getGeminiImageModelConfig(id).productName,
    })),
    aspectRatios: getGeminiImageModelConfig("gemini-3-pro-image-preview").aspectRatios,
    imageSizes: getGeminiImageModelConfig("gemini-3-pro-image-preview").imageSizes,
    qualityLevels: null,
    thinkingLevels: null,
    maxReferenceImages: 14,
    defaults: {
      model: "gemini-3-pro-image-preview",
      aspectRatio: "1:1",
      imageSize: "2K",
      quality: null,
      thinkingLevel: null,
    },
  },
  "gpt-image": {
    id: "gpt-image",
    label: "GPT-Image",
    providerName: "OpenAI",
    models: [
      { id: "gpt-image-2", label: "GPT Image 2", productName: "GPT Image 2", maxReferenceImages: 0 },
    ],
    aspectRatios: [],
    imageSizes: ["auto", "1024x1024", "1536x1024", "1024x1536"],
    qualityLevels: ["auto", "low", "medium", "high"],
    thinkingLevels: null,
    maxReferenceImages: 0,
    defaults: {
      model: "gpt-image-2",
      aspectRatio: "auto",
      imageSize: "auto",
      quality: "auto",
      thinkingLevel: null,
    },
  },
  "grok-imagine": {
    id: "grok-imagine",
    label: "Grok Imagine",
    providerName: "xAI",
    models: [
      { id: "grok-imagine-image", label: "Grok Imagine Image", productName: "Grok Imagine" },
    ],
    aspectRatios: [
      "auto",
      "1:1",
      "3:4",
      "4:3",
      "9:16",
      "16:9",
      "2:3",
      "3:2",
      "9:19.5",
      "19.5:9",
      "9:20",
      "20:9",
      "1:2",
      "2:1",
    ],
    imageSizes: ["1k", "2k"],
    qualityLevels: ["low", "medium", "high"],
    thinkingLevels: null,
    maxReferenceImages: 5,
    defaults: {
      model: "grok-imagine-image",
      aspectRatio: "auto",
      imageSize: "1k",
      quality: "medium",
      thinkingLevel: null,
    },
  },
};

export const IMAGE_PROVIDER_IDS = Object.keys(IMAGE_PROVIDERS) as ImageProviderId[];

export function getProviderConfig(provider: string): ProviderConfig {
  return IMAGE_PROVIDERS[isImageProviderId(provider) ? provider : "nano-banana"];
}

export function getProviderModelDisplayName(provider: string, model: string): string {
  return getProviderConfig(provider).models.find((item) => item.id === model)?.productName ?? model;
}

export function normalizeProviderOptions(
  provider: string,
  model: string,
  options: {
    aspectRatio: string;
    imageSize: string;
    quality: string;
    thinkingLevel: string;
  },
) {
  if (provider === "nano-banana") {
    const normalized = normalizeGeminiImageOptions(model, options);
    const geminiConfig = getGeminiImageModelConfig(model);
    return {
      model,
      aspectRatio: normalized.aspectRatio,
      imageSize: normalized.imageSize,
      quality: "",
      thinkingLevel: normalized.thinkingLevel,
      maxReferenceImages: geminiConfig.maxReferenceImages,
    };
  }

  const config = getProviderConfig(provider);
  const normalizedModel = config.models.some((item) => item.id === model) ? model : config.defaults.model;
  const modelConfig = config.models.find((item) => item.id === normalizedModel);
  return {
    model: normalizedModel,
    aspectRatio: config.aspectRatios.includes(options.aspectRatio)
      ? options.aspectRatio
      : config.defaults.aspectRatio,
    imageSize:
      config.imageSizes && config.imageSizes.includes(options.imageSize)
        ? options.imageSize
        : config.defaults.imageSize ?? "",
    quality:
      config.qualityLevels && config.qualityLevels.includes(options.quality)
        ? options.quality
        : config.defaults.quality ?? "",
    thinkingLevel: config.defaults.thinkingLevel ?? "",
    maxReferenceImages: modelConfig?.maxReferenceImages ?? config.maxReferenceImages,
  };
}

export function isImageProviderId(provider: string): provider is ImageProviderId {
  return Object.prototype.hasOwnProperty.call(IMAGE_PROVIDERS, provider);
}
