import { useCallback, useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { saveCurrentPrompt } from "../api";
import {
  VIDEO_PROVIDER_IDS,
  getVideoDurations,
  getVideoModelConfig,
  getVideoProviderConfig,
  nearestVideoDuration,
} from "../models/videoProviders";
import type {
  AppSettings,
  ReferenceImageInput,
  VideoGenerationRequest,
  VideoInputImageRole,
  VideoInputMode,
  VideoProviderId,
} from "../types";
import { isSupportedImagePath, pathToReferenceImage } from "../utils/referenceImages";
import type { EnqueueGenerationTask } from "./useGenerationQueue";

type ProviderState = {
  aspectRatio: string;
  duration: number;
  generateAudio: boolean;
  inputImages: ReferenceImageInput[];
  inputImageRoles: Record<string, VideoInputImageRole>;
  model: string;
  resolution: string;
};

function initialProviderStates(): Record<VideoProviderId, ProviderState> {
  return VIDEO_PROVIDER_IDS.reduce<Record<VideoProviderId, ProviderState>>(
    (states, provider) => {
      const config = getVideoProviderConfig(provider);
      const model = config.models[0];
      states[provider] = {
        aspectRatio: config.defaultAspectRatio,
        duration: model.defaultDuration,
        generateAudio: provider === "seedance",
        inputImages: [],
        inputImageRoles: {},
        model: model.id,
        resolution: model.defaultResolution,
      };
      return states;
    },
    {} as Record<VideoProviderId, ProviderState>,
  );
}

export function videoInputMode(
  images: ReferenceImageInput[],
  roles: Record<string, VideoInputImageRole>,
): VideoInputMode {
  if (images.length === 0) {
    return "text";
  }
  if (images.some((image) => roles[image.id] === "ending")) {
    return "frames";
  }
  if (images.some((image) => roles[image.id] === "starting")) {
    return "image";
  }
  return "reference";
}

function supportsEndFrame(provider: VideoProviderId) {
  return provider !== "grok-imagine";
}

export function reconcileInputImageRoles(
  provider: VideoProviderId,
  previousImages: ReferenceImageInput[],
  nextImages: ReferenceImageInput[],
  previousRoles: Record<string, VideoInputImageRole>,
): Record<string, VideoInputImageRole> {
  const nextIds = new Set(nextImages.map((image) => image.id));
  const roles = Object.fromEntries(
    nextImages.map((image) => [image.id, previousRoles[image.id] ?? "reference"]),
  ) as Record<string, VideoInputImageRole>;
  const previousMode = videoInputMode(previousImages, previousRoles);

  if (nextImages.some((image) => image.assetId)) {
    return Object.fromEntries(
      nextImages.map((image) => [image.id, "reference"]),
    ) as Record<string, VideoInputImageRole>;
  }

  if (nextImages.length === 1 && roles[nextImages[0].id] === "ending") {
    roles[nextImages[0].id] = "starting";
  }
  if (nextImages.length === 2 && previousImages.length === 1 && previousMode === "image") {
    const previousImage = previousImages[0];
    const addedImage = nextImages.find((image) => !previousImages.some((item) => item.id === image.id));
    if (supportsEndFrame(provider) && nextIds.has(previousImage.id) && addedImage) {
      roles[previousImage.id] = "starting";
      roles[addedImage.id] = "ending";
    } else {
      for (const image of nextImages) {
        roles[image.id] = "reference";
      }
    }
  }
  if (
    nextImages.length > 2
    || (provider === "grok-imagine" && nextImages.length > 1)
  ) {
    for (const image of nextImages) {
      roles[image.id] = "reference";
    }
  }
  return roles;
}

export function updateInputImageRole(
  provider: VideoProviderId,
  images: ReferenceImageInput[],
  roles: Record<string, VideoInputImageRole>,
  imageId: string,
  role: VideoInputImageRole,
): Record<string, VideoInputImageRole> {
  if (!images.some((image) => image.id === imageId)) {
    return roles;
  }
  if (role === "reference") {
    return Object.fromEntries(
      images.map((image) => [image.id, "reference"]),
    ) as Record<string, VideoInputImageRole>;
  }
  if (images.some((image) => image.assetId)) {
    return roles;
  }
  if (role === "starting") {
    if (images.length > 2 || (provider === "grok-imagine" && images.length > 1)) {
      return roles;
    }
    const nextRoles = { ...roles, [imageId]: "starting" as const };
    if (images.length === 2 && supportsEndFrame(provider)) {
      const other = images.find((image) => image.id !== imageId);
      if (other) {
        nextRoles[other.id] = "ending";
      }
    }
    return nextRoles;
  }
  if (!supportsEndFrame(provider) || images.length !== 2) {
    return roles;
  }
  const other = images.find((image) => image.id !== imageId);
  return {
    ...roles,
    [imageId]: "ending",
    ...(other ? { [other.id]: "starting" as const } : {}),
  };
}

export function useVideoGeneration({
  enqueueTask,
  getPrompt,
  getPromptSnapshot,
  settings,
}: {
  enqueueTask: EnqueueGenerationTask;
  getPrompt: () => Promise<string> | string;
  getPromptSnapshot: () => string;
  settings: AppSettings | null;
}) {
  const [provider, setProvider] = useState<VideoProviderId>("grok-imagine");
  const [providerStates, setProviderStates] = useState(initialProviderStates);
  const providerConfig = getVideoProviderConfig(provider);
  const providerState = providerStates[provider];
  const modelConfig = getVideoModelConfig(provider, providerState.model);
  const inputMode = videoInputMode(providerState.inputImages, providerState.inputImageRoles);
  const allowedDurations = useMemo(
    () => getVideoDurations(provider, modelConfig.id, inputMode, providerState.resolution),
    [inputMode, modelConfig.id, provider, providerState.resolution],
  );
  const duration = nearestVideoDuration(providerState.duration, allowedDurations);
  const updateProviderState = useCallback(
    (update: Partial<ProviderState> | ((current: ProviderState) => Partial<ProviderState>)) => {
      setProviderStates((current) => {
        const active = current[provider];
        const patch = typeof update === "function" ? update(active) : update;
        return { ...current, [provider]: { ...active, ...patch } };
      });
    },
    [provider],
  );

  useEffect(() => {
    const defaultModel = settings?.seedanceDefaultModel;
    if (!defaultModel) {
      return;
    }
    const model = getVideoModelConfig("seedance", defaultModel);
    setProviderStates((current) => {
      const seedance = current.seedance;
      if (seedance.model === model.id) {
        return current;
      }
      return {
        ...current,
        seedance: {
          ...seedance,
          model: model.id,
          resolution: model.resolutions.includes(seedance.resolution)
            ? seedance.resolution
            : model.defaultResolution,
        },
      };
    });
  }, [settings?.seedanceDefaultModel]);

  useEffect(() => {
    if (providerState.duration !== duration) {
      updateProviderState({ duration });
    }
  }, [duration, providerState.duration, updateProviderState]);

  const setInputImages: Dispatch<SetStateAction<ReferenceImageInput[]>> = useCallback(
    (next) => updateProviderState((current) => {
      const inputImages = typeof next === "function" ? next(current.inputImages) : next;
      return {
        inputImages,
        inputImageRoles: reconcileInputImageRoles(
          provider,
          current.inputImages,
          inputImages,
          current.inputImageRoles,
        ),
      };
    }),
    [provider, updateProviderState],
  );

  const setInputImageRole = useCallback((imageId: string, role: VideoInputImageRole) => {
    updateProviderState((current) => ({
      inputImageRoles: updateInputImageRole(
        provider,
        current.inputImages,
        current.inputImageRoles,
        imageId,
        role,
      ),
    }));
  }, [provider, updateProviderState]);

  const addInputImagePaths = useCallback(
    async (paths: string[]) => {
      const supportedPaths = paths.filter(isSupportedImagePath);
      if (supportedPaths.length === 0) {
        return 0;
      }

      const remaining = providerConfig.maxInputImages - providerState.inputImages.length;
      if (remaining <= 0) {
        return 0;
      }
      const nextImages = await Promise.all(
        supportedPaths.slice(0, remaining).map(pathToReferenceImage),
      );
      setInputImages((current) =>
        [...current, ...nextImages].slice(0, providerConfig.maxInputImages),
      );
      return nextImages.length;
    },
    [providerConfig.maxInputImages, providerState.inputImages.length, setInputImages],
  );

  const runGeneration = useCallback(async () => {
    if (!settings) {
      return;
    }
    const prompt = await getPrompt();
    const promptSnapshot = getPromptSnapshot();
    const taskId = crypto.randomUUID();
    const startingImage = providerState.inputImages.find(
      (image) => providerState.inputImageRoles[image.id] === "starting",
    );
    const endingImage = providerState.inputImages.find(
      (image) => providerState.inputImageRoles[image.id] === "ending",
    );
    const request = buildVideoGenerationRequest({
      aspectRatio: providerState.aspectRatio,
      duration,
      generateAudio: providerState.generateAudio,
      inputMode,
      model: modelConfig.id,
      prompt,
      promptSnapshot,
      provider,
      referenceImages: inputMode === "reference" ? providerState.inputImages : [],
      resolution: providerState.resolution,
      startingImage,
      endingImage,
      taskId,
    });

    await saveCurrentPrompt(promptSnapshot);
    enqueueTask({ id: taskId, mediaType: "video", request, settings });
  }, [
    duration,
    enqueueTask,
    getPrompt,
    getPromptSnapshot,
    inputMode,
    provider,
    modelConfig.id,
    providerState,
    settings,
  ]);

  return {
    addInputImagePaths,
    allowedDurations,
    aspectRatio: providerState.aspectRatio,
    duration,
    generateAudio: providerState.generateAudio,
    inputImages: providerState.inputImages,
    inputImageRoles: providerState.inputImageRoles,
    inputMode,
    model: modelConfig.id,
    modelConfig,
    provider,
    providerConfig,
    providerDisplayName:
      provider === "seedance" && settings?.seedanceApiPlatform === "higgsfield"
        ? "Higgsfield CLI"
        : providerConfig.platformLabel,
    modelDisplayName: modelConfig.productName,
    resolution: providerState.resolution,
    runGeneration,
    stopAvailable: !(
      provider === "seedance" && settings?.seedanceApiPlatform === "higgsfield"
    ),
    setAspectRatio: (value: string) => updateProviderState({ aspectRatio: value }),
    setDuration: (value: number) => updateProviderState({ duration: value }),
    setGenerateAudio: (value: boolean) => updateProviderState({ generateAudio: value }),
    setInputImages,
    setInputImageRole,
    setModel: (value: string) => {
      const nextModel = getVideoModelConfig(provider, value);
      updateProviderState((current) => ({
        model: nextModel.id,
        resolution: nextModel.resolutions.includes(current.resolution)
          ? current.resolution
          : nextModel.defaultResolution,
      }));
    },
    setProvider,
    setResolution: (value: string) => updateProviderState({ resolution: value }),
  };
}

export function buildVideoGenerationRequest({
  aspectRatio,
  duration,
  generateAudio,
  inputMode,
  model,
  prompt,
  promptSnapshot,
  provider,
  referenceImages,
  resolution,
  startingImage,
  endingImage,
  taskId,
}: {
  aspectRatio: string;
  duration: number;
  generateAudio: boolean;
  inputMode: VideoInputMode;
  model: string;
  prompt: string;
  promptSnapshot: string;
  provider: VideoProviderId;
  referenceImages: ReferenceImageInput[];
  resolution: string;
  startingImage?: ReferenceImageInput;
  endingImage?: ReferenceImageInput;
  taskId: string;
}): VideoGenerationRequest {
  return {
    taskId,
    provider,
    model,
    prompt,
    promptSnapshot,
    inputMode,
    ...((inputMode === "image" || inputMode === "frames") && startingImage
      ? {
          startingImage: {
            name: startingImage.name,
            mimeType: startingImage.mimeType,
            data: startingImage.data,
            ...(startingImage.assetId ? { assetId: startingImage.assetId } : {}),
          },
        }
      : {}),
    ...(inputMode === "frames" && endingImage
      ? {
          endingImage: {
            name: endingImage.name,
            mimeType: endingImage.mimeType,
            data: endingImage.data,
            ...(endingImage.assetId ? { assetId: endingImage.assetId } : {}),
          },
        }
      : {}),
    ...(inputMode === "reference" && referenceImages.length > 0
      ? {
          referenceImages: referenceImages.map((image) => ({
            name: image.name,
            mimeType: image.mimeType,
            data: image.data,
            ...(image.assetId ? { assetId: image.assetId } : {}),
          })),
        }
      : {}),
    options: {
      duration,
      aspectRatio,
      resolution,
      ...(provider === "seedance" ? { generateAudio } : {}),
    },
  };
}
