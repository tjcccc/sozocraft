import { FilePlus2, SlidersHorizontal, X } from "lucide-react";
import { useRef } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { AppSettings, ReferenceImageInput } from "../types";
import {
  GEMINI_IMAGE_MODEL_IDS,
  getGeminiImageModelConfig,
  normalizeGeminiImageOptions,
} from "../models/geminiImageModels";
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
  seed: string;
  setSeed: (value: string) => void;
  thinkingLevel: string;
  setThinkingLevel: (value: string) => void;
  referenceImages: ReferenceImageInput[];
  setReferenceImages: Dispatch<SetStateAction<ReferenceImageInput[]>>;
}) {
  const modelConfig = getGeminiImageModelConfig(props.settings.defaultModel);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const canAddReferenceImages = props.referenceImages.length < modelConfig.maxReferenceImages;

  async function addReferenceFiles(files: FileList | null) {
    if (!files || files.length === 0) {
      return;
    }

    const remaining = modelConfig.maxReferenceImages - props.referenceImages.length;
    const nextFiles = [...files]
      .filter((file) => file.type.startsWith("image/"))
      .slice(0, Math.max(0, remaining));
    const nextImages = await Promise.all(nextFiles.map(fileToReferenceImage));

    props.setReferenceImages((current) => [...current, ...nextImages].slice(0, modelConfig.maxReferenceImages));
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  }

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
            onChange={(event) => {
              const nextModel = event.target.value;
              const nextConfig = getGeminiImageModelConfig(nextModel);
              const next = normalizeGeminiImageOptions(nextModel, {
                aspectRatio: props.aspectRatio,
                imageSize: props.imageSize,
                thinkingLevel: props.thinkingLevel,
              });
              props.setSettings({ ...props.settings, defaultModel: nextModel });
              props.setAspectRatio(next.aspectRatio);
              props.setImageSize(next.imageSize);
              props.setThinkingLevel(next.thinkingLevel);
              props.setReferenceImages((current) => current.slice(0, nextConfig.maxReferenceImages));
            }}
          >
            {GEMINI_IMAGE_MODEL_IDS.map((model) => (
              <option key={model} value={model}>
                {getGeminiImageModelConfig(model).productName}
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
        {modelConfig.thinkingLevels ? (
          <Field label="Thinking">
            <select
              value={props.thinkingLevel}
              onChange={(event) => props.setThinkingLevel(event.target.value)}
            >
              {modelConfig.thinkingLevels.map((level) => (
                <option key={level} value={level}>
                  {level[0].toUpperCase() + level.slice(1)}
                </option>
              ))}
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
      <div className="reference-field">
        <span className="reference-label">Reference Images</span>
        <div className="reference-images">
          <button
            className="reference-add"
            disabled={!canAddReferenceImages}
            onClick={() => fileInputRef.current?.click()}
            title={
              canAddReferenceImages
                ? "Add reference image"
                : `Maximum ${modelConfig.maxReferenceImages} reference images`
            }
            type="button"
          >
            <FilePlus2 size={24} />
            <span>{`${props.referenceImages.length} / ${modelConfig.maxReferenceImages}`}</span>
          </button>
          <input
            ref={fileInputRef}
            accept="image/png,image/jpeg,image/webp"
            multiple
            onChange={(event) => void addReferenceFiles(event.target.files)}
            type="file"
          />
          {props.referenceImages.map((image) => (
            <div className="reference-thumb" key={image.id} title={image.name}>
              <img alt={image.name} src={image.dataUrl} />
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
    </section>
  );
}

async function fileToReferenceImage(file: File): Promise<ReferenceImageInput> {
  const dataUrl = await readFileAsDataUrl(file);
  const base64Marker = ";base64,";
  const base64Index = dataUrl.indexOf(base64Marker);
  const data = base64Index >= 0 ? dataUrl.slice(base64Index + base64Marker.length) : "";

  return {
    id: `${file.name}-${file.lastModified}-${newReferenceImageId()}`,
    name: file.name,
    mimeType: file.type || "image/png",
    data,
    dataUrl,
  };
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
