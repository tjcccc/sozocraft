import type { GenerationBatch } from "../types";
import { geminiImageModelDisplayName } from "../models/geminiImageModels";
import { localDateString } from "./dates";

export function batchTitle(batch: GenerationBatch): string {
  const d = new Date(batch.createdAt);
  const date = localDateString(batch.createdAt);
  const time = [
    String(d.getHours()).padStart(2, "0"),
    String(d.getMinutes()).padStart(2, "0"),
    String(d.getSeconds()).padStart(2, "0"),
  ].join(":");
  const suffix = batch.images.length > 1 ? " (batch)" : "";
  return `${date} ${time} - ${providerDisplayName(batch.provider)} / ${geminiImageModelDisplayName(batch.model)} - ${shortId(batch.id)}${suffix}`;
}

function providerDisplayName(provider: string): string {
  switch (provider) {
    case "nano-banana":
      return "Gemini";
    default:
      return provider;
  }
}

function shortId(id: string) {
  return id.slice(0, 6);
}
