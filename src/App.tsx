import { Loader2, Play, Settings, Square } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { saveOutputTemplate } from "./api";
import { GenerationPanel } from "./components/GenerationPanel";
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
import { clamp } from "./utils/math";

const MIN_COLUMN_WIDTHS = [24, 24, 28];

export function App() {
  const workspaceRef = useRef<HTMLElement>(null);
  const [mode, setMode] = useState<GenerationMode>("image");
  const [expandedBatchId, setExpandedBatchId] = useState<string | null>(null);
  const [previewBatchId, setPreviewBatchId] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [columnWidths, setColumnWidths] = useState([35.5, 27.2, 37.3]);
  const [resizingDivider, setResizingDivider] = useState<number | null>(null);

  const {
    apiKey,
    apiKeySaved,
    batches,
    configStatus,
    message,
    prompt,
    saveKey,
    setApiKey,
    setBatches,
    setMessage,
    setPrompt,
    setSettings,
    setStatus,
    settings,
    status,
    updateSettings,
  } = useAppState();

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
  const generation = useGeneration({
    prompt,
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
    settings,
    setAspectRatio: generation.setAspectRatio,
    setImageSize: generation.setImageSize,
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
          <button
            className="primary-button"
            disabled={status === "running"}
            onClick={generation.runGeneration}
          >
            {status === "running" ? <Loader2 className="spin" size={17} /> : <Play size={17} />}
            Run
          </button>
          <button className="secondary-button" disabled>
            <Square size={14} />
            Stop
          </button>
        </div>
        <div className="toolbar-right">
          <ModeSwitch mode={mode} setMode={setMode} />
          <div className="divider" />
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
          configStatus={configStatus}
          settings={settings}
          setApiKey={setApiKey}
          setSettings={setSettings}
          onSaveKey={() => void saveKey()}
          onSaveSettings={() => void updateSettings(settings)}
        />
      ) : null}

      <section
        className="workspace"
        ref={workspaceRef}
        style={{
          gridTemplateColumns: `${columnWidths[0]}fr 10px ${columnWidths[1]}fr 10px ${columnWidths[2]}fr`,
        }}
      >
        <PromptColumn prompt={prompt} setPrompt={setPrompt} />
        <ColumnResizer
          active={resizingDivider === 0}
          label="Resize prompt and generation columns"
          onPointerDown={(event) => startColumnResize(0, event)}
        />
        <GenerationPanel settings={settings} setSettings={setSettings} {...generation} />
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
          settings={settings}
          setExpandedBatchId={setExpandedBatchId}
          setHistoryDate={setHistoryDate}
          setPreviewBatchId={setPreviewBatchId}
          setSettings={setSettings}
          onSaveTemplate={(template) => {
            void saveOutputTemplate(template).catch(() => undefined);
          }}
        />
      </section>

      <footer className="statusbar">
        <StatusPill status={status} label={status === "error" ? "Error" : message} />
        <div className="status-info">
          <span>Gemini</span>
          <span>/</span>
          <span>{settings.defaultModel}</span>
        </div>
        <div className="status-counts">
          <span>Queue: 0</span>
          <span>Running: {status === "running" ? 1 : 0}</span>
          <span>Completed: {batches.reduce((count, batch) => count + batch.images.length, 0)}</span>
        </div>
      </footer>
    </main>
  );
}
