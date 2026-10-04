import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  cancelGenerationTask,
  generateImages,
  generateVideo,
  loadAppState,
  resumeVideo,
  saveAppSettings,
} from "../api";
import type { AppSettings, GenerationBatch } from "../types";
import { canResumeVideoBatch } from "../utils/history";
import { canStopTask, useGenerationQueue } from "./useGenerationQueue";
import { useInterruptedVideoResume } from "./useInterruptedVideoResume";

vi.mock("../api", () => ({
  cancelGenerationTask: vi.fn(),
  generateImages: vi.fn(),
  generateVideo: vi.fn(),
  loadAppState: vi.fn(),
  resumeVideo: vi.fn(),
  saveAppSettings: vi.fn(),
}));

const settings: AppSettings = {
  defaultProvider: "grok-imagine",
  defaultModel: "grok-imagine-image-quality",
  outputDirectory: "/tmp/sozocraft",
  outputTemplate: "{provider}_{model}_{id}.{extension}",
  higgsfieldOutputEnabled: false,
  higgsfieldOutputDirectory: "/tmp/higgsfield",
  higgsfieldOutputTemplate: "{yyMMdd} {id:3} {higgsfield_filename}",
  promptDirectory: "/tmp/prompts",
  promptDslEnabled: true,
  promptEditorOnly: false,
  promptPreviewPlacement: "bottom",
  nanoBananaApiPlatform: "gemini",
  geminiProxyEnabled: false,
  openaiApiPlatform: "openai",
  openaiProxyEnabled: false,
  grokApiPlatform: "xai",
  xaiProxyEnabled: false,
  seedanceApiPlatform: "ark",
  seedanceDefaultModel: "doubao-seedance-2-0-260128",
  arkProxyEnabled: false,
  higgsfieldProxyEnabled: true,
  higgsfieldProxyUrl: null,
  timeoutSeconds: 180,
  geminiTimeoutSeconds: 180,
  openaiTimeoutSeconds: 180,
  xaiTimeoutSeconds: 180,
  openrouterProxyEnabled: true,
  openrouterTimeoutSeconds: 180,
  arkTimeoutSeconds: 180,
};

const videoBatch: GenerationBatch = {
  id: "batch-video",
  mediaType: "video",
  provider: "grok-imagine",
  model: "grok-imagine-video",
  promptSnapshot: "Source prompt",
  status: "completed",
  images: [],
  videos: [],
  createdAt: "2026-08-01T00:00:00Z",
};

describe("shared generation queue", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(saveAppSettings).mockResolvedValue({ settings, currentPrompt: "", batches: [] });
    vi.mocked(generateVideo).mockResolvedValue(videoBatch);
  });

  it("executes video tasks through the shared queue and selects the completed batch", async () => {
    const setBatches = vi.fn();
    const setExpandedBatchId = vi.fn();
    const setMessage = vi.fn();
    const setPreviewBatchId = vi.fn();
    const setStatus = vi.fn();
    const { result } = renderHook(() =>
      useGenerationQueue({
        setBatches,
        setExpandedBatchId,
        setMessage,
        setPreviewBatchId,
        setStatus,
      }),
    );

    act(() => {
      result.current.enqueueTask({
        id: "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09",
        mediaType: "video",
        settings,
        request: {
          taskId: "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09",
          provider: "grok-imagine",
          model: "grok-imagine-video",
          prompt: "Rendered prompt",
          promptSnapshot: "Source prompt",
          inputMode: "text",
          options: { duration: 5, aspectRatio: "16:9", resolution: "480p" },
        },
      });
    });

    await waitFor(() => expect(generateVideo).toHaveBeenCalledOnce());
    expect(generateImages).not.toHaveBeenCalled();
    expect(saveAppSettings).toHaveBeenCalledWith(settings);
    await waitFor(() => expect(setPreviewBatchId).toHaveBeenCalledWith("batch-video"));
    expect(setMessage).toHaveBeenCalledWith("Completed video");

    const update = setBatches.mock.calls.find(([value]) => typeof value === "function")?.[0];
    expect(update?.([])).toEqual([videoBatch]);
  });

  it("disables Stop for every Higgsfield-backed provider", () => {
    const request = {
      taskId: "72c61fd4-5f57-4d0f-935f-a63bf45208e2",
      model: "test-model",
      prompt: "Prompt",
      batchCount: 1,
      outputTemplate: "{id}.{extension}",
      options: {},
    };
    expect(canStopTask({
      id: "nano",
      mediaType: "image",
      settings: { ...settings, nanoBananaApiPlatform: "higgsfield" },
      request: { ...request, provider: "nano-banana" },
    })).toBe(false);
    expect(canStopTask({
      id: "gpt",
      mediaType: "image",
      settings: { ...settings, openaiApiPlatform: "higgsfield" },
      request: { ...request, provider: "gpt-image" },
    })).toBe(false);
    expect(canStopTask({
      id: "grok",
      mediaType: "image",
      settings: { ...settings, grokApiPlatform: "higgsfield" },
      request: { ...request, provider: "grok-imagine" },
    })).toBe(false);
    expect(canStopTask({
      id: "experimental",
      mediaType: "image",
      settings: { ...settings, grokApiPlatform: "higgsfield", openaiApiPlatform: "higgsfield" },
      request: { ...request, provider: "experimental", model: "meta/muse-image" },
    })).toBe(true);
    expect(canStopTask({
      id: "direct",
      mediaType: "image",
      settings,
      request: { ...request, provider: "grok-imagine" },
    })).toBe(true);
  });

  it("routes Stop to the active task id", async () => {
    let resolveVideo: ((batch: GenerationBatch) => void) | undefined;
    vi.mocked(generateVideo).mockImplementation(
      () => new Promise((resolve) => { resolveVideo = resolve; }),
    );
    vi.mocked(cancelGenerationTask).mockResolvedValue(true);
    vi.mocked(loadAppState).mockResolvedValue({ settings, currentPrompt: "", batches: [] });
    const { result } = renderHook(() =>
      useGenerationQueue({
        setBatches: vi.fn(),
        setExpandedBatchId: vi.fn(),
        setMessage: vi.fn(),
        setPreviewBatchId: vi.fn(),
        setStatus: vi.fn(),
      }),
    );
    const taskId = "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09";

    act(() => {
      result.current.enqueueTask({
        id: taskId,
        mediaType: "video",
        settings,
        request: {
          taskId,
          provider: "grok-imagine",
          model: "grok-imagine-video",
          prompt: "Prompt",
          inputMode: "text",
          options: { duration: 5, aspectRatio: "16:9", resolution: "480p" },
        },
      });
    });
    await waitFor(() => expect(result.current.runningTask?.id).toBe(taskId));
    await act(async () => result.current.stopGeneration());
    expect(cancelGenerationTask).toHaveBeenCalledWith(taskId);

    await act(async () => resolveVideo?.(videoBatch));
  });

  it("disables Stop for Higgsfield Seedance video tasks", async () => {
    let resolveVideo: ((batch: GenerationBatch) => void) | undefined;
    vi.mocked(generateVideo).mockImplementation(
      () => new Promise((resolve) => { resolveVideo = resolve; }),
    );
    const { result } = renderHook(() =>
      useGenerationQueue({
        setBatches: vi.fn(),
        setExpandedBatchId: vi.fn(),
        setMessage: vi.fn(),
        setPreviewBatchId: vi.fn(),
        setStatus: vi.fn(),
      }),
    );
    const taskId = "5f1ba084-00f8-495e-af44-c9017168b371";

    act(() => {
      result.current.enqueueTask({
        id: taskId,
        mediaType: "video",
        settings: { ...settings, seedanceApiPlatform: "higgsfield" },
        request: {
          taskId,
          provider: "seedance",
          model: "doubao-seedance-2-0-260128",
          prompt: "Prompt",
          inputMode: "text",
          options: { duration: 5, aspectRatio: "16:9", resolution: "480p" },
        },
      });
    });
    await waitFor(() => expect(result.current.runningTask?.id).toBe(taskId));
    expect(result.current.canStopRunningTask).toBe(false);
    await act(async () => result.current.stopGeneration());
    expect(cancelGenerationTask).not.toHaveBeenCalled();

    await act(async () => resolveVideo?.(videoBatch));
  });

  it("resumes a stored video job once without resubmitting or saving settings", async () => {
    let resolveResume: ((batch: GenerationBatch) => void) | undefined;
    vi.mocked(resumeVideo).mockImplementation(
      () => new Promise((resolve) => { resolveResume = resolve; }),
    );
    const setBatches = vi.fn();
    const { result } = renderHook(() =>
      useGenerationQueue({
        setBatches,
        setExpandedBatchId: vi.fn(),
        setMessage: vi.fn(),
        setPreviewBatchId: vi.fn(),
        setStatus: vi.fn(),
      }),
    );

    act(() => result.current.resumeVideoBatch("batch-video"));
    await waitFor(() => expect(resumeVideo).toHaveBeenCalledOnce());
    act(() => result.current.resumeVideoBatch("batch-video"));

    const taskId = vi.mocked(resumeVideo).mock.calls[0][0];
    expect(vi.mocked(resumeVideo).mock.calls[0][1]).toBe("batch-video");
    expect(result.current.runningTask?.id).toBe(taskId);
    expect(result.current.resumingBatchIds.has("batch-video")).toBe(true);
    expect(result.current.canStopRunningTask).toBe(false);
    expect(result.current.queuedCount).toBe(0);
    expect(generateVideo).not.toHaveBeenCalled();
    expect(saveAppSettings).not.toHaveBeenCalled();

    await act(async () => resolveResume?.(videoBatch));
    await waitFor(() => expect(result.current.runningTask).toBeNull());
    expect(resumeVideo).toHaveBeenCalledOnce();
  });

  it("auto-resumes only interrupted Higgsfield jobs after history loads", () => {
    const higgsfieldJob = { provider: "seedance", platform: "higgsfield" };
    const running: GenerationBatch = {
      ...videoBatch,
      id: "running",
      status: "running",
      providerRequestId: "3f2b8c1e-7a4d-4e9b-9c2a-1d5e6f7a8b9c",
      videoJob: higgsfieldJob,
    };
    const failed: GenerationBatch = { ...running, id: "failed", status: "failed" };
    const ark: GenerationBatch = {
      ...running,
      id: "ark",
      videoJob: { provider: "seedance", platform: "volcengine-ark" },
    };
    const unsubmitted: GenerationBatch = { ...running, id: "unsubmitted", providerRequestId: null };
    expect(canResumeVideoBatch(failed)).toBe(true);
    expect(canResumeVideoBatch({ ...running, status: "completed" })).toBe(false);
    expect(canResumeVideoBatch(ark)).toBe(false);
    expect(canResumeVideoBatch(unsubmitted)).toBe(false);

    const resumeVideoBatch = vi.fn();
    const batches = [running, failed, ark, unsubmitted];
    const { rerender } = renderHook(
      ({ loaded }) => useInterruptedVideoResume({ batches, loaded, resumeVideoBatch }),
      { initialProps: { loaded: false } },
    );
    expect(resumeVideoBatch).not.toHaveBeenCalled();
    rerender({ loaded: true });
    rerender({ loaded: true });
    expect(resumeVideoBatch).toHaveBeenCalledTimes(1);
    expect(resumeVideoBatch).toHaveBeenCalledWith("running");
  });
});
