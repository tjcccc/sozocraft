import {
  GEMINI_IMAGE_MODEL_IDS,
  getGeminiImageModelConfig,
  normalizeGeminiImageOptions,
} from "./geminiImageModels";

export type ImageProviderId = "nano-banana" | "gpt-image" | "grok-imagine";
export type NanoBananaApiPlatform = "gemini" | "higgsfield";
export type GptImageApiPlatform = "openai" | "openrouter" | "higgsfield";
export type GrokImagineApiPlatform = "xai" | "higgsfield";
export type ImageProviderApiPlatform =
  | NanoBananaApiPlatform
  | GptImageApiPlatform
  | GrokImagineApiPlatform;

export type ProviderModelConfig = {
  id: string;
  label: string;
  productName: string;
  aspectRatios?: string[];
  imageSizes?: string[] | null;
  qualityLevels?: string[] | null;
  qualityLabel?: string;
  maxReferenceImages?: number;
  defaults?: Partial<ProviderConfig["defaults"]>;
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

const HIGGSFIELD_PRO_ASPECT_RATIOS = [
  "auto",
  "1:1",
  "3:2",
  "2:3",
  "4:3",
  "3:4",
  "4:5",
  "5:4",
  "9:16",
  "16:9",
  "21:9",
];

const HIGGSFIELD_NANO_BANANA_ASPECT_RATIOS = [
  "1:1",
  "3:2",
  "2:3",
  "4:3",
  "3:4",
  "4:5",
  "5:4",
  "9:16",
  "16:9",
  "21:9",
];
const HIGGSFIELD_GPT_IMAGE_ASPECT_RATIOS = ["1:1", "4:3", "3:4", "16:9", "9:16", "3:2", "2:3"];
const HIGGSFIELD_GROK_ASPECT_RATIOS = ["1:1", "4:3", "3:4", "16:9", "9:16"];
const HIGGSFIELD_RESOLUTIONS = ["1k", "2k", "4k"];

const GEMINI_NANO_BANANA_MODELS: ProviderModelConfig[] = GEMINI_IMAGE_MODEL_IDS.map((id) => ({
  id,
  label: getGeminiImageModelConfig(id).label,
  productName: getGeminiImageModelConfig(id).productName,
}));

const HIGGSFIELD_NANO_BANANA_MODELS: ProviderModelConfig[] = [
  {
    id: "nano_banana_2",
    label: "Nano Banana Pro",
    productName: "Nano Banana Pro",
    aspectRatios: HIGGSFIELD_PRO_ASPECT_RATIOS,
    imageSizes: HIGGSFIELD_RESOLUTIONS,
    maxReferenceImages: 8,
    defaults: { aspectRatio: "1:1", imageSize: "2k" },
  },
  {
    id: "nano_banana_flash",
    label: "Nano Banana 2",
    productName: "Nano Banana 2",
    aspectRatios: HIGGSFIELD_NANO_BANANA_ASPECT_RATIOS,
    imageSizes: HIGGSFIELD_RESOLUTIONS,
    maxReferenceImages: 8,
    defaults: { aspectRatio: "1:1", imageSize: "2k" },
  },
  {
    id: "nano_banana",
    label: "Nano Banana",
    productName: "Nano Banana",
    aspectRatios: HIGGSFIELD_NANO_BANANA_ASPECT_RATIOS,
    imageSizes: null,
    maxReferenceImages: 8,
    defaults: { aspectRatio: "1:1", imageSize: null },
  },
];

const OPENAI_GPT_IMAGE_MODELS: ProviderModelConfig[] = [
  { id: "gpt-image-2", label: "GPT Image 2", productName: "GPT Image 2", maxReferenceImages: 16 },
  {
    id: "openai/gpt-5.4-image-2",
    label: "GPT-5.4 Image 2",
    productName: "GPT-5.4 Image 2",
    maxReferenceImages: 16,
  },
  {
    id: "gpt_image_2",
    label: "GPT Image 2",
    productName: "GPT Image 2",
    aspectRatios: HIGGSFIELD_GPT_IMAGE_ASPECT_RATIOS,
    imageSizes: HIGGSFIELD_RESOLUTIONS,
    qualityLevels: ["low", "medium", "high"],
    maxReferenceImages: 16,
    defaults: { aspectRatio: "1:1", imageSize: "2k", quality: "high" },
  },
];

const XAI_GROK_IMAGE_MODELS: ProviderModelConfig[] = [
  {
    id: "grok-imagine-image-quality",
    label: "Grok Imagine Quality",
    productName: "Grok Imagine Quality",
  },
  { id: "grok-imagine-image", label: "Grok Imagine", productName: "Grok Imagine Standard" },
  {
    id: "grok_image",
    label: "Grok Image",
    productName: "Grok Image",
    aspectRatios: HIGGSFIELD_GROK_ASPECT_RATIOS,
    imageSizes: null,
    qualityLevels: ["std", "pro"],
    qualityLabel: "Quality",
    maxReferenceImages: 5,
    defaults: { aspectRatio: "1:1", imageSize: null, quality: "std" },
  },
];

export const IMAGE_PROVIDERS: Record<ImageProviderId, ProviderConfig> = {
  "nano-banana": {
    id: "nano-banana",
    label: "Nano Banana",
    providerName: "Gemini",
    models: [...GEMINI_NANO_BANANA_MODELS, ...HIGGSFIELD_NANO_BANANA_MODELS],
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
    providerName: "GPT-Image",
    models: OPENAI_GPT_IMAGE_MODELS,
    aspectRatios: [],
    imageSizes: [
      "auto",
      "1024x1024",
      "1536x1024",
      "1024x1536",
      "2048x2048",
      "2048x1152",
      "3840x2160",
      "2160x3840",
    ],
    qualityLevels: ["auto", "low", "medium", "high"],
    thinkingLevels: null,
    maxReferenceImages: 16,
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
    models: XAI_GROK_IMAGE_MODELS,
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
      model: "grok-imagine-image-quality",
      aspectRatio: "auto",
      imageSize: "1k",
      quality: "medium",
      thinkingLevel: null,
    },
  },
};

export const NANO_BANANA_PLATFORM_MODELS: Record<NanoBananaApiPlatform, string> = {
  gemini: "gemini-3-pro-image-preview",
  higgsfield: "nano_banana_2",
};

export const GPT_IMAGE_PLATFORM_MODELS: Record<GptImageApiPlatform, string> = {
  openai: "gpt-image-2",
  openrouter: "openai/gpt-5.4-image-2",
  higgsfield: "gpt_image_2",
};

export const GROK_IMAGE_PLATFORM_MODELS: Record<GrokImagineApiPlatform, string> = {
  xai: "grok-imagine-image-quality",
  higgsfield: "grok_image",
};

export const IMAGE_PROVIDER_IDS = Object.keys(IMAGE_PROVIDERS) as ImageProviderId[];

export function getProviderConfig(provider: string): ProviderConfig {
  return IMAGE_PROVIDERS[isImageProviderId(provider) ? provider : "nano-banana"];
}

export function getProviderModels(provider: string, platform: ImageProviderApiPlatform = "openai") {
  const config = getProviderConfig(provider);
  if (provider === "nano-banana") {
    return platform === "higgsfield" ? HIGGSFIELD_NANO_BANANA_MODELS : GEMINI_NANO_BANANA_MODELS;
  }
  if (provider === "gpt-image") {
    const model = GPT_IMAGE_PLATFORM_MODELS[isGptImageApiPlatform(platform) ? platform : "openai"];
    return config.models.filter((item) => item.id === model);
  }
  if (provider === "grok-imagine") {
    return platform === "higgsfield"
      ? XAI_GROK_IMAGE_MODELS.filter((item) => item.id === "grok_image")
      : XAI_GROK_IMAGE_MODELS.filter((item) => item.id !== "grok_image");
  }
  return config.models;
}

export function getProviderControlConfig(
  provider: string,
  model: string,
  platform: ImageProviderApiPlatform = "openai",
) {
  if (provider === "nano-banana" && platform !== "higgsfield") {
    const geminiConfig = getGeminiImageModelConfig(model);
    return {
      aspectRatios: geminiConfig.aspectRatios,
      imageSizes: geminiConfig.imageSizes,
      qualityLevels: null,
      qualityLabel: "Quality",
      thinkingLevels: geminiConfig.thinkingLevels,
      maxReferenceImages: geminiConfig.maxReferenceImages,
      defaults: {
        model,
        aspectRatio: "1:1",
        imageSize: geminiConfig.defaults.imageSize,
        quality: null,
        thinkingLevel: geminiConfig.defaults.thinkingLevel,
      },
    };
  }

  const config = getProviderConfig(provider);
  const modelConfig = getProviderModels(provider, platform).find((item) => item.id === model);
  const defaults = {
    ...config.defaults,
    ...(modelConfig?.defaults ?? {}),
  };

  return {
    aspectRatios: modelConfig?.aspectRatios ?? config.aspectRatios,
    imageSizes: modelConfig?.imageSizes === undefined ? config.imageSizes : modelConfig.imageSizes,
    qualityLevels: modelConfig?.qualityLevels === undefined ? config.qualityLevels : modelConfig.qualityLevels,
    qualityLabel: modelConfig?.qualityLabel ?? "Quality",
    thinkingLevels: config.thinkingLevels,
    maxReferenceImages: modelConfig?.maxReferenceImages ?? config.maxReferenceImages,
    defaults,
  };
}

export function getProviderModelDisplayName(provider: string, model: string): string {
  return getProviderConfig(provider).models.find((item) => item.id === model)?.productName ?? model;
}

export function getImageSizeDisplayName(provider: string, imageSize: string): string {
  if (provider !== "gpt-image") {
    return imageSize;
  }
  return GPT_IMAGE_SIZE_LABELS[imageSize] ?? imageSize;
}

const GPT_IMAGE_SIZE_LABELS: Record<string, string> = {
  auto: "auto",
  "1024x1024": "1:1 (1024x1024)",
  "1536x1024": "3:2 (1536x1024)",
  "1024x1536": "2:3 (1024x1536)",
  "2048x2048": "1:1 (2048x2048)",
  "2048x1152": "16:9 (2048x1152)",
  "3840x2160": "16:9 (3840x2160)",
  "2160x3840": "9:16 (2160x3840)",
};

export function normalizeProviderOptions(
  provider: string,
  model: string,
  options: {
    aspectRatio: string;
    imageSize: string;
    quality: string;
    thinkingLevel: string;
  },
  platform: ImageProviderApiPlatform = "openai",
) {
  if (provider === "nano-banana" && platform !== "higgsfield") {
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

  const models = getProviderModels(provider, platform);
  const fallbackModel = defaultModelForPlatform(provider, platform);
  const normalizedModel = models.some((item) => item.id === model) ? model : fallbackModel;
  const controls = getProviderControlConfig(provider, normalizedModel, platform);
  return {
    model: normalizedModel,
    aspectRatio: controls.aspectRatios.includes(options.aspectRatio)
      ? options.aspectRatio
      : controls.defaults.aspectRatio,
    imageSize:
      controls.imageSizes && controls.imageSizes.includes(options.imageSize)
        ? options.imageSize
        : controls.defaults.imageSize ?? "",
    quality:
      controls.qualityLevels && controls.qualityLevels.includes(options.quality)
        ? options.quality
        : controls.defaults.quality ?? "",
    thinkingLevel: controls.defaults.thinkingLevel ?? "",
    maxReferenceImages: controls.maxReferenceImages,
  };
}

export function isImageProviderId(provider: string): provider is ImageProviderId {
  return Object.prototype.hasOwnProperty.call(IMAGE_PROVIDERS, provider);
}

function defaultModelForPlatform(provider: string, platform: ImageProviderApiPlatform) {
  if (provider === "nano-banana") {
    return NANO_BANANA_PLATFORM_MODELS[platform === "higgsfield" ? "higgsfield" : "gemini"];
  }
  if (provider === "gpt-image") {
    return GPT_IMAGE_PLATFORM_MODELS[isGptImageApiPlatform(platform) ? platform : "openai"];
  }
  if (provider === "grok-imagine") {
    return GROK_IMAGE_PLATFORM_MODELS[platform === "higgsfield" ? "higgsfield" : "xai"];
  }
  return getProviderConfig(provider).defaults.model;
}

function isGptImageApiPlatform(platform: ImageProviderApiPlatform): platform is GptImageApiPlatform {
  return platform === "openai" || platform === "openrouter" || platform === "higgsfield";
}
