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

type QueuedGenerationTask = {
  id: string;
  request: GenerationRequest;
  settings: AppSettings;
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
  getPrompt: () => string;
  getPromptSnapshot: () => string;
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
  const [queuedTasks, setQueuedTasks] = useState<QueuedGenerationTask[]>([]);
  const [runningTask, setRunningTask] = useState<QueuedGenerationTask | null>(null);
  const queueRef = useRef<QueuedGenerationTask[]>([]);
  const processingRef = useRef(false);

  useEffect(() => {
    queueRef.current = queuedTasks;
  }, [queuedTasks]);

  const runGeneration = useCallback(async () => {
    if (!settings) {
      return;
    }
    const currentPrompt = getPrompt();
    const promptSnapshot = getPromptSnapshot();
    const taskId = crypto.randomUUID();
    const request: GenerationRequest = {
      taskId,
      provider: settings.defaultProvider,
      model: settings.defaultModel,
      prompt: currentPrompt,
      promptSnapshot,
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
    };

    await saveCurrentPrompt(promptSnapshot);
    setQueuedTasks((current) => [...current, { id: taskId, request, settings }]);
    setStatus("running");
    setMessage(runningTask ? "Task queued" : "Generating images");
  }, [
    aspectRatio,
    batchCount,
    getPrompt,
    getPromptSnapshot,
    imageSize,
    quality,
    referenceImages,
    runningTask,
    setMessage,
    setStatus,
    settings,
    temperature,
    thinkingLevel,
    topP,
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
    aspectRatio,
    batchCount,
    imageSize,
    quality,
    queuedCount: queuedTasks.length,
    runningTask,
    temperature,
    thinkingLevel,
    topP,
    runGeneration,
    stopGeneration,
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
