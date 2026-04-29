import { useEffect } from "react";
import type { AppSettings } from "../types";
import { normalizeProviderOptions } from "../models/imageProviders";

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
    });
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
