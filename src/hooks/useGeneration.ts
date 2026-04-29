import { useCallback, useState } from "react";
import { generateImages, saveAppSettings, saveCurrentPrompt } from "../api";
import type { AppSettings, GenerationBatch, ReferenceImageInput } from "../types";
import type { AppStatus } from "../components/common";

export function useGeneration({
  prompt,
  setBatches,
  setExpandedBatchId,
  setMessage,
  setPreviewBatchId,
  setStatus,
  settings,
}: {
  prompt: string;
  setBatches: React.Dispatch<React.SetStateAction<GenerationBatch[]>>;
  setExpandedBatchId: (id: string | null) => void;
  setMessage: (message: string) => void;
  setPreviewBatchId: (id: string | null) => void;
  setStatus: (status: AppStatus) => void;
  settings: AppSettings | null;
}) {
  const [batchCount, setBatchCount] = useState(1);
  const [aspectRatio, setAspectRatio] = useState("3:4");
  const [imageSize, setImageSize] = useState("2K");
  const [temperature, setTemperature] = useState(-1);
  const [topP, setTopP] = useState(0.95);
  const [quality, setQuality] = useState("auto");
  const [thinkingLevel, setThinkingLevel] = useState("minimal");
  const [referenceImages, setReferenceImages] = useState<ReferenceImageInput[]>([]);

  const runGeneration = useCallback(async () => {
    if (!settings) {
      return;
    }
    setStatus("running");
    setMessage("Generating images");
    await saveCurrentPrompt(prompt);

    try {
      await saveAppSettings(settings);
      const batch = await generateImages({
        provider: settings.defaultProvider,
        model: settings.defaultModel,
        prompt,
        promptSnapshot: prompt,
        batchCount,
        referenceImages,
        outputTemplate: settings.outputTemplate,
        baseUrl:
          settings.defaultProvider === "gpt-image"
            ? settings.openaiBaseUrl
            : settings.defaultProvider === "grok-imagine"
              ? settings.xaiBaseUrl
              : settings.optionalBaseUrl,
        options: {
          aspectRatio,
          imageSize,
          temperature,
          topP,
          thinkingLevel,
          quality,
        },
      });
      setBatches((current) => [batch, ...current.filter((item) => item.id !== batch.id)]);
      setPreviewBatchId(batch.id);
      setExpandedBatchId(null);
      setStatus("ready");
      setMessage(`Completed ${batch.images.length} image${batch.images.length === 1 ? "" : "s"}`);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [
    aspectRatio,
    batchCount,
    imageSize,
    prompt,
    quality,
    referenceImages,
    setBatches,
    setExpandedBatchId,
    setMessage,
    setPreviewBatchId,
    setStatus,
    settings,
    temperature,
    thinkingLevel,
    topP,
  ]);

  return {
    aspectRatio,
    batchCount,
    imageSize,
    quality,
    temperature,
    thinkingLevel,
    topP,
    runGeneration,
    referenceImages,
    setAspectRatio,
    setBatchCount,
    setImageSize,
    setQuality,
    setReferenceImages,
    setTemperature,
    setThinkingLevel,
    setTopP,
  };
}
