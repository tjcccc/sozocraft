import { useEffect } from "react";
import type { AppSettings } from "../types";
import { normalizeProviderOptions } from "../models/imageProviders";
import type { ImageProviderApiPlatform, ImageProviderId } from "../models/imageProviders";

export function useModelOptions({
  aspectRatio,
  imageSize,
  quality,
  settings,
  setAspectRatio,
  setImageSize,
  setQuality,
  setThinkingLevel,
  thinkingLevel,
}: {
  aspectRatio: string;
  imageSize: string;
  quality: string;
  settings: AppSettings | null;
  setAspectRatio: (value: string) => void;
  setImageSize: (value: string) => void;
  setQuality: (value: string) => void;
  setThinkingLevel: (value: string) => void;
  thinkingLevel: string;
}) {
  useEffect(() => {
    if (!settings) {
      return;
    }

    const next = normalizeProviderOptions(settings.defaultProvider, settings.defaultModel, {
      aspectRatio,
      imageSize,
      quality,
      thinkingLevel,
    }, settingsPlatformForProvider(settings, settings.defaultProvider));
    if (next.aspectRatio !== aspectRatio) {
      setAspectRatio(next.aspectRatio);
    }
    if (next.imageSize !== imageSize) {
      setImageSize(next.imageSize);
    }
    if (next.quality !== quality) {
      setQuality(next.quality);
    }
    if (next.thinkingLevel !== thinkingLevel) {
      setThinkingLevel(next.thinkingLevel);
    }
  }, [
    aspectRatio,
    imageSize,
    quality,
    settings,
    setAspectRatio,
    setImageSize,
    setQuality,
    setThinkingLevel,
    thinkingLevel,
  ]);
}

function settingsPlatformForProvider(
  settings: AppSettings,
  provider: ImageProviderId,
): ImageProviderApiPlatform {
  if (provider === "nano-banana") {
    return settings.nanoBananaApiPlatform;
  }
  if (provider === "grok-imagine") {
    return settings.grokApiPlatform;
  }
  return settings.openaiApiPlatform;
}
