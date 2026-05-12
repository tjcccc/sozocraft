import { readImageDataUrl } from "../api";
import type { ReferenceImageInput } from "../types";

export async function fileToReferenceImage(file: File): Promise<ReferenceImageInput> {
  const dataUrl = await readFileAsDataUrl(file);
  return dataUrlToReferenceImage(file.name, dataUrl, `${file.name}-${file.lastModified}`);
}

export async function pathToReferenceImage(path: string): Promise<ReferenceImageInput> {
  const dataUrl = await readImageDataUrl(path);
  return dataUrlToReferenceImage(fileNameFromPath(path), dataUrl, path);
}

export function isSupportedImagePath(path: string): boolean {
  return /\.(png|jpe?g|webp)$/i.test(path);
}

export function fileNameFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? "reference-image";
}

function dataUrlToReferenceImage(name: string, dataUrl: string, idPrefix: string): ReferenceImageInput {
  const base64Marker = ";base64,";
  const base64Index = dataUrl.indexOf(base64Marker);
  const data = base64Index >= 0 ? dataUrl.slice(base64Index + base64Marker.length) : "";
  const mimeType = dataUrl.startsWith("data:") && base64Index >= 0 ? dataUrl.slice(5, base64Index) : "";

  return {
    id: `${idPrefix}-${newReferenceImageId()}`,
    name,
    mimeType: mimeType || mimeTypeFromName(name),
    data,
    dataUrl,
  };
}

function mimeTypeFromName(name: string): string {
  const lower = name.toLowerCase();
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) {
    return "image/jpeg";
  }
  if (lower.endsWith(".webp")) {
    return "image/webp";
  }
  return "image/png";
}

function newReferenceImageId(): string {
  return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error ?? new Error("Failed to read image file."));
    reader.readAsDataURL(file);
  });
}
