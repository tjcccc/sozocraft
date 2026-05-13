import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelGenerationTask,
  generateImages,
  loadAppState,
  saveAppSettings,
  saveCurrentPrompt,
} from "../api";
import type { AppSettings, GenerationBatch, GenerationRequest, ReferenceImageInput } from "../types";
import type { AppStatus } from "../components/common";
import { getProviderControlConfig } from "../models/imageProviders";
import type { ImageProviderApiPlatform, ImageProviderId } from "../models/imageProviders";
import { isSupportedImagePath, pathToReferenceImage } from "../utils/referenceImages";

type QueuedGenerationTask = {
  id: string;
  request: GenerationRequest;
  settings: AppSettings;
};

type ReferenceImagesByProvider = Record<ImageProviderId, ReferenceImageInput[]>;
type GenerationOptionsByProvider = Record<ImageProviderId, GenerationOptionState>;
type GenerationOptionState = {
  batchCount: number;
  aspectRatio: string;
  imageSize: string;
  temperature: number;
  topP: number;
  quality: string;
  thinkingLevel: string;
  unlimited: boolean;
};

const EMPTY_REFERENCE_IMAGES: ReferenceImagesByProvider = {
  "nano-banana": [],
  "gpt-image": [],
  "grok-imagine": [],
};

const DEFAULT_GENERATION_OPTIONS: GenerationOptionsByProvider = {
  "nano-banana": {
    batchCount: 1,
    aspectRatio: "1:1",
    imageSize: "2K",
    temperature: -1,
    topP: 0.95,
    quality: "",
    thinkingLevel: "",
    unlimited: false,
  },
  "gpt-image": {
    batchCount: 1,
    aspectRatio: "auto",
    imageSize: "auto",
    temperature: -1,
    topP: 0.95,
    quality: "auto",
    thinkingLevel: "",
    unlimited: false,
  },
  "grok-imagine": {
    batchCount: 1,
    aspectRatio: "auto",
    imageSize: "1k",
    temperature: -1,
    topP: 0.95,
    quality: "medium",
    thinkingLevel: "",
    unlimited: false,
  },
};

export function useGeneration({
  getPrompt,
  getPromptSnapshot,
  setBatches,
  setExpandedBatchId,
  setMessage,
  setPreviewBatchId,
  setStatus,
  settings,
}: {
  getPrompt: () => Promise<string> | string;
  getPromptSnapshot: () => string;
  setBatches: React.Dispatch<React.SetStateAction<GenerationBatch[]>>;
  setExpandedBatchId: (id: string | null) => void;
  setMessage: (message: string) => void;
  setPreviewBatchId: (id: string | null) => void;
  setStatus: (status: AppStatus) => void;
  settings: AppSettings | null;
}) {
  const [optionsByProvider, setOptionsByProvider] =
    useState<GenerationOptionsByProvider>(DEFAULT_GENERATION_OPTIONS);
  const [referenceImagesByProvider, setReferenceImagesByProvider] =
    useState<ReferenceImagesByProvider>(EMPTY_REFERENCE_IMAGES);
  const [queuedTasks, setQueuedTasks] = useState<QueuedGenerationTask[]>([]);
  const [runningTask, setRunningTask] = useState<QueuedGenerationTask | null>(null);
  const queueRef = useRef<QueuedGenerationTask[]>([]);
  const processingRef = useRef(false);
  const activeProvider = settings?.defaultProvider ?? "nano-banana";
  const activeOptions = optionsByProvider[activeProvider];
  const referenceImages = referenceImagesByProvider[activeProvider];

  const setActiveOption = useCallback(
    <K extends keyof GenerationOptionState>(key: K, value: GenerationOptionState[K]) => {
      setOptionsByProvider((current) => ({
        ...current,
        [activeProvider]: {
          ...current[activeProvider],
          [key]: value,
        },
      }));
    },
    [activeProvider],
  );

  const setBatchCount = useCallback((value: number) => setActiveOption("batchCount", value), [setActiveOption]);
  const setAspectRatio = useCallback((value: string) => setActiveOption("aspectRatio", value), [setActiveOption]);
  const setImageSize = useCallback((value: string) => setActiveOption("imageSize", value), [setActiveOption]);
  const setTemperature = useCallback((value: number) => setActiveOption("temperature", value), [setActiveOption]);
  const setTopP = useCallback((value: number) => setActiveOption("topP", value), [setActiveOption]);
  const setQuality = useCallback((value: string) => setActiveOption("quality", value), [setActiveOption]);
  const setThinkingLevel = useCallback((value: string) => setActiveOption("thinkingLevel", value), [setActiveOption]);
  const setUnlimited = useCallback((value: boolean) => setActiveOption("unlimited", value), [setActiveOption]);

  const setReferenceImages: React.Dispatch<React.SetStateAction<ReferenceImageInput[]>> =
    useCallback(
      (update) => {
        setReferenceImagesByProvider((current) => {
          const currentImages = current[activeProvider];
          const nextImages = typeof update === "function" ? update(currentImages) : update;
          return { ...current, [activeProvider]: nextImages };
        });
      },
      [activeProvider],
    );

  const addReferenceImagePaths = useCallback(
    async (paths: string[]) => {
      if (!settings) {
        return 0;
      }
      const provider = settings.defaultProvider;
      const maxReferenceImages = maxReferenceImagesForSettings(settings);
      const currentImages = referenceImagesByProvider[provider];
      const remaining = maxReferenceImages - currentImages.length;
      const nextPaths = paths.filter(isSupportedImagePath).slice(0, Math.max(0, remaining));
      if (nextPaths.length === 0) {
        return 0;
      }
      const nextImages = await Promise.all(nextPaths.map(pathToReferenceImage));
      setReferenceImagesByProvider((current) => ({
        ...current,
        [provider]: [...current[provider], ...nextImages].slice(0, maxReferenceImages),
      }));
      return nextImages.length;
    },
    [referenceImagesByProvider, settings],
  );

  const applyImportedOptions = useCallback(
    (provider: ImageProviderId, options: Partial<GenerationOptionState>) => {
      setOptionsByProvider((current) => ({
        ...current,
        [provider]: {
          ...current[provider],
          ...options,
        },
      }));
    },
    [],
  );

  useEffect(() => {
    queueRef.current = queuedTasks;
  }, [queuedTasks]);

  const runGeneration = useCallback(async () => {
    if (!settings) {
      return;
    }
    const currentPrompt = await getPrompt();
    const promptSnapshot = getPromptSnapshot();
    const taskId = crypto.randomUUID();
    const request: GenerationRequest = {
      taskId,
      provider: settings.defaultProvider,
      model: settings.defaultModel,
      prompt: currentPrompt,
      promptSnapshot,
      batchCount: activeOptions.batchCount,
      referenceImages,
      outputTemplate: settings.outputTemplate,
      baseUrl: baseUrlForSettings(settings),
      options: {
        aspectRatio: activeOptions.aspectRatio,
        imageSize: activeOptions.imageSize,
        temperature: activeOptions.temperature,
        topP: activeOptions.topP,
        thinkingLevel: activeOptions.thinkingLevel,
        quality: activeOptions.quality,
        unlimited: shouldRequestUnlimited(settings, activeOptions),
      },
    };

    await saveCurrentPrompt(promptSnapshot);
    setQueuedTasks((current) => [...current, { id: taskId, request, settings }]);
    setStatus("running");
    setMessage(runningTask ? "Task queued" : "Generating images");
  }, [
    activeOptions,
    getPrompt,
    getPromptSnapshot,
    referenceImages,
    runningTask,
    setMessage,
    setStatus,
    settings,
  ]);

  const stopGeneration = useCallback(async () => {
    if (!runningTask) {
      return;
    }
    setMessage("Stopping current task");
    await cancelGenerationTask(runningTask.id).catch((error) => {
      setStatus("error");
      setMessage(String(error));
    });
  }, [runningTask, setMessage, setStatus]);

  const executeTask = useCallback(
    async (task: QueuedGenerationTask) => {
      setStatus("running");
      setMessage("Generating images");

      try {
        await saveAppSettings(task.settings);
        const batch = await generateImages(task.request);
        setBatches((current) => [batch, ...current.filter((item) => item.id !== batch.id)]);
        if (batch.status === "completed") {
          setPreviewBatchId(batch.id);
          setMessage(`Completed ${batch.images.length} image${batch.images.length === 1 ? "" : "s"}`);
        } else if (batch.status === "cancelled") {
          setMessage("Task cancelled");
        }
        setExpandedBatchId(null);
        setStatus("ready");
      } catch (error) {
        setStatus("error");
        setMessage(String(error));
        await loadAppState()
          .then((state) => setBatches(state.batches))
          .catch(() => undefined);
      }
    },
    [
      setBatches,
      setExpandedBatchId,
      setMessage,
      setPreviewBatchId,
      setStatus,
    ],
  );

  useEffect(() => {
    if (processingRef.current || runningTask || queuedTasks.length === 0) {
      return;
    }

    const [nextTask, ...remaining] = queuedTasks;
    processingRef.current = true;
    setQueuedTasks(remaining);
    setRunningTask(nextTask);

    void executeTask(nextTask).finally(() => {
      processingRef.current = false;
      setRunningTask(null);
    });
  }, [executeTask, queuedTasks, runningTask]);

  return {
    aspectRatio: activeOptions.aspectRatio,
    batchCount: activeOptions.batchCount,
    imageSize: activeOptions.imageSize,
    quality: activeOptions.quality,
    queuedCount: queuedTasks.length,
    runningTask,
    temperature: activeOptions.temperature,
    thinkingLevel: activeOptions.thinkingLevel,
    topP: activeOptions.topP,
    unlimited: activeOptions.unlimited,
    runGeneration,
    stopGeneration,
    addReferenceImagePaths,
    applyImportedOptions,
    referenceImages,
    setAspectRatio,
    setBatchCount,
    setImageSize,
    setQuality,
    setReferenceImages,
    setTemperature,
    setThinkingLevel,
    setTopP,
    setUnlimited,
  };
}

function maxReferenceImagesForSettings(settings: AppSettings): number {
  return getProviderControlConfig(
    settings.defaultProvider,
    settings.defaultModel,
    settingsPlatformForProvider(settings, settings.defaultProvider),
  ).maxReferenceImages;
}

function baseUrlForSettings(settings: AppSettings) {
  if (settings.defaultProvider === "gpt-image") {
    if (settings.openaiApiPlatform === "higgsfield") {
      return null;
    }
    return settings.openaiApiPlatform === "openrouter"
      ? settings.openrouterBaseUrl
      : settings.openaiBaseUrl;
  }
  if (settings.defaultProvider === "grok-imagine") {
    return settings.grokApiPlatform === "higgsfield" ? null : settings.xaiBaseUrl;
  }
  return settings.nanoBananaApiPlatform === "higgsfield" ? null : settings.optionalBaseUrl;
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

function shouldRequestUnlimited(settings: AppSettings, options: GenerationOptionState) {
  return (
    settings.defaultProvider === "nano-banana" &&
    settings.nanoBananaApiPlatform === "higgsfield" &&
    settings.defaultModel === "nano_banana_2" &&
    options.imageSize.toLowerCase() !== "4k" &&
    options.unlimited
  );
}
