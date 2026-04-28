import {
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  ScrollText,
  Copy,
  FolderOpen,
  Image as ImageIcon,
  KeyRound,
  Loader2,
  Play,
  Save,
  Search,
  Settings,
  SlidersHorizontal,
  Square,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  generateImages,
  getConfigStatus,
  hasGeminiApiKey,
  loadAppState,
  readImageDataUrl,
  saveAppSettings,
  saveCurrentPrompt,
  saveOutputTemplate,
  setGeminiApiKey,
} from "./api";
import type { AppSettings, ConfigStatus, GenerationBatch, OutputImage } from "./types";

type GenerationMode = "image" | "video";

type ModelConfig = {
  aspectRatios: string[];
  imageSizes: string[] | null;
  supportsThinking: boolean;
};

const MODEL_CONFIGS: Record<string, ModelConfig> = {
  "gemini-3-pro-image-preview": {
    aspectRatios: ["Auto", "1:1", "9:16", "16:9", "3:4", "4:3", "2:3", "3:2", "4:5", "5:4"],
    imageSizes: ["512", "1K", "2K", "4K"],
    supportsThinking: false,
  },
  "gemini-3.1-flash-image-preview": {
    aspectRatios: ["Auto", "1:1", "9:16", "16:9", "3:4", "4:3", "2:3", "3:2", "4:5", "5:4"],
    imageSizes: ["512", "1K", "2K", "4K"],
    supportsThinking: true,
  },
  "gemini-2.5-flash-image": {
    aspectRatios: ["Auto", "1:1", "9:16", "16:9", "3:4", "4:3"],
    imageSizes: null,
    supportsThinking: false,
  },
};

const MODELS = Object.keys(MODEL_CONFIGS);
const MIN_COLUMN_WIDTHS = [24, 24, 28];

type Status = "ready" | "running" | "error";

export function App() {
  const workspaceRef = useRef<HTMLElement>(null);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [prompt, setPrompt] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [mode, setMode] = useState<GenerationMode>("image");
  const [batchCount, setBatchCount] = useState(1);
  const [aspectRatio, setAspectRatio] = useState("3:4");
  const [imageSize, setImageSize] = useState("2K");
  const [temperature, setTemperature] = useState(-1);
  const [topP, setTopP] = useState(0.95);
  const [seed, setSeed] = useState("");
  const [thinkingLevel, setThinkingLevel] = useState("HIGH");
  const [batches, setBatches] = useState<GenerationBatch[]>([]);
  const [expandedBatchId, setExpandedBatchId] = useState<string | null>(null);
  const [previewBatchId, setPreviewBatchId] = useState<string | null>(null);
  const [historyDate, setHistoryDate] = useState<string>(() =>
    localDateString(new Date().toISOString()),
  );
  const [imageDataUrls, setImageDataUrls] = useState<Record<string, string>>({});
  const [failedImagePaths, setFailedImagePaths] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<Status>("ready");
  const [message, setMessage] = useState("Ready");
  const [showSettings, setShowSettings] = useState(false);
  const [configStatus, setConfigStatus] = useState<ConfigStatus | null>(null);
  const [columnWidths, setColumnWidths] = useState([35.5, 27.2, 37.3]);
  const [resizingDivider, setResizingDivider] = useState<number | null>(null);

  useEffect(() => {
    void loadAppState()
      .then((state) => {
        setSettings(state.settings);
        setPrompt(state.currentPrompt);
        setBatches(state.batches);
        setMessage("Ready");
      })
      .catch((error) => {
        setStatus("error");
        setMessage(String(error));
      });

    void hasGeminiApiKey()
      .then(setApiKeySaved)
      .catch(() => setApiKeySaved(false));

    void getConfigStatus()
      .then(setConfigStatus)
      .catch(() => undefined);
  }, []);

  const filteredBatches = useMemo(
    () => batches.filter((b) => localDateString(b.createdAt) === historyDate),
    [batches, historyDate],
  );

  const expandedBatch = useMemo(
    () => batches.find((b) => b.id === expandedBatchId),
    [batches, expandedBatchId],
  );

  // Only show a preview after the user generates or explicitly selects a batch
  const previewBatch = useMemo(
    () => (previewBatchId ? batches.find((b) => b.id === previewBatchId) : undefined),
    [batches, previewBatchId],
  );

  const imagesToLoad = useMemo(() => {
    const seen = new Set<string>();
    for (const img of previewBatch?.images ?? []) seen.add(img.path);
    for (const img of expandedBatch?.images ?? []) seen.add(img.path);
    return [...seen];
  }, [previewBatch, expandedBatch]);

  useEffect(() => {
    setExpandedBatchId(null);
  }, [historyDate]);

  useEffect(() => {
    for (const path of imagesToLoad) {
      if (imageDataUrls[path] || failedImagePaths.has(path)) {
        continue;
      }
      void readImageDataUrl(path)
        .then((url) => setImageDataUrls((cur) => ({ ...cur, [path]: url })))
        .catch(() => setFailedImagePaths((cur) => new Set([...cur, path])));
    }
  }, [imagesToLoad, imageDataUrls, failedImagePaths]);

  const updateSettings = useCallback(
    async (next: AppSettings) => {
      setSettings(next);
      const state = await saveAppSettings(next);
      setBatches(state.batches);
      setMessage("Settings saved");
      setStatus("ready");
    },
    [],
  );

  const runGeneration = useCallback(async () => {
    if (!settings) {
      return;
    }
    setStatus("running");
    setMessage("Generating images");
    await saveCurrentPrompt(prompt);

    const requestSeed = seed.trim()
      ? Number(seed)
      : Math.floor(Math.random() * 2_147_483_647);
    if (!seed.trim()) {
      setSeed(String(requestSeed));
    }

    try {
      await saveAppSettings(settings);
      const batch = await generateImages({
        provider: "nano-banana",
        model: settings.defaultModel,
        prompt,
        batchCount,
        referenceImages: [],
        outputTemplate: settings.outputTemplate,
        baseUrl: settings.optionalBaseUrl,
        options: {
          aspectRatio,
          imageSize,
          temperature,
          topP,
          seed: requestSeed,
          thinkingLevel,
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
    seed,
    setSeed,
    settings,
    temperature,
    thinkingLevel,
    topP,
  ]);

  const saveKey = useCallback(async () => {
    try {
      await setGeminiApiKey(apiKey);
      setApiKey("");
      setApiKeySaved(apiKey.trim().length > 0);
      setStatus("ready");
      setMessage(apiKey.trim().length > 0 ? "API key saved" : "API key cleared");
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [apiKey]);

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
          <button className="primary-button" disabled={status === "running"} onClick={runGeneration}>
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
          <button className="icon-button" title="Settings" onClick={() => setShowSettings((value) => !value)}>
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
          onSaveSettings={() => void updateSettings(settings)}
          onSaveKey={() => void saveKey()}
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
        <GenerationColumn
          aspectRatio={aspectRatio}
          batchCount={batchCount}
          imageSize={imageSize}
          seed={seed}
          settings={settings}
          temperature={temperature}
          thinkingLevel={thinkingLevel}
          topP={topP}
          setAspectRatio={setAspectRatio}
          setBatchCount={setBatchCount}
          setImageSize={setImageSize}
          setSeed={setSeed}
          setSettings={setSettings}
          setTemperature={setTemperature}
          setThinkingLevel={setThinkingLevel}
          setTopP={setTopP}
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
        <StatusPill status={status} label={message} />
        <div className="status-info">
          <span>Provider: Gemini</span>
          <span>Model: {settings.defaultModel}</span>
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

function ColumnResizer({
  active,
  label,
  onPointerDown,
}: {
  active: boolean;
  label: string;
  onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => void;
}) {
  return (
    <div
      aria-label={label}
      aria-orientation="vertical"
      className={`column-resizer ${active ? "active" : ""}`}
      onPointerDown={onPointerDown}
      role="separator"
      tabIndex={0}
    />
  );
}

function PromptColumn({
  prompt,
  setPrompt,
}: {
  prompt: string;
  setPrompt: (value: string) => void;
}) {
  const editorWrapRef = useRef<HTMLDivElement>(null);
  const [previewHeight, setPreviewHeight] = useState(260);
  const [isPreviewResizing, setIsPreviewResizing] = useState(false);

  const startPreviewResize = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const bounds = editorWrapRef.current?.getBoundingClientRect();
    if (!bounds) {
      return;
    }

    event.preventDefault();
    setIsPreviewResizing(true);
    document.body.classList.add("is-panel-resizing");

    const onMove = (moveEvent: PointerEvent) => {
      const nextHeight = bounds.bottom - moveEvent.clientY;
      setPreviewHeight(clamp(nextHeight, 120, Math.max(120, bounds.height - 130)));
    };

    const onUp = () => {
      setIsPreviewResizing(false);
      document.body.classList.remove("is-panel-resizing");
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
  }, []);

  return (
    <section className="panel prompt-panel">
      <PanelHeader icon={<ScrollText size={16} />} title="Prompt Editor" />
      <div className="prompt-body">
        <aside className="prompt-library">
          <div className="search-box">
            <Search size={15} />
            <input placeholder="Search prompts..." />
          </div>
          <nav className="tree">
            <TreeGroup title="nanobanana" items={["photorealistic", "film_look"]} active="photorealistic" />
            <TreeGroup title="identity" items={["face_lock"]} />
            <TreeGroup title="gpt-image" items={["portrait", "ui_mockup"]} />
            <TreeGroup title="grok" items={["fury!"]} />
          </nav>
        </aside>
        <div className="editor-wrap" ref={editorWrapRef}>
          <div className="editor-topline">
            <strong>photorealistic</strong>
            <label className="toggle">
              <span>DSL</span>
              <input disabled type="checkbox" />
            </label>
          </div>
          <textarea
            className="prompt-textarea"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            spellCheck={false}
          />
          <div
            aria-label="Resize prompt editor and preview"
            aria-orientation="horizontal"
            className={`preview-resizer ${isPreviewResizing ? "active" : ""}`}
            onPointerDown={startPreviewResize}
            role="separator"
            tabIndex={0}
          />
          <div className="preview-box" style={{ flexBasis: previewHeight }}>
            <div className="preview-heading">
              <strong>Preview</strong>
              <span>Plain text</span>
            </div>
            <p>{prompt.trim() || "Enter a prompt to preview the final text."}</p>
          </div>
        </div>
      </div>
    </section>
  );
}

function GenerationColumn(props: {
  settings: AppSettings;
  setSettings: (settings: AppSettings) => void;
  batchCount: number;
  setBatchCount: (count: number) => void;
  aspectRatio: string;
  setAspectRatio: (value: string) => void;
  imageSize: string;
  setImageSize: (value: string) => void;
  temperature: number;
  setTemperature: (value: number) => void;
  topP: number;
  setTopP: (value: number) => void;
  seed: string;
  setSeed: (value: string) => void;
  thinkingLevel: string;
  setThinkingLevel: (value: string) => void;
}) {
  const modelConfig = MODEL_CONFIGS[props.settings.defaultModel] ?? MODEL_CONFIGS[MODELS[0]];

  return (
    <section className="panel generation-panel">
      <PanelHeader icon={<SlidersHorizontal size={16} />} title="Image Generation" />
      <div className="tabs">
        <button className="active">Nano Banana</button>
        <button disabled>GPT-Image</button>
        <button disabled>Grok Imagine</button>
      </div>
      <div className="form-grid">
        <Field label="Model" className="field-full">
          <select
            value={props.settings.defaultModel}
            onChange={(event) =>
              props.setSettings({ ...props.settings, defaultModel: event.target.value })
            }
          >
            {MODELS.map((model) => (
              <option key={model} value={model}>
                {model}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Aspect Ratio">
          <select value={props.aspectRatio} onChange={(event) => props.setAspectRatio(event.target.value)}>
            {modelConfig.aspectRatios.map((ratio) => (
              <option key={ratio} value={ratio}>
                {ratio}
              </option>
            ))}
          </select>
        </Field>
        {modelConfig.imageSizes ? (
          <Field label="Image Size">
            <select value={props.imageSize} onChange={(event) => props.setImageSize(event.target.value)}>
              {modelConfig.imageSizes.map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
        <Field label="Temperature">
          <input
            max={1}
            min={-1}
            step={0.01}
            type="number"
            value={props.temperature}
            onChange={(event) => props.setTemperature(Number(event.target.value))}
          />
        </Field>
        <Field label="Top P">
          <input
            max={1}
            min={0}
            step={0.01}
            type="number"
            value={props.topP}
            onChange={(event) => props.setTopP(Number(event.target.value))}
          />
        </Field>
        {modelConfig.supportsThinking ? (
          <Field label="Thinking">
            <select
              value={props.thinkingLevel}
              onChange={(event) => props.setThinkingLevel(event.target.value)}
            >
              <option value="MINIMAL">Minimal</option>
              <option value="LOW">Low</option>
              <option value="MEDIUM">Medium</option>
              <option value="HIGH">High</option>
            </select>
          </Field>
        ) : null}
        <Field label="Seed">
          <input
            placeholder="Auto"
            value={props.seed}
            onChange={(event) => props.setSeed(event.target.value)}
          />
        </Field>
        <Field label="Batch">
          <input
            max={8}
            min={1}
            type="number"
            value={props.batchCount}
            onChange={(event) => props.setBatchCount(Number(event.target.value))}
          />
        </Field>
      </div>
      <div className="reference-drop">
        <FolderOpen size={23} />
        <span>Reference image upload planned for v0.2</span>
      </div>
    </section>
  );
}

function OutputColumn({
  batches,
  errorMessage,
  expandedBatchId,
  failedImagePaths,
  historyDate,
  imageDataUrls,
  previewBatch,
  settings,
  setExpandedBatchId,
  setHistoryDate,
  setPreviewBatchId,
  setSettings,
  onSaveTemplate,
}: {
  batches: GenerationBatch[];
  errorMessage: string | null;
  expandedBatchId: string | null;
  failedImagePaths: Set<string>;
  historyDate: string;
  imageDataUrls: Record<string, string>;
  previewBatch?: GenerationBatch;
  settings: AppSettings;
  setExpandedBatchId: (id: string | null) => void;
  setHistoryDate: (date: string) => void;
  setPreviewBatchId: (id: string | null) => void;
  setSettings: (settings: AppSettings) => void;
  onSaveTemplate: (template: string) => void;
}) {
  const panelRef = useRef<HTMLElement>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historyHeight, setHistoryHeight] = useState(280);
  const [isResizingHistory, setIsResizingHistory] = useState(false);

  const templateIssues = validateOutputTemplate(settings.outputTemplate);
  const todayStr = localDateString(new Date().toISOString());

  function shiftDate(days: number) {
    const d = new Date(historyDate + "T12:00:00");
    d.setDate(d.getDate() + days);
    setHistoryDate(d.toISOString().slice(0, 10));
  }

  const startHistoryResize = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      setIsResizingHistory(true);
      document.body.classList.add("is-panel-resizing");

      const startY = event.clientY;
      const startH = historyHeight;
      const panelH = panelRef.current?.clientHeight ?? 700;

      const onMove = (e: PointerEvent) => {
        const delta = startY - e.clientY;
        setHistoryHeight(clamp(startH + delta, 120, Math.floor(panelH * 0.78)));
      };

      const onUp = () => {
        setIsResizingHistory(false);
        document.body.classList.remove("is-panel-resizing");
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
      };

      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp, { once: true });
    },
    [historyHeight],
  );

  return (
    <section className="panel output-panel" ref={panelRef}>
      <PanelHeader icon={<ImageIcon size={16} />} title="Output Images" />
      <Field label="Filename Template">
        <div className="template-row">
          <input
            value={settings.outputTemplate}
            onChange={(event) => setSettings({ ...settings, outputTemplate: event.target.value })}
            onBlur={(event) => onSaveTemplate(event.target.value)}
          />
          <button className="icon-button" title="Copy template">
            <Copy size={16} />
          </button>
        </div>
        {templateIssues.length > 0 ? (
          <div className="template-issues">
            {templateIssues.map((issue, index) => (
              <span className="template-issue" key={index}>{issue}</span>
            ))}
          </div>
        ) : null}
      </Field>

      <div className="active-batch">
        {errorMessage ? (
          <div className="preview-error">
            <AlertCircle size={13} />
            <span>{errorMessage}</span>
          </div>
        ) : null}
        <div className={`gallery${(previewBatch?.images ?? []).length === 1 ? " gallery-single" : ""}`}>
          {(previewBatch?.images ?? []).map((image, index) => (
            <ImageTile
              failed={failedImagePaths.has(image.path)}
              image={image}
              index={index + 1}
              key={image.id}
              src={imageDataUrls[image.path]}
            />
          ))}
        </div>
        {previewBatch ? (
          <div className="batch-info-bar">
            <strong>{batchTitle(previewBatch)}</strong>
            <StatusText status={previewBatch.status} />
          </div>
        ) : null}
      </div>

      <div className="history-section" style={historyOpen ? { flex: `0 0 ${historyHeight}px` } : { flex: "0 0 32px" }}>
        {historyOpen ? (
          <div
            aria-label="Resize history panel"
            aria-orientation="horizontal"
            className={`history-resize-handle${isResizingHistory ? " active" : ""}`}
            onPointerDown={startHistoryResize}
            role="separator"
          />
        ) : null}
        <button className="history-toggle-bar" onClick={() => setHistoryOpen((v) => !v)}>
          {historyOpen ? <ChevronDown size={12} /> : <ChevronUp size={12} />}
          <span>History</span>
        </button>
        {historyOpen ? (
          <>
            <div className="history-date-bar">
              <button className="date-nav-btn" onClick={() => shiftDate(-1)}>‹</button>
              <span>{historyDate === todayStr ? "Today" : historyDate}</span>
              <button
                className="date-nav-btn"
                disabled={historyDate >= todayStr}
                onClick={() => shiftDate(1)}
              >›</button>
            </div>
            <div className="history">
              {batches.length === 0 ? (
                <div className="history-empty">No tasks on this day</div>
              ) : null}
              {batches.map((batch) => (
                <div className="history-entry" key={batch.id}>
                  <button
                    className="history-row"
                    onClick={() => {
                      setPreviewBatchId(batch.id);
                      setExpandedBatchId(expandedBatchId === batch.id ? null : batch.id);
                    }}
                  >
                    {expandedBatchId === batch.id
                      ? <ChevronDown size={12} />
                      : <ChevronRight size={12} />}
                    <span className="history-title">{batchTitle(batch)}</span>
                    <StatusText status={batch.status} />
                  </button>
                  {expandedBatchId === batch.id ? (
                    <div className="history-expand">
                      {batch.images.length > 0 ? (
                        <div className="history-thumbnails">
                          {batch.images.map((image) => (
                            <div className="history-thumb" key={image.id}>
                              {failedImagePaths.has(image.path) ? (
                                <div className="thumb-state missing">File not found</div>
                              ) : imageDataUrls[image.path] ? (
                                <img alt={image.filename} src={imageDataUrls[image.path]} />
                              ) : (
                                <div className="thumb-state">Loading</div>
                              )}
                            </div>
                          ))}
                        </div>
                      ) : null}
                      {batch.status === "failed" && batch.error ? (
                        <div className="batch-error-detail">
                          <pre>{batch.error}</pre>
                        </div>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              ))}
            </div>
          </>
        ) : null}
      </div>
    </section>
  );
}

function SettingsPanel({
  apiKey,
  apiKeySaved,
  configStatus,
  settings,
  setApiKey,
  setSettings,
  onSaveKey,
  onSaveSettings,
}: {
  apiKey: string;
  apiKeySaved: boolean;
  configStatus: ConfigStatus | null;
  settings: AppSettings;
  setApiKey: (value: string) => void;
  setSettings: (settings: AppSettings) => void;
  onSaveKey: () => void;
  onSaveSettings: () => void;
}) {
  return (
    <section className="settings-panel">
      <Field label="Gemini API Key">
        <div className="template-row">
          <input
            placeholder={apiKeySaved ? "Stored in ~/.sozocraft/config.toml" : "Paste API key"}
            type="password"
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
          />
          <button className="secondary-button" onClick={onSaveKey}>
            <KeyRound size={15} />
            Save Key
          </button>
        </div>
      </Field>
      <Field label="Output Directory">
        <input
          value={settings.outputDirectory}
          onChange={(event) => setSettings({ ...settings, outputDirectory: event.target.value })}
        />
      </Field>
      <Field label="Base URL">
        <input
          placeholder="Default Google Generative Language API"
          value={settings.optionalBaseUrl ?? ""}
          onChange={(event) =>
            setSettings({ ...settings, optionalBaseUrl: event.target.value || null })
          }
        />
      </Field>
      <Field label="Proxy URL">
        <input
          placeholder="http://127.0.0.1:7890"
          value={settings.proxyUrl ?? ""}
          onChange={(event) => setSettings({ ...settings, proxyUrl: event.target.value || null })}
        />
      </Field>
      <Field label="Timeout">
        <input
          min={10}
          type="number"
          value={settings.timeoutSeconds}
          onChange={(event) =>
            setSettings({ ...settings, timeoutSeconds: Number(event.target.value) })
          }
        />
      </Field>
      <button className="primary-button" onClick={onSaveSettings}>
        <Save size={15} />
        Save Settings
      </button>
      {configStatus ? (
        <div className="config-status">
          <div className="config-status-row">
            <span className="config-status-label">Config file</span>
            <code className="config-status-value">{configStatus.configPath}</code>
          </div>
          <div className="config-status-row">
            <span className="config-status-label">Gemini API key</span>
            <span className={`config-status-badge ${configStatus.hasApiKey ? "ok" : "missing"}`}>
              {configStatus.hasApiKey ? "Configured" : "Missing"}
            </span>
          </div>
          <div className="config-status-row">
            <span className="config-status-label">Proxy</span>
            <span className={`config-status-badge ${configStatus.hasProxy ? "ok" : "missing"}`}>
              {configStatus.hasProxy ? "Configured" : "Not set"}
            </span>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function ImageTile({
  failed,
  image,
  index,
  src,
}: {
  failed?: boolean;
  image: OutputImage;
  index: number;
  src?: string;
}) {
  return (
    <article className="image-tile">
      {failed ? (
        <div className="image-placeholder missing">File not found</div>
      ) : src ? (
        <img alt={image.filename} src={src} />
      ) : (
        <div className="image-placeholder">Loading</div>
      )}
      <footer>
        <span># {index}</span>
        {/*<span>{image.filename}</span>*/}
        <time>{formatTime(image.createdAt)}</time>
      </footer>
    </article>
  );
}

function TreeGroup({ active, items, title }: { active?: string; items: string[]; title: string }) {
  return (
    <div className="tree-group">
      <strong>{title}</strong>
      {items.map((item) => (
        <button className={item === active ? "active" : ""} key={item}>
          {item}
        </button>
      ))}
    </div>
  );
}

function PanelHeader({ icon, title }: { icon: React.ReactNode; title: string }) {
  return (
    <header className="panel-header">
      {icon}
      <h2>{title}</h2>
    </header>
  );
}

function Field({
  children,
  className,
  label,
}: {
  children: React.ReactNode;
  className?: string;
  label: string;
}) {
  return (
    <label className={`field${className ? ` ${className}` : ""}`}>
      <span>{label}</span>
      {children}
    </label>
  );
}

function ModeSwitch({
  mode,
  setMode,
}: {
  mode: GenerationMode;
  setMode: (mode: GenerationMode) => void;
}) {
  return (
    <div className="mode-switch">
      <button className={mode === "image" ? "active" : ""} onClick={() => setMode("image")}>
        <ImageIcon size={14} />
        Image
      </button>
      <button disabled className={mode === "video" ? "active" : ""}>
        Video
      </button>
    </div>
  );
}

function StatusPill({ label, status }: { label: string; status: Status }) {
  const Icon = status === "running" ? Loader2 : status === "error" ? AlertCircle : CheckCircle2;
  return (
    <span className={`status-pill ${status}`}>
      <Icon className={status === "running" ? "spin" : ""} size={15} />
      {label}
    </span>
  );
}

function StatusText({ status }: { status: string }) {
  return <span className={`status-text ${status}`}>{status}</span>;
}

function shortId(id: string) {
  return id.slice(0, 6);
}

function localDateString(isoUtc: string): string {
  const d = new Date(isoUtc);
  return [
    d.getFullYear(),
    String(d.getMonth() + 1).padStart(2, "0"),
    String(d.getDate()).padStart(2, "0"),
  ].join("-");
}

function providerDisplayName(provider: string): string {
  switch (provider) {
    case "nano-banana": return "Gemini";
    default: return provider;
  }
}

function modelDisplayName(model: string): string {
  switch (model) {
    case "gemini-3-pro-image-preview": return "Nano Banana Pro";
    case "gemini-3.1-flash-image-preview": return "Nano Banana 2";
    case "gemini-2.5-flash-image": return "Nano Banana";
    default: return model;
  }
}

function batchTitle(batch: GenerationBatch): string {
  const d = new Date(batch.createdAt);
  const date = localDateString(batch.createdAt);
  const time = [
    String(d.getHours()).padStart(2, "0"),
    String(d.getMinutes()).padStart(2, "0"),
    String(d.getSeconds()).padStart(2, "0"),
  ].join(":");
  const prov = providerDisplayName(batch.provider);
  const mod = modelDisplayName(batch.model);
  const suffix = batch.images.length > 1 ? " (batch)" : "";
  return `${date} ${time} - ${prov} / ${mod} - ${shortId(batch.id)}${suffix}`;
}

function formatTime(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(value));
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

const SUPPORTED_DATE_TOKENS = new Set([
  "yyyyMMdd_HHmmss", "yyyyMMdd", "yyMMdd_HHmmss", "yyMMdd",
  "yyyy", "yy", "MM", "dd", "HH", "mm", "ss",
]);

function validateOutputTemplate(template: string): string[] {
  const issues: string[] = [];
  if (!template.trim()) {
    issues.push("Template cannot be empty.");
    return issues;
  }
  const tokenRegex = /\{([^}]+)\}/g;
  const known = new Set([
    "provider", "model", "id", "batch_id", "extension", "datetime",
    ...SUPPORTED_DATE_TOKENS,
  ]);
  let match;
  while ((match = tokenRegex.exec(template)) !== null) {
    const token = match[1];
    if (token.startsWith("datetime:")) {
      continue;
    }
    if (!known.has(token)) {
      const looksLikeDate = /^[YMDHhmsS_]+$/.test(token);
      if (looksLikeDate) {
        issues.push(`{${token}} — use lowercase tokens, e.g. {yyyyMMdd_HHmmss}`);
      } else {
        issues.push(`{${token}} is not a recognised token and will render literally.`);
      }
    }
  }
  return issues;
}
