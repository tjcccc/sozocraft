import { FilePlus2, SlidersHorizontal, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { AppSettings, ReferenceImageInput } from "../types";
import type { LightboxImage } from "./ImageLightbox";
import {
  GPT_IMAGE_PLATFORM_MODELS,
  IMAGE_PROVIDER_IDS,
  getImageSizeDisplayName,
  getProviderControlConfig,
  getProviderConfig,
  getProviderModels,
  normalizeProviderOptions,
} from "../models/imageProviders";
import type { ImageProviderApiPlatform, ImageProviderId } from "../models/imageProviders";
import { fileToReferenceImage } from "../utils/referenceImages";
import { Field, PanelHeader, ToggleSwitch } from "./common";
import grokIconUrl from "../assets/grok.svg";
import nanoBananaIconUrl from "../assets/nanobanana-color.svg";
import openaiIconUrl from "../assets/openai.svg";

const PROVIDER_ICON_URLS: Record<(typeof IMAGE_PROVIDER_IDS)[number], string> = {
  "nano-banana": nanoBananaIconUrl,
  "gpt-image": openaiIconUrl,
  "grok-imagine": grokIconUrl,
};

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
  unlimited: boolean;
  setUnlimited: (value: boolean) => void;
  referenceImages: ReferenceImageInput[];
  setReferenceImages: Dispatch<SetStateAction<ReferenceImageInput[]>>;
  fileDropActive?: boolean;
  onPreviewImages: (images: LightboxImage[], index: number) => void;
}) {
  const activePlatform = settingsPlatformForProvider(props.settings, props.settings.defaultProvider);
  const controlConfig = getProviderControlConfig(
    props.settings.defaultProvider,
    props.settings.defaultModel,
    activePlatform,
  );
  const aspectRatios = controlConfig.aspectRatios;
  const imageSizes = controlConfig.imageSizes;
  const thinkingLevels = controlConfig.thinkingLevels;
  const providerModels = getProviderModels(
    props.settings.defaultProvider,
    activePlatform,
  );
  const maxReferenceImages = controlConfig.maxReferenceImages;
  const supportsUnlimited =
    activePlatform === "higgsfield" &&
    props.settings.defaultModel === "nano_banana_2" &&
    props.imageSize.toLowerCase() !== "4k";
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [modelByProvider, setModelByProvider] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      IMAGE_PROVIDER_IDS.map((provider) => [provider, getProviderConfig(provider).defaults.model]),
    ),
  );
  const [isReferenceDropActive, setIsReferenceDropActive] = useState(false);
  const canAddReferenceImages = props.referenceImages.length < maxReferenceImages;

  useEffect(() => {
    setModelByProvider((current) => ({
      ...current,
      [props.settings.defaultProvider]: props.settings.defaultModel,
    }));
  }, [props.settings.defaultModel, props.settings.defaultProvider]);

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

  return (
    <section
      className={`panel generation-panel${props.fileDropActive ? " file-drop-active" : ""}`}
      data-file-drop-zone="generation"
    >
      <PanelHeader icon={<SlidersHorizontal size={16} />} title="Image Generation" />
      <div className="tabs">
        {IMAGE_PROVIDER_IDS.map((provider) => {
          const config = getProviderConfig(provider);

          return (
            <button
              aria-label={config.label}
              className={props.settings.defaultProvider === provider ? "active" : ""}
              key={provider}
              onClick={() => {
                const platform = settingsPlatformForProvider(props.settings, provider);
                const fallbackModel =
                  provider === "gpt-image"
                    ? GPT_IMAGE_PLATFORM_MODELS[props.settings.openaiApiPlatform]
                    : getProviderModels(provider, platform)[0]?.id ?? config.defaults.model;
                const next = normalizeProviderOptions(provider, modelByProvider[provider] ?? fallbackModel, {
                  aspectRatio: props.aspectRatio,
                  imageSize: props.imageSize,
                  quality: props.quality,
                  thinkingLevel: props.thinkingLevel,
                }, platform);
                props.setSettings({
                  ...props.settings,
                  defaultProvider: provider,
                  defaultModel: next.model,
                });
              }}
              title={config.label}
              type="button"
            >
              <img alt="" className="provider-tab-icon" src={PROVIDER_ICON_URLS[provider]} />
              <span className="provider-tab-label">{config.label}</span>
            </button>
          );
        })}
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
              }, activePlatform);
              setModelByProvider((current) => ({
                ...current,
                [props.settings.defaultProvider]: next.model,
              }));
              props.setSettings({ ...props.settings, defaultModel: nextModel });
              props.setAspectRatio(next.aspectRatio);
              props.setImageSize(next.imageSize);
              props.setQuality(next.quality);
              props.setThinkingLevel(next.thinkingLevel);
              props.setReferenceImages((current) => current.slice(0, next.maxReferenceImages));
            }}
          >
            {providerModels.map((model) => (
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
                  {getImageSizeDisplayName(props.settings.defaultProvider, size)}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
        {props.settings.defaultProvider === "nano-banana" && activePlatform !== "higgsfield" ? (
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
        {controlConfig.qualityLevels ? (
          <Field label={controlConfig.qualityLabel}>
            <select value={props.quality} onChange={(event) => props.setQuality(event.target.value)}>
              {controlConfig.qualityLevels.map((level) => (
                <option key={level} value={level}>
                  {qualityLevelDisplayName(level)}
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
        {supportsUnlimited ? (
          <div className="generation-toggle-row">
            <ToggleSwitch
              checked={props.unlimited}
              label="Unlimited"
              onChange={props.setUnlimited}
            />
          </div>
        ) : null}
      </div>
      {maxReferenceImages > 0 ? (
        <div className="reference-field">
          <span className="reference-label">Reference Images</span>
          <div className="reference-images">
            <button
              className={`reference-add ${
                isReferenceDropActive || props.fileDropActive ? "drop-active" : ""
              }`}
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

function qualityLevelDisplayName(level: string) {
  if (level === "std") {
    return "Standard";
  }
  if (level === "pro") {
    return "Quality";
  }
  return level;
}

function settingsPlatformForProvider(
  settings: AppSettings,
  provider: ImageProviderId,
): ImageProviderApiPlatform {
  if (provider === "nano-banana") {
    return settings.nanoBananaApiPlatform;
  }
  if (provider === "grok-imagine") {
    return settings.grokApiPlatform;
  }
  return settings.openaiApiPlatform;
}
