import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelGenerationTask,
  generateImages,
  generateVideo,
  loadAppState,
  resumeVideo,
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

/** Resumes monitoring a submitted Higgsfield video job; never resubmits it. */
type QueuedVideoResumeTask = {
  id: string;
  mediaType: "video";
  resumeBatchId: string;
};

export type QueuedGenerationTask = QueuedImageTask | QueuedVideoTask | QueuedVideoResumeTask;
export type EnqueueGenerationTask = (task: QueuedGenerationTask) => void;

export function usesHiggsfieldTask(task: QueuedGenerationTask) {
  if ("resumeBatchId" in task) {
    return true;
  }
  if (task.mediaType === "video") {
    return task.request.provider === "seedance"
      && task.settings.seedanceApiPlatform === "higgsfield";
  }
  if (task.request.provider === "nano-banana") {
    return task.settings.nanoBananaApiPlatform === "higgsfield";
  }
  if (task.request.provider === "gpt-image") {
    return task.settings.openaiApiPlatform === "higgsfield";
  }
  return task.settings.grokApiPlatform === "higgsfield";
}

export function canStopTask(task: QueuedGenerationTask | null) {
  return Boolean(task && !usesHiggsfieldTask(task));
}

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
  const canStopRunningTask = canStopTask(runningTask);
  // A fresh video submission persists its running batch before it finishes, so
  // history can show it as running while this process is still monitoring it.
  const monitoringNewVideo = Boolean(
    runningTask && runningTask.mediaType === "video" && !("resumeBatchId" in runningTask),
  );
  const resumingBatchIds = new Set(
    [runningTask, ...queuedTasks].flatMap((task) =>
      task && "resumeBatchId" in task ? [task.resumeBatchId] : [],
    ),
  );

  const enqueueTask = useCallback<EnqueueGenerationTask>(
    (task) => {
      setQueuedTasks((current) => [...current, task]);
      setStatus("running");
      setMessage(runningTask ? "Task queued" : runningMessage(task.mediaType));
    },
    [runningTask, setMessage, setStatus],
  );

  const resumeVideoBatch = useCallback(
    (batchId: string) => {
      const alreadyQueued = (task: QueuedGenerationTask | null) =>
        Boolean(task && "resumeBatchId" in task && task.resumeBatchId === batchId);
      setQueuedTasks((current) => {
        if (alreadyQueued(runningTask) || current.some(alreadyQueued)) {
          return current;
        }
        return [...current, { id: crypto.randomUUID(), mediaType: "video", resumeBatchId: batchId }];
      });
      setStatus("running");
      setMessage(runningTask ? "Task queued" : "Resuming video");
    },
    [runningTask, setMessage, setStatus],
  );

  const stopGeneration = useCallback(async () => {
    if (!runningTask || !canStopTask(runningTask)) {
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
      const resumeBatchId = "resumeBatchId" in task ? task.resumeBatchId : null;
      setMessage(resumeBatchId ? "Resuming video" : runningMessage(task.mediaType));
      if (resumeBatchId) {
        setBatches((current) =>
          current.map((item) =>
            item.id === resumeBatchId ? { ...item, status: "running", error: null } : item,
          ),
        );
      }

      try {
        let batch: GenerationBatch;
        if ("resumeBatchId" in task) {
          batch = await resumeVideo(task.id, task.resumeBatchId);
        } else {
          await saveAppSettings(task.settings);
          batch =
            task.mediaType === "image"
              ? await generateImages(task.request)
              : await generateVideo(task.request);
        }
        setBatches((current) => [batch, ...current.filter((item) => item.id !== batch.id)]);
        if (batch.status === "completed") {
          setPreviewBatchId(batch.id);
          setMessage(completedMessage(batch));
        } else if (batch.status === "cancelled") {
          setMessage(
            batch.mediaType === "video"
              ? "Stopped monitoring; the provider may still complete the job"
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
    canStopRunningTask,
    queuedCount: queuedTasks.length,
    monitoringNewVideo,
    resumeVideoBatch,
    resumingBatchIds,
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
