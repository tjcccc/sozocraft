import {
  AlertCircle,
  CheckCircle2,
  Circle,
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
  hasGeminiApiKey,
  loadAppState,
  readImageDataUrl,
  saveAppSettings,
  saveCurrentPrompt,
  setGeminiApiKey,
} from "./api";
import type { AppSettings, GenerationBatch, OutputImage } from "./types";

const MODELS = [
  "gemini-3-pro-image-preview",
  "gemini-3.1-flash-image-preview",
  "gemini-2.5-flash-image",
];

const ASPECT_RATIOS = [
  "Auto",
  "1:1",
  "2:3",
  "3:2",
  "3:4",
  "4:3",
  "4:5",
  "5:4",
  "9:16",
  "16:9",
  "21:9",
  "1:4",
  "4:1",
  "1:8",
  "8:1",
];

const IMAGE_SIZES = ["512", "1K", "2K", "4K"];
const MIN_COLUMN_WIDTHS = [24, 24, 28];

type Status = "ready" | "running" | "error";

export function App() {
  const workspaceRef = useRef<HTMLElement>(null);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [prompt, setPrompt] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [batchCount, setBatchCount] = useState(1);
  const [aspectRatio, setAspectRatio] = useState("16:9");
  const [imageSize, setImageSize] = useState("1K");
  const [temperature, setTemperature] = useState(0.5);
  const [topP, setTopP] = useState(0.95);
  const [seed, setSeed] = useState("");
  const [thinkingLevel, setThinkingLevel] = useState("MINIMAL");
  const [batches, setBatches] = useState<GenerationBatch[]>([]);
  const [activeBatchId, setActiveBatchId] = useState<string | null>(null);
  const [imageDataUrls, setImageDataUrls] = useState<Record<string, string>>({});
  const [status, setStatus] = useState<Status>("ready");
  const [message, setMessage] = useState("Ready");
  const [showSettings, setShowSettings] = useState(false);
  const [columnWidths, setColumnWidths] = useState([35.5, 27.2, 37.3]);
  const [resizingDivider, setResizingDivider] = useState<number | null>(null);

  useEffect(() => {
    void loadAppState()
      .then((state) => {
        setSettings(state.settings);
        setPrompt(state.currentPrompt);
        setBatches(state.batches);
        setActiveBatchId(state.batches[0]?.id ?? null);
        setImageSize(state.settings.defaultModel === "gemini-2.5-flash-image" ? "1K" : "1K");
        setMessage("Ready");
      })
      .catch((error) => {
        setStatus("error");
        setMessage(String(error));
      });

    void hasGeminiApiKey()
      .then(setApiKeySaved)
      .catch(() => setApiKeySaved(false));
  }, []);

  const activeBatch = useMemo(
    () => batches.find((batch) => batch.id === activeBatchId) ?? batches[0],
    [activeBatchId, batches],
  );

  const activeImages = activeBatch?.images ?? [];

  useEffect(() => {
    for (const image of activeImages) {
      if (imageDataUrls[image.path]) {
        continue;
      }
      void readImageDataUrl(image.path)
        .then((url) => setImageDataUrls((current) => ({ ...current, [image.path]: url })))
        .catch(() => undefined);
    }
  }, [activeImages, imageDataUrls]);

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
          seed: seed.trim() ? Number(seed) : null,
          thinkingLevel,
        },
      });
      setBatches((current) => [batch, ...current.filter((item) => item.id !== batch.id)]);
      setActiveBatchId(batch.id);
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
        <span>Loading VisionCraft</span>
      </main>
    );
  }

  return (
    <main className="app-shell">
      <header className="toolbar">
        <div className="brand">
          <div className="brand-mark">V</div>
          <strong>VisionCraft</strong>
        </div>
        <div className="toolbar-actions">
          <button className="primary-button" disabled={status === "running"} onClick={runGeneration}>
            {status === "running" ? <Loader2 className="spin" size={17} /> : <Play size={17} />}
            Run
          </button>
          <button className="secondary-button" disabled>
            <Square size={14} />
            Stop
          </button>
          <div className="divider" />
          <label className="stepper">
            <span>Batch</span>
            <input
              min={1}
              max={8}
              type="number"
              value={batchCount}
              onChange={(event) => setBatchCount(Number(event.target.value))}
            />
          </label>
        </div>
        <div className="toolbar-status">
          <StatusPill status={status} label={message} />
          <span>Provider: Gemini</span>
          <span>Model: {settings.defaultModel}</span>
        </div>
        <button className="icon-button" title="Settings" onClick={() => setShowSettings((value) => !value)}>
          <Settings size={18} />
        </button>
      </header>

      {showSettings ? (
        <SettingsPanel
          apiKey={apiKey}
          apiKeySaved={apiKeySaved}
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
          activeBatch={activeBatch}
          batches={batches}
          imageDataUrls={imageDataUrls}
          settings={settings}
          setActiveBatchId={setActiveBatchId}
          setSettings={setSettings}
        />
      </section>

      <footer className="statusbar">
        <StatusPill status={status} label={message} />
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
      <PanelHeader icon={<Circle size={16} />} title="Prompt Editor" />
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
  return (
    <section className="panel generation-panel">
      <PanelHeader icon={<SlidersHorizontal size={16} />} title="Image Generation" />
      <div className="tabs">
        <button className="active">Nano Banana</button>
        <button disabled>GPT-Image</button>
        <button disabled>Grok Imagine</button>
      </div>
      <div className="form-grid">
        <Field label="Model">
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
            {ASPECT_RATIOS.map((ratio) => (
              <option key={ratio} value={ratio}>
                {ratio}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Image Size">
          <select value={props.imageSize} onChange={(event) => props.setImageSize(event.target.value)}>
            {IMAGE_SIZES.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Seed">
          <input placeholder="Random" value={props.seed} onChange={(event) => props.setSeed(event.target.value)} />
        </Field>
        <Field label="Temperature">
          <input
            max={1}
            min={0}
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
  activeBatch,
  batches,
  imageDataUrls,
  settings,
  setActiveBatchId,
  setSettings,
}: {
  activeBatch?: GenerationBatch;
  batches: GenerationBatch[];
  imageDataUrls: Record<string, string>;
  settings: AppSettings;
  setActiveBatchId: (id: string) => void;
  setSettings: (settings: AppSettings) => void;
}) {
  return (
    <section className="panel output-panel">
      <PanelHeader icon={<ImageIcon size={16} />} title="Output Images" />
      <Field label="Filename Template">
        <div className="template-row">
          <input
            value={settings.outputTemplate}
            onChange={(event) => setSettings({ ...settings, outputTemplate: event.target.value })}
          />
          <button className="icon-button" title="Copy template">
            <Copy size={16} />
          </button>
        </div>
      </Field>
      <div className="active-batch">
        <div className="batch-heading">
          <strong>{activeBatch ? `Batch ${shortId(activeBatch.id)}` : "No batches yet"}</strong>
          <span>{activeBatch?.images.length ?? 0} images</span>
          {activeBatch ? <StatusText status={activeBatch.status} /> : null}
        </div>
        <div className="gallery">
          {(activeBatch?.images ?? []).map((image, index) => (
            <ImageTile
              image={image}
              index={index + 1}
              key={image.id}
              src={imageDataUrls[image.path]}
            />
          ))}
        </div>
      </div>
      <div className="history">
        {batches.slice(0, 12).map((batch) => (
          <button className="history-row" key={batch.id} onClick={() => setActiveBatchId(batch.id)}>
            <strong>Batch {shortId(batch.id)}</strong>
            <span>{batch.images.length} images</span>
            <StatusText status={batch.status} />
            <time>{formatTime(batch.createdAt)}</time>
          </button>
        ))}
      </div>
    </section>
  );
}

function SettingsPanel({
  apiKey,
  apiKeySaved,
  settings,
  setApiKey,
  setSettings,
  onSaveKey,
  onSaveSettings,
}: {
  apiKey: string;
  apiKeySaved: boolean;
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
            placeholder={apiKeySaved ? "Stored in ~/.visioncraft/config.toml" : "Paste API key"}
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
    </section>
  );
}

function ImageTile({ image, index, src }: { image: OutputImage; index: number; src?: string }) {
  return (
    <article className="image-tile">
      {src ? <img alt={image.filename} src={src} /> : <div className="image-placeholder">Loading</div>}
      <footer>
        <span>#{index}</span>
        <span>{image.filename}</span>
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

function Field({ children, label }: { children: React.ReactNode; label: string }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
    </label>
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

function formatTime(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}
