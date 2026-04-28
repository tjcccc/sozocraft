export type GeminiImageModelId =
  | "gemini-3.1-flash-image-preview"
  | "gemini-3-pro-image-preview"
  | "gemini-2.5-flash-image";

export type GeminiThinkingLevel = "minimal" | "high";

export type GeminiImageModelConfig = {
  id: GeminiImageModelId;
  label: string;
  productName: string;
  aspectRatios: string[];
  imageSizes: string[] | null;
  thinkingLevels: GeminiThinkingLevel[] | null;
  maxReferenceImages: number;
  defaults: {
    aspectRatio: string;
    imageSize: string | null;
    thinkingLevel: GeminiThinkingLevel | null;
  };
};

export type GeminiImageOptions = {
  aspectRatio: string;
  imageSize: string;
  thinkingLevel: string;
};

export const GEMINI_IMAGE_MODELS: Record<GeminiImageModelId, GeminiImageModelConfig> = {
  "gemini-3.1-flash-image-preview": {
    id: "gemini-3.1-flash-image-preview",
    label: "Gemini 3.1 Flash Image Preview",
    productName: "Nano Banana 2",
    aspectRatios: [
      "Auto",
      "1:1",
      "1:4",
      "1:8",
      "2:3",
      "3:2",
      "3:4",
      "4:1",
      "4:3",
      "4:5",
      "5:4",
      "8:1",
      "9:16",
      "16:9",
      "21:9",
    ],
    imageSizes: ["512", "1K", "2K", "4K"],
    thinkingLevels: ["minimal", "high"],
    maxReferenceImages: 14,
    defaults: {
      aspectRatio: "1:1",
      imageSize: "2K",
      thinkingLevel: "minimal",
    },
  },
  "gemini-3-pro-image-preview": {
    id: "gemini-3-pro-image-preview",
    label: "Gemini 3 Pro Image Preview",
    productName: "Nano Banana Pro",
    aspectRatios: [
      "Auto",
      "1:1",
      "2:3",
      "3:2",
      "3:4",
      "4:3",
      "4:5",
      "5:4",
      "9:16",
      "16:9",
      "21:9",
    ],
    imageSizes: ["1K", "2K", "4K"],
    thinkingLevels: null,
    maxReferenceImages: 14,
    defaults: {
      aspectRatio: "1:1",
      imageSize: "2K",
      thinkingLevel: null,
    },
  },
  "gemini-2.5-flash-image": {
    id: "gemini-2.5-flash-image",
    label: "Gemini 2.5 Flash Image",
    productName: "Nano Banana",
    aspectRatios: [
      "Auto",
      "1:1",
      "2:3",
      "3:2",
      "3:4",
      "4:3",
      "4:5",
      "5:4",
      "9:16",
      "16:9",
      "21:9",
    ],
    imageSizes: null,
    thinkingLevels: null,
    maxReferenceImages: 3,
    defaults: {
      aspectRatio: "1:1",
      imageSize: null,
      thinkingLevel: null,
    },
  },
};

export const GEMINI_IMAGE_MODEL_IDS = Object.keys(
  GEMINI_IMAGE_MODELS,
) as GeminiImageModelId[];

export function getGeminiImageModelConfig(model: string): GeminiImageModelConfig {
  return GEMINI_IMAGE_MODELS[isGeminiImageModelId(model) ? model : GEMINI_IMAGE_MODEL_IDS[0]];
}

export function normalizeGeminiImageOptions(
  model: string,
  options: GeminiImageOptions,
): GeminiImageOptions {
  const config = getGeminiImageModelConfig(model);
  return {
    aspectRatio: config.aspectRatios.includes(options.aspectRatio)
      ? options.aspectRatio
      : config.defaults.aspectRatio,
    imageSize:
      config.imageSizes && config.imageSizes.includes(options.imageSize)
        ? options.imageSize
        : config.defaults.imageSize ?? "1K",
    thinkingLevel:
      config.thinkingLevels && config.thinkingLevels.includes(options.thinkingLevel as GeminiThinkingLevel)
        ? options.thinkingLevel
        : config.defaults.thinkingLevel ?? "minimal",
  };
}

export function geminiImageModelDisplayName(model: string): string {
  return getGeminiImageModelConfig(model).productName;
}

function isGeminiImageModelId(model: string): model is GeminiImageModelId {
  return Object.prototype.hasOwnProperty.call(GEMINI_IMAGE_MODELS, model);
}
