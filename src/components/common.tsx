import {
  AlertCircle,
  CheckCircle2,
  Image as ImageIcon,
  Loader2,
} from "lucide-react";
import type { ReactNode } from "react";
import type { OutputImage } from "../types";
import { formatTime } from "../utils/dates";

export type GenerationMode = "image" | "video";
export type AppStatus = "ready" | "running" | "error";

export function ColumnResizer({
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

export function Field({
  children,
  className,
  label,
}: {
  children: ReactNode;
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

export function ImageTile({
  failed,
  image,
  index,
  src,
  onPreview,
}: {
  failed?: boolean;
  image: OutputImage;
  index: number;
  src?: string;
  onPreview?: () => void;
}) {
  return (
    <article className="image-tile">
      {failed ? (
        <div className="image-placeholder missing">File not found</div>
      ) : src ? (
        <button className="image-preview-button" onClick={onPreview} type="button">
          <img alt={image.filename} src={src} />
        </button>
      ) : (
        <div className="image-placeholder">Loading</div>
      )}
      <footer>
        {index > 1 && <span># {index}</span>}
        {/*<time>{formatTime(image.createdAt)}</time>*/}
      </footer>
    </article>
  );
}

export function ModeSwitch({
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

export function PanelHeader({ icon, title }: { icon: ReactNode; title: string }) {
  return (
    <header className="panel-header">
      {icon}
      <h2>{title}</h2>
    </header>
  );
}

export function StatusPill({ label, status }: { label: string; status: AppStatus }) {
  const Icon = status === "running" ? Loader2 : status === "error" ? AlertCircle : CheckCircle2;
  return (
    <span className={`status-pill ${status}`}>
      <Icon className={status === "running" ? "spin" : ""} size={15} />
      {label}
    </span>
  );
}

export function StatusText({ status }: { status: string }) {
  return <span className={`status-text ${status}`}>{status}</span>;
}

export function TreeGroup({
  active,
  items,
  title,
}: {
  active?: string;
  items: string[];
  title: string;
}) {
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
