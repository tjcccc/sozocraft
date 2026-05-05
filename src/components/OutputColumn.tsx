import {
  AlertCircle,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Image as ImageIcon,
} from "lucide-react";
import { useCallback, useMemo, useRef, useState } from "react";
import type { GenerationBatch } from "../types";
import { localDateString } from "../utils/dates";
import { batchTitle } from "../utils/history";
import { clamp } from "../utils/math";
import { ImageTile, PanelHeader, StatusText } from "./common";
import type { LightboxImage } from "./ImageLightbox";

export function OutputColumn({
  batches,
  errorMessage,
  expandedBatchId,
  failedImagePaths,
  historyDate,
  imageDataUrls,
  previewBatch,
  setExpandedBatchId,
  setHistoryDate,
  onPreviewImages,
}: {
  batches: GenerationBatch[];
  errorMessage: string | null;
  expandedBatchId: string | null;
  failedImagePaths: Set<string>;
  historyDate: string;
  imageDataUrls: Record<string, string>;
  previewBatch?: GenerationBatch;
  setExpandedBatchId: (id: string | null) => void;
  setHistoryDate: (date: string) => void;
  onPreviewImages: (images: LightboxImage[], index: number) => void;
}) {
  const panelRef = useRef<HTMLElement>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historyHeight, setHistoryHeight] = useState(280);
  const [isResizingHistory, setIsResizingHistory] = useState(false);

  const todayStr = localDateString(new Date().toISOString());
  const previewImages = useMemo(
    () => lightboxImagesForBatch(previewBatch, imageDataUrls, failedImagePaths),
    [failedImagePaths, imageDataUrls, previewBatch],
  );

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
              onPreview={() => {
                const previewIndex = previewImages.findIndex((item) => item.id === image.id);
                onPreviewImages(previewImages, Math.max(0, previewIndex));
              }}
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

      <div
        className="history-section"
        style={historyOpen ? { flex: `0 0 ${historyHeight}px` } : { flex: "0 0 32px" }}
      >
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
              <button className="date-nav-btn" onClick={() => shiftDate(-1)}>
                ‹
              </button>
              <span>{historyDate === todayStr ? "Today" : historyDate}</span>
              <button
                className="date-nav-btn"
                disabled={historyDate >= todayStr}
                onClick={() => shiftDate(1)}
              >
                ›
              </button>
            </div>
            <div className="history">
              {batches.length === 0 ? <div className="history-empty">No tasks on this day</div> : null}
              {batches.map((batch) => (
                <div className="history-entry" key={batch.id}>
                  <button
                    className="history-row"
                    onClick={() => {
                      setExpandedBatchId(expandedBatchId === batch.id ? null : batch.id);
                    }}
                  >
                    {expandedBatchId === batch.id ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
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
                                <button
                                  className="history-preview-button"
                                  onClick={() => {
                                    const images = lightboxImagesForBatch(
                                      batch,
                                      imageDataUrls,
                                      failedImagePaths,
                                    );
                                    const previewIndex = images.findIndex((item) => item.id === image.id);
                                    onPreviewImages(images, Math.max(0, previewIndex));
                                  }}
                                  type="button"
                                >
                                  <img alt={image.filename} src={imageDataUrls[image.path]} />
                                </button>
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

function lightboxImagesForBatch(
  batch: GenerationBatch | undefined,
  imageDataUrls: Record<string, string>,
  failedImagePaths: Set<string>,
): LightboxImage[] {
  return (batch?.images ?? [])
    .filter((image) => imageDataUrls[image.path] && !failedImagePaths.has(image.path))
    .map((image) => ({
      id: image.id,
      alt: image.filename,
      src: imageDataUrls[image.path],
    }));
}
