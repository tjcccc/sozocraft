import { Loader2, Play, Settings, Square } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { exportRenderedPrompt } from "./api";
import { GenerationPanel } from "./components/GenerationPanel";
import { ImageLightbox, type LightboxImage, type LightboxState } from "./components/ImageLightbox";
import { OutputColumn } from "./components/OutputColumn";
import { PromptColumn } from "./components/PromptColumn";
import { SettingsPanel } from "./components/SettingsPanel";
import {
  ColumnResizer,
  ModeSwitch,
  StatusPill,
  type GenerationMode,
} from "./components/common";
import { useAppState } from "./hooks/useAppState";
import { useGeneration } from "./hooks/useGeneration";
import { useHistoryDate } from "./hooks/useHistoryDate";
import { useImagePreviews } from "./hooks/useImagePreviews";
import { useModelOptions } from "./hooks/useModelOptions";
import { usePromptLibrary } from "./hooks/usePromptLibrary";
import { getProviderConfig, getProviderModelDisplayName } from "./models/imageProviders";
import { clamp } from "./utils/math";
import type { AppSettings } from "./types";

const MIN_COLUMN_WIDTHS = [24, 24, 28];

export function App() {
  const workspaceRef = useRef<HTMLElement>(null);
  const [mode, setMode] = useState<GenerationMode>("image");
  const [expandedBatchId, setExpandedBatchId] = useState<string | null>(null);
  const [previewBatchId, setPreviewBatchId] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [toolbarHint, setToolbarHint] = useState<string | null>(null);
  const [lightbox, setLightbox] = useState<LightboxState | null>(null);
  const [columnWidths, setColumnWidths] = useState([35.5, 27.2, 37.3]);
  const [resizingDivider, setResizingDivider] = useState<number | null>(null);
  const promptRef = useRef("");

  const {
    apiKey,
    apiKeySaved,
    batches,
    configStatus,
    currentPromptId,
    message,
    openaiApiKey,
    openaiApiKeySaved,
    prompt,
    saveKey,
    saveOpenaiKey,
    saveXaiKey,
    setApiKey,
    setOpenaiApiKey,
    setBatches,
    setCurrentPromptId,
    setMessage,
    setPrompt,
    setSettings,
    setStatus,
    setXaiApiKey,
    settings,
    status,
    updateSettings,
    xaiApiKey,
    xaiApiKeySaved,
  } = useAppState();

  useEffect(() => {
    promptRef.current = prompt;
  }, [prompt]);

  const promptLibrary = usePromptLibrary({
    currentPromptId,
    prompt,
    promptDslEnabled: settings?.promptDslEnabled ?? true,
    setCurrentPromptId,
    setMessage,
    setPrompt,
    setStatus,
    settings,
  });

  const { filteredBatches, historyDate, setHistoryDate } = useHistoryDate(batches);
  useEffect(() => {
    setExpandedBatchId(null);
  }, [historyDate]);

  const expandedBatch = useMemo(
    () => batches.find((batch) => batch.id === expandedBatchId),
    [batches, expandedBatchId],
  );
  const previewBatch = useMemo(
    () => (previewBatchId ? batches.find((batch) => batch.id === previewBatchId) : undefined),
    [batches, previewBatchId],
  );
  const { failedImagePaths, imageDataUrls } = useImagePreviews({
    expandedBatch,
    previewBatch,
  });
  const getCurrentPrompt = useCallback(() => promptLibrary.renderedPrompt, [promptLibrary.renderedPrompt]);
  const getPromptSnapshot = useCallback(() => promptRef.current, []);
  const generation = useGeneration({
    getPrompt: getCurrentPrompt,
    getPromptSnapshot,
    setBatches,
    setExpandedBatchId,
    setMessage,
    setPreviewBatchId,
    setStatus,
    settings,
  });

  useModelOptions({
    aspectRatio: generation.aspectRatio,
    imageSize: generation.imageSize,
    quality: generation.quality,
    settings,
    setAspectRatio: generation.setAspectRatio,
    setImageSize: generation.setImageSize,
    setQuality: generation.setQuality,
    setThinkingLevel: generation.setThinkingLevel,
    thinkingLevel: generation.thinkingLevel,
  });

  const startColumnResize = useCallback(
    (dividerIndex: 0 | 1, event: React.PointerEvent<HTMLDivElement>) => {
      const bounds = workspaceRef.current?.getBoundingClientRect();
      if (!bounds) {
        return;
      }

      event.preventDefault();
      setResizingDivider(dividerIndex);
      document.body.classList.add("is-column-resizing");

      const initialWidths = columnWidths;
      const onMove = (moveEvent: PointerEvent) => {
        const cursorPercent = ((moveEvent.clientX - bounds.left) / bounds.width) * 100;
        const next = [...initialWidths];

        if (dividerIndex === 0) {
          const fixedRight = initialWidths[2];
          const left = clamp(
            cursorPercent,
            MIN_COLUMN_WIDTHS[0],
            100 - fixedRight - MIN_COLUMN_WIDTHS[1],
          );
          next[0] = left;
          next[1] = 100 - fixedRight - left;
        } else {
          const fixedLeft = initialWidths[0];
          const middleRightEdge = clamp(
            cursorPercent,
            fixedLeft + MIN_COLUMN_WIDTHS[1],
            100 - MIN_COLUMN_WIDTHS[2],
          );
          next[1] = middleRightEdge - fixedLeft;
          next[2] = 100 - middleRightEdge;
        }

        setColumnWidths(next);
      };

      const onUp = () => {
        setResizingDivider(null);
        document.body.classList.remove("is-column-resizing");
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
      };

      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp, { once: true });
    },
    [columnWidths],
  );

  const openLightbox = useCallback((images: LightboxImage[], index: number) => {
    if (images.length === 0) {
      return;
    }
    setLightbox({ images, index: clamp(index, 0, images.length - 1) });
  }, []);

  const updatePrompt = useCallback(
    (nextPrompt: string) => {
      promptRef.current = nextPrompt;
      setPrompt(nextPrompt);
    },
    [setPrompt],
  );

  const defaultExportPath = useMemo(() => {
    if (!settings) {
      return "";
    }
    return `${settings.promptDirectory}/exports/${sanitizeExportName(
      promptLibrary.title || "rendered-prompt",
    )}.md`;
  }, [promptLibrary.title, settings]);

  const exportPrompt = useCallback((outputPath: string) => {
    void exportRenderedPrompt(outputPath, promptLibrary.renderedPrompt)
      .then((path) => {
        setStatus("ready");
        setMessage(`Exported ${path}`);
      })
      .catch((error) => {
        setStatus("error");
        setMessage(String(error));
      });
  }, [promptLibrary.renderedPrompt, setMessage, setStatus]);

  const handleRun = useCallback(() => {
    const wasRunning = Boolean(generation.runningTask);
    void generation.runGeneration();
    if (wasRunning) {
      setToolbarHint("Task added to queue");
      window.setTimeout(() => setToolbarHint(null), 1800);
    }
  }, [generation]);

  const handleStop = useCallback(() => {
    if (!generation.runningTask) {
      return;
    }
    const totalActiveTasks = 1 + generation.queuedCount;
    setToolbarHint(`Cancelled task 1/${totalActiveTasks}`);
    window.setTimeout(() => setToolbarHint(null), 1800);
    void generation.stopGeneration();
  }, [generation]);

  const saveSettings = useCallback(
    (nextSettings: AppSettings) => {
      void updateSettings(nextSettings);
    },
    [updateSettings],
  );

  const showEditorOnly = !!settings?.promptEditorOnly && !showSettings;
  const completedTasks = batches.filter((batch) => batch.status === "completed").length;
  const failedTasks = batches.filter((batch) => batch.status === "failed").length;
  const cancelledTasks = batches.filter((batch) => batch.status === "cancelled").length;

  if (!settings) {
    return (
      <main className="loading">
        <Loader2 className="spin" size={22} />
        <span>Loading SōzōCraft</span>
      </main>
    );
  }

  return (
    <main className="app-shell">
      <header className="toolbar">
        <div className="toolbar-left">
          <div className="brand">
            <div className="brand-mark">S</div>
            <strong>SōzōCraft</strong>
          </div>
        </div>
        <div className="toolbar-center">
          {!settings.promptEditorOnly ? (
            <>
              <button className="primary-button" disabled={showSettings} onClick={handleRun}>
                {status === "running" ? <Loader2 className="spin" size={17} /> : <Play size={17} />}
                Run
              </button>
              <button
                className="secondary-button"
                disabled={showSettings || !generation.runningTask}
                onClick={handleStop}
              >
                <Square size={14} />
                Stop
              </button>
              {toolbarHint ? <div className="toolbar-hint">{toolbarHint}</div> : null}
            </>
          ) : null}
        </div>
        <div className="toolbar-right">
          {!settings.promptEditorOnly ? (
            <>
              <ModeSwitch disabled={showSettings} mode={mode} setMode={setMode} />
              <div className="divider" />
            </>
          ) : null}
          <button
            className="icon-button"
            title="Settings"
            onClick={() => setShowSettings((value) => !value)}
          >
            <Settings size={18} />
          </button>
        </div>
      </header>

      {showSettings ? (
        <SettingsPanel
          apiKey={apiKey}
          apiKeySaved={apiKeySaved}
          openaiApiKey={openaiApiKey}
          openaiApiKeySaved={openaiApiKeySaved}
          xaiApiKey={xaiApiKey}
          xaiApiKeySaved={xaiApiKeySaved}
          configStatus={configStatus}
          settings={settings}
          setApiKey={setApiKey}
          setOpenaiApiKey={setOpenaiApiKey}
          setSettings={setSettings}
          setXaiApiKey={setXaiApiKey}
          onSaveKey={() => void saveKey()}
          onSaveOpenaiKey={() => void saveOpenaiKey()}
          onSaveSettings={saveSettings}
          onSaveXaiKey={() => void saveXaiKey()}
        />
      ) : showEditorOnly ? (
        <section className="workspace editor-only-workspace" ref={workspaceRef}>
          <PromptColumn
            items={promptLibrary.filteredItems}
            prompt={prompt}
            query={promptLibrary.query}
            renderedPrompt={promptLibrary.renderedPrompt}
            saveState={promptLibrary.saveState}
            selectedPromptId={promptLibrary.selectedPromptId}
            sortMode={promptLibrary.sortMode}
            tagsText={promptLibrary.tagsText}
            title={promptLibrary.title}
            dslEnabled={settings.promptDslEnabled}
            previewPlacement={settings.promptPreviewPlacement}
            setDslEnabled={(enabled) => {
              const nextSettings = { ...settings, promptDslEnabled: enabled };
              setSettings(nextSettings);
              void updateSettings(nextSettings);
            }}
            setPreviewPlacement={(placement) => {
              const nextSettings = { ...settings, promptPreviewPlacement: placement };
              setSettings(nextSettings);
              void updateSettings(nextSettings);
            }}
            setQuery={promptLibrary.setQuery}
            setSortMode={promptLibrary.setSortMode}
            setTagsText={promptLibrary.setTagsText}
            setTitle={promptLibrary.setTitle}
            onCommitMetadata={() => void promptLibrary.commitMetadata()}
            onCreatePrompt={() => void promptLibrary.createNewPrompt()}
            onDeletePrompt={(id) => void promptLibrary.deletePromptById(id)}
            defaultExportPath={defaultExportPath}
            onExportRenderedPrompt={exportPrompt}
            onPromptChange={updatePrompt}
            onSelectPrompt={(id) => void promptLibrary.selectPrompt(id)}
          />
        </section>
      ) : (
        <section
          className="workspace"
          ref={workspaceRef}
          style={{
            gridTemplateColumns: `${columnWidths[0]}fr 10px ${columnWidths[1]}fr 10px ${columnWidths[2]}fr`,
          }}
        >
          <PromptColumn
            items={promptLibrary.filteredItems}
            prompt={prompt}
            query={promptLibrary.query}
            renderedPrompt={promptLibrary.renderedPrompt}
            saveState={promptLibrary.saveState}
            selectedPromptId={promptLibrary.selectedPromptId}
            sortMode={promptLibrary.sortMode}
            tagsText={promptLibrary.tagsText}
            title={promptLibrary.title}
            dslEnabled={settings.promptDslEnabled}
            previewPlacement={settings.promptPreviewPlacement}
            setDslEnabled={(enabled) => {
              const nextSettings = { ...settings, promptDslEnabled: enabled };
              setSettings(nextSettings);
              void updateSettings(nextSettings);
            }}
            setPreviewPlacement={(placement) => {
              const nextSettings = { ...settings, promptPreviewPlacement: placement };
              setSettings(nextSettings);
              void updateSettings(nextSettings);
            }}
            setQuery={promptLibrary.setQuery}
            setSortMode={promptLibrary.setSortMode}
            setTagsText={promptLibrary.setTagsText}
            setTitle={promptLibrary.setTitle}
            onCommitMetadata={() => void promptLibrary.commitMetadata()}
            onCreatePrompt={() => void promptLibrary.createNewPrompt()}
            onDeletePrompt={(id) => void promptLibrary.deletePromptById(id)}
            defaultExportPath={defaultExportPath}
            onExportRenderedPrompt={exportPrompt}
            onPromptChange={updatePrompt}
            onSelectPrompt={(id) => void promptLibrary.selectPrompt(id)}
          />
          <ColumnResizer
            active={resizingDivider === 0}
            label="Resize prompt and generation columns"
            onPointerDown={(event) => startColumnResize(0, event)}
          />
          <GenerationPanel
            settings={settings}
            setSettings={setSettings}
            onPreviewImages={openLightbox}
            {...generation}
          />
          <ColumnResizer
            active={resizingDivider === 1}
            label="Resize generation and output columns"
            onPointerDown={(event) => startColumnResize(1, event)}
          />
          <OutputColumn
            batches={filteredBatches}
            errorMessage={status === "error" ? message : null}
            expandedBatchId={expandedBatchId}
            failedImagePaths={failedImagePaths}
            historyDate={historyDate}
            imageDataUrls={imageDataUrls}
            previewBatch={previewBatch}
            setExpandedBatchId={setExpandedBatchId}
            setHistoryDate={setHistoryDate}
            onPreviewImages={openLightbox}
          />
        </section>
      )}

      {lightbox ? (
        <ImageLightbox
          state={lightbox}
          onClose={() => setLightbox(null)}
          onIndexChange={(index) => setLightbox((current) => (current ? { ...current, index } : current))}
        />
      ) : null}

      <footer className="statusbar">
        <StatusPill status={status} label={status === "error" ? "Error" : message} />
        {!settings.promptEditorOnly || showSettings ? (
          <>
            <div className="status-info">
              <span>{getProviderConfig(settings.defaultProvider).providerName}</span>
              <span>/</span>
              <span>{getProviderModelDisplayName(settings.defaultProvider, settings.defaultModel)}</span>
            </div>
            <div className="status-counts">
              <span>Queue: {generation.queuedCount}</span>
              <span>Running: {generation.runningTask ? 1 : 0}</span>
              <span>Completed: {completedTasks}</span>
              <span>Failed: {failedTasks}</span>
              <span>Cancelled: {cancelledTasks}</span>
            </div>
          </>
        ) : (
          <>
            <div />
            <div />
          </>
        )}
      </footer>
    </main>
  );
}

function sanitizeExportName(value: string) {
  const safe = value
    .trim()
    .replace(/[\\/:*?"<>|]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/^-+|-+$/g, "");
  return safe || "rendered-prompt";
}
