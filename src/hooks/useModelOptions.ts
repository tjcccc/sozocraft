import { useEffect } from "react";
import type { AppSettings } from "../types";
import { normalizeGeminiImageOptions } from "../models/geminiImageModels";

export function useModelOptions({
  aspectRatio,
  imageSize,
  settings,
  setAspectRatio,
  setImageSize,
  setThinkingLevel,
  thinkingLevel,
}: {
  aspectRatio: string;
  imageSize: string;
  settings: AppSettings | null;
  setAspectRatio: (value: string) => void;
  setImageSize: (value: string) => void;
  setThinkingLevel: (value: string) => void;
  thinkingLevel: string;
}) {
  useEffect(() => {
    if (!settings) {
      return;
    }

    const next = normalizeGeminiImageOptions(settings.defaultModel, {
      aspectRatio,
      imageSize,
      thinkingLevel,
    });
    if (next.aspectRatio !== aspectRatio) {
      setAspectRatio(next.aspectRatio);
    }
    if (next.imageSize !== imageSize) {
      setImageSize(next.imageSize);
    }
    if (next.thinkingLevel !== thinkingLevel) {
      setThinkingLevel(next.thinkingLevel);
    }
  }, [
    aspectRatio,
    imageSize,
    settings,
    setAspectRatio,
    setImageSize,
    setThinkingLevel,
    thinkingLevel,
  ]);
}
