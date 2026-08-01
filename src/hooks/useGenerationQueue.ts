import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelGenerationTask,
  generateImages,
  generateVideo,
  loadAppState,
  saveAppSettings,
} from "../api";
import type { AppStatus } from "../components/common";
import type {
  AppSettings,
  GenerationBatch,
  GenerationMediaType,
  GenerationRequest,
  VideoGenerationRequest,
} from "../types";

type QueuedImageTask = {
  id: string;
  mediaType: "image";
  request: GenerationRequest;
  settings: AppSettings;
};

type QueuedVideoTask = {
  id: string;
  mediaType: "video";
  request: VideoGenerationRequest;
  settings: AppSettings;
};

export type QueuedGenerationTask = QueuedImageTask | QueuedVideoTask;
export type EnqueueGenerationTask = (task: QueuedGenerationTask) => void;

export function useGenerationQueue({
  setBatches,
  setExpandedBatchId,
  setMessage,
  setPreviewBatchId,
  setStatus,
}: {
  setBatches: React.Dispatch<React.SetStateAction<GenerationBatch[]>>;
  setExpandedBatchId: (id: string | null) => void;
  setMessage: (message: string) => void;
  setPreviewBatchId: (id: string | null) => void;
  setStatus: (status: AppStatus) => void;
}) {
  const [queuedTasks, setQueuedTasks] = useState<QueuedGenerationTask[]>([]);
  const [runningTask, setRunningTask] = useState<QueuedGenerationTask | null>(null);
  const processingRef = useRef(false);

  const enqueueTask = useCallback<EnqueueGenerationTask>(
    (task) => {
      setQueuedTasks((current) => [...current, task]);
      setStatus("running");
      setMessage(runningTask ? "Task queued" : runningMessage(task.mediaType));
    },
    [runningTask, setMessage, setStatus],
  );

  const stopGeneration = useCallback(async () => {
    if (!runningTask) {
      return;
    }
    setMessage(
      runningTask.mediaType === "video"
        ? "Stopping local video monitoring"
        : "Stopping current task",
    );
    await cancelGenerationTask(runningTask.id).catch((error) => {
      setStatus("error");
      setMessage(String(error));
    });
  }, [runningTask, setMessage, setStatus]);

  const executeTask = useCallback(
    async (task: QueuedGenerationTask) => {
      setStatus("running");
      setMessage(runningMessage(task.mediaType));

      try {
        await saveAppSettings(task.settings);
        const batch =
          task.mediaType === "image"
            ? await generateImages(task.request)
            : await generateVideo(task.request);
        setBatches((current) => [batch, ...current.filter((item) => item.id !== batch.id)]);
        if (batch.status === "completed") {
          setPreviewBatchId(batch.id);
          setMessage(completedMessage(batch));
        } else if (batch.status === "cancelled") {
          setMessage(
            batch.mediaType === "video"
              ? "Stopped monitoring; xAI may still complete the job"
              : "Task cancelled",
          );
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
    [setBatches, setExpandedBatchId, setMessage, setPreviewBatchId, setStatus],
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
    enqueueTask,
    queuedCount: queuedTasks.length,
    runningTask,
    stopGeneration,
  };
}

function runningMessage(mediaType: GenerationMediaType) {
  return mediaType === "video" ? "Generating video" : "Generating images";
}

function completedMessage(batch: GenerationBatch) {
  if (batch.mediaType === "video") {
    return "Completed video";
  }
  return `Completed ${batch.images.length} image${batch.images.length === 1 ? "" : "s"}`;
}
