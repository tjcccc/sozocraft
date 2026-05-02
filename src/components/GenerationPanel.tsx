import { FilePlus2, SlidersHorizontal, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { readImageDataUrl } from "../api";
import type { AppSettings, ReferenceImageInput } from "../types";
import type { LightboxImage } from "./ImageLightbox";
import { getGeminiImageModelConfig } from "../models/geminiImageModels";
import {
  IMAGE_PROVIDER_IDS,
  getProviderConfig,
  normalizeProviderOptions,
} from "../models/imageProviders";
import { Field, PanelHeader } from "./common";

export function GenerationPanel(props: {
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
  quality: string;
  setQuality: (value: string) => void;
  thinkingLevel: string;
  setThinkingLevel: (value: string) => void;
  referenceImages: ReferenceImageInput[];
  setReferenceImages: Dispatch<SetStateAction<ReferenceImageInput[]>>;
  onPreviewImages: (images: LightboxImage[], index: number) => void;
}) {
  const providerConfig = getProviderConfig(props.settings.defaultProvider);
  const modelConfig =
    props.settings.defaultProvider === "nano-banana"
      ? getGeminiImageModelConfig(props.settings.defaultModel)
      : null;
  const aspectRatios = modelConfig?.aspectRatios ?? providerConfig.aspectRatios;
  const imageSizes = modelConfig?.imageSizes ?? providerConfig.imageSizes;
  const thinkingLevels = modelConfig?.thinkingLevels ?? providerConfig.thinkingLevels;
  const providerModelConfig = providerConfig.models.find((model) => model.id === props.settings.defaultModel);
  const maxReferenceImages =
    modelConfig?.maxReferenceImages ??
    providerModelConfig?.maxReferenceImages ??
    providerConfig.maxReferenceImages;
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isReferenceDropActive, setIsReferenceDropActive] = useState(false);
  const canAddReferenceImages = props.referenceImages.length < maxReferenceImages;

  async function addReferenceFiles(files: FileList | null) {
    if (!files || files.length === 0) {
      return;
    }

    const remaining = maxReferenceImages - props.referenceImages.length;
    const nextFiles = [...files]
      .filter((file) => file.type.startsWith("image/"))
      .slice(0, Math.max(0, remaining));
    const nextImages = await Promise.all(nextFiles.map(fileToReferenceImage));

    props.setReferenceImages((current) => [...current, ...nextImages].slice(0, maxReferenceImages));
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  }

  async function addReferencePaths(paths: string[]) {
    const remaining = maxReferenceImages - props.referenceImages.length;
    const nextPaths = paths.filter(isSupportedImagePath).slice(0, Math.max(0, remaining));
    const nextImages = await Promise.all(nextPaths.map(pathToReferenceImage));

    props.setReferenceImages((current) => [...current, ...nextImages].slice(0, maxReferenceImages));
  }

  useEffect(() => {
    let disposed = false;
    const unlistenPromise = getCurrentWebview()
      .onDragDropEvent((event) => {
        if (disposed) {
          return;
        }
        if (event.payload.type === "leave" || !canAddReferenceImages) {
          setIsReferenceDropActive(false);
          return;
        }
        if (event.payload.type === "enter") {
          setIsReferenceDropActive(event.payload.paths.some(isSupportedImagePath));
          return;
        }
        if (event.payload.type === "over") {
          return;
        }
        const hasSupportedImage = event.payload.paths.some(isSupportedImagePath);
        if (!hasSupportedImage) {
          setIsReferenceDropActive(false);
          return;
        }
        setIsReferenceDropActive(false);
        void addReferencePaths(event.payload.paths);
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, [canAddReferenceImages, maxReferenceImages, props.referenceImages.length]);

  return (
    <section className="panel generation-panel">
      <PanelHeader icon={<SlidersHorizontal size={16} />} title="Image Generation" />
      <div className="tabs">
        {IMAGE_PROVIDER_IDS.map((provider) => (
          <button
            className={props.settings.defaultProvider === provider ? "active" : ""}
            key={provider}
            onClick={() => {
              const config = getProviderConfig(provider);
              const next = normalizeProviderOptions(provider, config.defaults.model, {
                aspectRatio: props.aspectRatio,
                imageSize: props.imageSize,
                quality: props.quality,
                thinkingLevel: props.thinkingLevel,
              });
              props.setSettings({
                ...props.settings,
                defaultProvider: provider,
                defaultModel: next.model,
              });
              props.setAspectRatio(next.aspectRatio);
              props.setImageSize(next.imageSize);
              props.setQuality(next.quality);
              props.setThinkingLevel(next.thinkingLevel);
              props.setReferenceImages((current) => current.slice(0, next.maxReferenceImages));
            }}
            type="button"
          >
            {getProviderConfig(provider).label}
          </button>
        ))}
      </div>
      <div className="form-grid">
        <Field label="Model" className="field-full">
          <select
            value={props.settings.defaultModel}
            onChange={(event) => {
              const nextModel = event.target.value;
              const next = normalizeProviderOptions(props.settings.defaultProvider, nextModel, {
                aspectRatio: props.aspectRatio,
                imageSize: props.imageSize,
                quality: props.quality,
                thinkingLevel: props.thinkingLevel,
              });
              props.setSettings({ ...props.settings, defaultModel: nextModel });
              props.setAspectRatio(next.aspectRatio);
              props.setImageSize(next.imageSize);
              props.setQuality(next.quality);
              props.setThinkingLevel(next.thinkingLevel);
              props.setReferenceImages((current) => current.slice(0, next.maxReferenceImages));
            }}
          >
            {providerConfig.models.map((model) => (
              <option key={model.id} value={model.id}>
                {model.productName}
              </option>
            ))}
          </select>
        </Field>
        {aspectRatios.length > 0 ? (
          <Field label="Aspect Ratio">
            <select value={props.aspectRatio} onChange={(event) => props.setAspectRatio(event.target.value)}>
              {aspectRatios.map((ratio) => (
                <option key={ratio} value={ratio}>
                  {ratio}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
        {imageSizes ? (
          <Field label="Image Size">
            <select value={props.imageSize} onChange={(event) => props.setImageSize(event.target.value)}>
              {imageSizes.map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
        {props.settings.defaultProvider === "nano-banana" ? (
          <>
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
          </>
        ) : null}
        {providerConfig.qualityLevels ? (
          <Field label="Quality">
            <select value={props.quality} onChange={(event) => props.setQuality(event.target.value)}>
              {providerConfig.qualityLevels.map((level) => (
                <option key={level} value={level}>
                  {level}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
        {thinkingLevels ? (
          <Field label="Thinking">
            <select
              value={props.thinkingLevel}
              onChange={(event) => props.setThinkingLevel(event.target.value)}
            >
              {thinkingLevels.map((level) => (
                <option key={level} value={level}>
                  {level[0].toUpperCase() + level.slice(1)}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
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
      {maxReferenceImages > 0 ? (
        <div className="reference-field">
          <span className="reference-label">Reference Images</span>
          <div className="reference-images">
            <button
              className={`reference-add ${isReferenceDropActive ? "drop-active" : ""}`}
              disabled={!canAddReferenceImages}
              onDragEnter={(event) => {
                if (!canAddReferenceImages) {
                  return;
                }
                event.preventDefault();
                setIsReferenceDropActive(true);
              }}
              onDragLeave={() => setIsReferenceDropActive(false)}
              onDragOver={(event) => {
                if (!canAddReferenceImages) {
                  return;
                }
                event.preventDefault();
                event.dataTransfer.dropEffect = "copy";
                setIsReferenceDropActive(true);
              }}
              onDrop={(event) => {
                if (!canAddReferenceImages) {
                  return;
                }
                event.preventDefault();
                setIsReferenceDropActive(false);
                void addReferenceFiles(event.dataTransfer.files);
              }}
              onClick={() => fileInputRef.current?.click()}
              title={
                canAddReferenceImages
                  ? "Add reference image"
                  : `Maximum ${maxReferenceImages} reference images`
              }
              type="button"
            >
              <FilePlus2 size={24} />
              <span>{`${props.referenceImages.length} / ${maxReferenceImages}`}</span>
            </button>
            <input
              ref={fileInputRef}
              accept="image/png,image/jpeg,image/webp"
              multiple
              onChange={(event) => void addReferenceFiles(event.target.files)}
              type="file"
            />
            {props.referenceImages.map((image, index) => (
              <div className="reference-thumb" key={image.id} title={image.name}>
                <button
                  className="reference-preview-button"
                  onClick={() =>
                    props.onPreviewImages(
                      props.referenceImages.map((item) => ({
                        id: item.id,
                        alt: item.name,
                        src: item.dataUrl,
                      })),
                      index,
                    )
                  }
                  type="button"
                >
                  <img alt={image.name} src={image.dataUrl} />
                </button>
                <button
                  aria-label={`Remove ${image.name}`}
                  onClick={() =>
                    props.setReferenceImages((current) => current.filter((item) => item.id !== image.id))
                  }
                  type="button"
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}

async function fileToReferenceImage(file: File): Promise<ReferenceImageInput> {
  const dataUrl = await readFileAsDataUrl(file);
  return dataUrlToReferenceImage(file.name, dataUrl, `${file.name}-${file.lastModified}`);
}

async function pathToReferenceImage(path: string): Promise<ReferenceImageInput> {
  const dataUrl = await readImageDataUrl(path);
  return dataUrlToReferenceImage(fileNameFromPath(path), dataUrl, path);
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

function isSupportedImagePath(path: string): boolean {
  return /\.(png|jpe?g|webp)$/i.test(path);
}

function fileNameFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? "reference-image";
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
