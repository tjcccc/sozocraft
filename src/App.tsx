import { Loader2, Play, Settings, Square } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { exportRenderedPrompt, readImageTextMetadata, readTextFile } from "./api";
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
import {
  getProviderConfig,
  getProviderModelDisplayName,
  isImageProviderId,
  normalizeProviderOptions,
} from "./models/imageProviders";
import type {
  GrokImagineApiPlatform,
  ImageProviderApiPlatform,
  ImageProviderId,
  NanoBananaApiPlatform,
} from "./models/imageProviders";
import { clamp } from "./utils/math";
import { fileNameFromPath, isSupportedImagePath } from "./utils/referenceImages";
import sozocraftIcon from "./assets/sozocraft-icon.png";
import type { AppSettings } from "./types";

const MIN_COLUMN_WIDTHS = [24, 24, 28];
type FileDropZone = "prompt-editor" | "generation" | "output";

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
  const [activeFileDropZone, setActiveFileDropZone] = useState<FileDropZone | null>(null);
  const draggedPromptIncludeRef = useRef<string | null>(null);
  const promptIncludeDropHandledAtRef = useRef(0);
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
    openrouterApiKey,
    openrouterApiKeySaved,
    prompt,
    saveKey,
    saveOpenaiKey,
    saveOpenrouterKey,
    saveXaiKey,
    setApiKey,
    setOpenaiApiKey,
    setOpenrouterApiKey,
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
  const importPromptSource = promptLibrary.importPromptSource;

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
  const { addReferenceImagePaths, applyImportedOptions } = generation;

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

  const markPromptIncludeDropHandled = useCallback(() => {
    promptIncludeDropHandledAtRef.current = Date.now();
  }, []);

  const insertPromptIncludeFromNativeDrop = useCallback(
    (token: string, position: { x: number; y: number }) => {
      const target = promptTextareaFromPosition(position);
      if (!target) {
        return false;
      }
      const index = textareaIndexFromPoint(target.textarea, target.x, target.y);
      const currentPrompt = promptRef.current;
      updatePrompt(`${currentPrompt.slice(0, index)}${token}${currentPrompt.slice(index)}`);
      markPromptIncludeDropHandled();
      window.requestAnimationFrame(() => {
        target.textarea.focus();
        target.textarea.selectionStart = index + token.length;
        target.textarea.selectionEnd = index + token.length;
      });
      return true;
    },
    [markPromptIncludeDropHandled, updatePrompt],
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

  const restoreGenerationFromSozocraftMetadata = useCallback(
    (metadata: unknown) => {
      if (!settings || !isPlainObject(metadata)) {
        return false;
      }
      const provider = stringValue(metadata.provider);
      if (!provider || !isImageProviderId(provider)) {
        return false;
      }

      const options = isPlainObject(metadata.options) ? metadata.options : {};
      const model = stringValue(metadata.model) ?? getProviderConfig(provider).defaults.model;
      const nextPlatform = platformForMetadata(provider, model, stringValue(metadata.platform), settings);
      const normalized = normalizeProviderOptions(provider, model, {
        aspectRatio: stringValue(options.aspectRatio) ?? "",
        imageSize: stringValue(options.imageSize) ?? "",
        quality: stringValue(options.quality) ?? "",
        thinkingLevel: stringValue(options.thinkingLevel) ?? "",
      }, nextPlatform);
      const importedTemperature = roundedNumberValue(options.temperature);
      const importedTopP = roundedNumberValue(options.topP);
      const importedUnlimited = booleanValue(options.unlimited);
      const importedOptions = {
        aspectRatio: normalized.aspectRatio,
        imageSize: normalized.imageSize,
        quality: normalized.quality,
        thinkingLevel: normalized.thinkingLevel,
        ...(importedTemperature === null ? {} : { temperature: importedTemperature }),
        ...(importedTopP === null ? {} : { topP: importedTopP }),
        ...(importedUnlimited === null ? {} : { unlimited: importedUnlimited }),
      };

      applyImportedOptions(provider, importedOptions);
      const nextSettings: AppSettings = {
        ...settings,
        defaultProvider: provider,
        defaultModel: normalized.model,
      };
      if (provider === "nano-banana") {
        nextSettings.nanoBananaApiPlatform = nextPlatform as NanoBananaApiPlatform;
      } else if (provider === "gpt-image") {
        nextSettings.openaiApiPlatform = nextPlatform as AppSettings["openaiApiPlatform"];
      } else if (provider === "grok-imagine") {
        nextSettings.grokApiPlatform = nextPlatform as GrokImagineApiPlatform;
      }
      setSettings(nextSettings);
      return true;
    },
    [applyImportedOptions, setSettings, settings],
  );

  const importPromptTextFile = useCallback(
    async (path: string) => {
      if (!isPromptTextPath(path)) {
        setStatus("error");
        setMessage("Drop one .md or .txt file into the prompt editor.");
        return;
      }
      const source = await readTextFile(path);
      await importPromptSource(fileNameFromPath(path), source);
    },
    [importPromptSource, setMessage, setStatus],
  );

  const importPromptFromImageMetadata = useCallback(
    async (path: string) => {
      if (!isSupportedImagePath(path)) {
        setStatus("error");
        setMessage("Drop one image file into Output Images to import prompt metadata.");
        return;
      }

      const metadata = await readImageTextMetadata(path);
      const importName = `${fileStemFromPath(path)} prompt`;
      const sozocraft = metadata.sozocraft?.trim();
      if (sozocraft) {
        try {
          const parsed: unknown = JSON.parse(sozocraft);
          if (isPlainObject(parsed)) {
            const promptSnapshot = stringValue(parsed.promptSnapshot)?.trim();
            if (promptSnapshot) {
              await importPromptSource(importName, promptSnapshot);
              const restored = restoreGenerationFromSozocraftMetadata(parsed);
              setStatus("ready");
              setMessage(
                restored
                  ? "Imported SozoCraft prompt and generation settings"
                  : "Imported SozoCraft prompt",
              );
              return;
            }
          }
        } catch {
          // Fall through to plain prompt metadata below.
        }
      }

      const promptMetadata = metadata.prompt?.trim();
      if (promptMetadata && !promptMetadata.startsWith("{")) {
        await importPromptSource(importName, promptMetadata);
        return;
      }
      setStatus("error");
      setMessage(
        promptMetadata?.startsWith("{")
          ? "Skipped JSON prompt metadata from this image."
          : "No reusable prompt metadata found in this image.",
      );
    },
    [importPromptSource, restoreGenerationFromSozocraftMetadata, setMessage, setStatus],
  );

  const handleNativeFileDrop = useCallback(
    async (zone: FileDropZone, paths: string[]) => {
      try {
        if (paths.length === 0) {
          return;
        }

        if (zone === "generation") {
          const added = await addReferenceImagePaths(paths);
          setStatus(added > 0 ? "ready" : "error");
          setMessage(
            added > 0
              ? `Added ${added} reference image${added === 1 ? "" : "s"}`
              : "No supported reference images were dropped.",
          );
          return;
        }

        if (paths.length !== 1) {
          setStatus("error");
          setMessage("Drop one file here. Multi-file drops are only for reference images.");
          return;
        }

        if (zone === "prompt-editor") {
          await importPromptTextFile(paths[0]);
          return;
        }

        await importPromptFromImageMetadata(paths[0]);
      } catch (error) {
        setStatus("error");
        setMessage(String(error));
      }
    },
    [
      addReferenceImagePaths,
      importPromptFromImageMetadata,
      importPromptTextFile,
      setMessage,
      setStatus,
    ],
  );

  useEffect(() => {
    let disposed = false;
    const unlistenPromise = getCurrentWebview()
      .onDragDropEvent((event) => {
        if (disposed) {
          return;
        }
        if (event.payload.type === "leave") {
          setActiveFileDropZone(null);
          return;
        }
        if (draggedPromptIncludeRef.current && event.payload.type !== "drop") {
          setActiveFileDropZone(null);
          return;
        }
        const zone = fileDropZoneFromPosition(event.payload.position);
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setActiveFileDropZone(zone);
          return;
        }
        setActiveFileDropZone(null);
        const draggedPromptInclude = draggedPromptIncludeRef.current;
        if (draggedPromptInclude) {
          const dropAlreadyHandled = Date.now() - promptIncludeDropHandledAtRef.current < 400;
          if (!dropAlreadyHandled) {
            insertPromptIncludeFromNativeDrop(draggedPromptInclude, event.payload.position);
          }
          return;
        }
        if (event.payload.paths.length === 0) {
          return;
        }
        if (zone) {
          void handleNativeFileDrop(zone, event.payload.paths);
        }
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      setActiveFileDropZone(null);
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, [handleNativeFileDrop, insertPromptIncludeFromNativeDrop]);

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
            <img className="brand-mark" src={sozocraftIcon} alt="" aria-hidden="true" />
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
          openrouterApiKey={openrouterApiKey}
          openrouterApiKeySaved={openrouterApiKeySaved}
          xaiApiKey={xaiApiKey}
          xaiApiKeySaved={xaiApiKeySaved}
          configStatus={configStatus}
          settings={settings}
          setApiKey={setApiKey}
          setOpenaiApiKey={setOpenaiApiKey}
          setOpenrouterApiKey={setOpenrouterApiKey}
          setSettings={setSettings}
          setXaiApiKey={setXaiApiKey}
          onSaveKey={() => void saveKey()}
          onSaveOpenaiKey={() => void saveOpenaiKey()}
          onSaveOpenrouterKey={() => void saveOpenrouterKey()}
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
            onPromptIncludeDragEnd={() => {
              window.setTimeout(() => {
                draggedPromptIncludeRef.current = null;
              }, 400);
            }}
            onPromptIncludeDragStart={(token) => {
              draggedPromptIncludeRef.current = token;
            }}
            onPromptIncludeDropHandled={markPromptIncludeDropHandled}
            onRenameTag={(oldTagPath, newTagPath) => void promptLibrary.renameTagPath(oldTagPath, newTagPath)}
            defaultExportPath={defaultExportPath}
            fileDropActive={activeFileDropZone === "prompt-editor"}
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
            onPromptIncludeDragEnd={() => {
              window.setTimeout(() => {
                draggedPromptIncludeRef.current = null;
              }, 400);
            }}
            onPromptIncludeDragStart={(token) => {
              draggedPromptIncludeRef.current = token;
            }}
            onPromptIncludeDropHandled={markPromptIncludeDropHandled}
            onRenameTag={(oldTagPath, newTagPath) => void promptLibrary.renameTagPath(oldTagPath, newTagPath)}
            defaultExportPath={defaultExportPath}
            fileDropActive={activeFileDropZone === "prompt-editor"}
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
            fileDropActive={activeFileDropZone === "generation"}
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
            fileDropActive={activeFileDropZone === "output"}
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
              <span>{providerPlatformDisplayName(settings)}</span>
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

function fileDropZoneFromPosition(position: { x: number; y: number }): FileDropZone | null {
  const candidates = [
    [position.x, position.y],
    [position.x / window.devicePixelRatio, position.y / window.devicePixelRatio],
  ];
  for (const [x, y] of candidates) {
    const element = document.elementFromPoint(x, y);
    const zoneElement = element?.closest<HTMLElement>("[data-file-drop-zone]");
    const zone = zoneElement?.dataset.fileDropZone;
    if (zone === "prompt-editor" || zone === "generation" || zone === "output") {
      return zone;
    }
  }
  return null;
}

function promptTextareaFromPosition(position: { x: number; y: number }) {
  const candidates = [
    [position.x, position.y],
    [position.x / window.devicePixelRatio, position.y / window.devicePixelRatio],
  ];
  for (const [x, y] of candidates) {
    const element = document.elementFromPoint(x, y);
    const textarea =
      element instanceof HTMLTextAreaElement
        ? element
        : element?.closest<HTMLTextAreaElement>("textarea.prompt-textarea");
    if (textarea) {
      return { textarea, x, y };
    }
  }
  return null;
}

function textareaIndexFromPoint(textarea: HTMLTextAreaElement, clientX: number, clientY: number) {
  const rect = textarea.getBoundingClientRect();
  const style = window.getComputedStyle(textarea);
  const paddingLeft = parseFloat(style.paddingLeft) || 0;
  const paddingRight = parseFloat(style.paddingRight) || 0;
  const paddingTop = parseFloat(style.paddingTop) || 0;
  const lineHeight = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.7 || 20;
  const charWidth = measureTextareaCharWidth(style);
  const contentWidth = Math.max(1, textarea.clientWidth - paddingLeft - paddingRight);
  const charsPerLine = Math.max(1, Math.floor(contentWidth / charWidth));
  const x = Math.max(0, clientX - rect.left - paddingLeft + textarea.scrollLeft);
  const y = Math.max(0, clientY - rect.top - paddingTop + textarea.scrollTop);
  const targetVisualLine = Math.max(0, Math.floor(y / lineHeight));
  const targetColumn = Math.max(0, Math.round(x / charWidth));
  const lines = textarea.value.split("\n");
  let sourceIndex = 0;
  let visualLine = 0;

  for (const line of lines) {
    const wrappedLines = Math.max(1, Math.ceil(Math.max(1, line.length) / charsPerLine));
    if (targetVisualLine < visualLine + wrappedLines) {
      const wrappedLine = targetVisualLine - visualLine;
      return sourceIndex + Math.min(line.length, wrappedLine * charsPerLine + targetColumn);
    }
    visualLine += wrappedLines;
    sourceIndex += line.length + 1;
  }

  return textarea.value.length;
}

function measureTextareaCharWidth(style: CSSStyleDeclaration) {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) {
    return 8;
  }
  context.font = style.font;
  return Math.max(1, context.measureText("0000000000").width / 10);
}

function isPromptTextPath(path: string) {
  return /\.(md|txt)$/i.test(path);
}

function fileStemFromPath(path: string) {
  return fileNameFromPath(path).replace(/\.[^.]+$/, "") || "Imported image";
}

function platformForMetadata(
  provider: ImageProviderId,
  model: string,
  metadataPlatform: string | null,
  settings: AppSettings,
): ImageProviderApiPlatform {
  if (provider === "nano-banana") {
    if (metadataPlatform === "higgsfield" || model.startsWith("nano_")) {
      return "higgsfield";
    }
    return settings.nanoBananaApiPlatform;
  }
  if (provider === "gpt-image") {
    if (metadataPlatform === "higgsfield" || model === "gpt_image_2") {
      return "higgsfield";
    }
    return model.startsWith("openai/") ? "openrouter" : "openai";
  }
  if (provider === "grok-imagine") {
    if (metadataPlatform === "higgsfield" || model === "grok_image") {
      return "higgsfield";
    }
    return settings.grokApiPlatform;
  }
  return "openai";
}

function providerPlatformDisplayName(settings: AppSettings) {
  if (settings.defaultProvider === "nano-banana") {
    return settings.nanoBananaApiPlatform === "higgsfield" ? "Higgsfield" : "Gemini";
  }
  if (settings.defaultProvider === "gpt-image") {
    if (settings.openaiApiPlatform === "higgsfield") {
      return "Higgsfield";
    }
    return settings.openaiApiPlatform === "openrouter" ? "OpenRouter" : "OpenAI";
  }
  return settings.grokApiPlatform === "higgsfield" ? "Higgsfield" : "xAI";
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringValue(value: unknown) {
  return typeof value === "string" && value.trim() ? value : null;
}

function roundedNumberValue(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? Math.round(value * 100) / 100 : null;
}

function booleanValue(value: unknown) {
  return typeof value === "boolean" ? value : null;
}
