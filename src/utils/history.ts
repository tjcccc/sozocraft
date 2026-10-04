import type { GenerationBatch } from "../types";
import { localDateString } from "./dates";

export function batchTitle(batch: GenerationBatch): string {
  const d = new Date(batch.createdAt);
  const date = localDateString(batch.createdAt);
  const time = [
    String(d.getHours()).padStart(2, "0"),
    String(d.getMinutes()).padStart(2, "0"),
    String(d.getSeconds()).padStart(2, "0"),
  ].join(":");
  const suffix = batch.mediaType === "video" ? " (video)" : batch.images.length > 1 ? " (batch)" : "";
  return `${date} ${time} - ${shortId(batch.id)}${suffix}`;
}

/** Mirrors the native resume check; Rust re-validates every resume request. */
export function canResumeVideoBatch(batch: GenerationBatch): boolean {
  return batch.mediaType === "video"
    && batch.status !== "completed"
    && Boolean(batch.providerRequestId)
    && batch.videoJob?.platform === "higgsfield";
}

function shortId(id: string) {
  return id.slice(0, 6);
}
