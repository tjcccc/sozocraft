import { Film, SlidersHorizontal, Sparkles } from "lucide-react";
import type { CSSProperties, Dispatch, SetStateAction } from "react";
import grokIconUrl from "../assets/grok.svg";
import { VIDEO_PROVIDER_IDS, getVideoProviderConfig } from "../models/videoProviders";
import type { VideoModelConfig } from "../models/videoProviders";
import type {
  ReferenceImageInput,
  VideoInputImageRole,
  VideoProviderId,
} from "../types";
import { Field, PanelHeader, ToggleSwitch } from "./common";
import type { LightboxImage } from "./ImageLightbox";
import { ReferenceImagesField } from "./ReferenceImagesField";

export function VideoGenerationPanel({
  allowedDurations,
  aspectRatio,
  duration,
  fileDropActive,
  generateAudio,
  inputImages,
  inputImageRoles,
  model,
  modelConfig,
  onPreviewImages,
  provider,
  providerConfig,
  resolution,
  setAspectRatio,
  setDuration,
  setGenerateAudio,
  setInputImages,
  setInputImageRole,
  setModel,
  setProvider,
  setResolution,
}: {
  allowedDurations: readonly number[];
  aspectRatio: string;
  duration: number;
  fileDropActive?: boolean;
  generateAudio: boolean;
  inputImages: ReferenceImageInput[];
  inputImageRoles: Record<string, VideoInputImageRole>;
  model: string;
  modelConfig: VideoModelConfig;
  onPreviewImages: (images: LightboxImage[], index: number) => void;
  provider: VideoProviderId;
  providerConfig: ReturnType<typeof getVideoProviderConfig>;
  resolution: string;
  setAspectRatio: (value: string) => void;
  setDuration: (value: number) => void;
  setGenerateAudio: (value: boolean) => void;
  setInputImages: Dispatch<SetStateAction<ReferenceImageInput[]>>;
  setInputImageRole: (imageId: string, role: VideoInputImageRole) => void;
  setModel: (value: string) => void;
  setProvider: (value: VideoProviderId) => void;
  setResolution: (value: string) => void;
}) {
  const minDuration = allowedDurations[0];
  const maxDuration = allowedDurations[allowedDurations.length - 1];
  const durationStep = allowedDurations.length > 1
    ? allowedDurations[1] - allowedDurations[0]
    : 1;
  const progress = maxDuration === minDuration
    ? 100
    : ((duration - minDuration) / (maxDuration - minDuration)) * 100;
  const supportsEndFrame = provider !== "grok-imagine";
  const frameRoleDisabled = inputImages.length > 2
    || (provider === "grok-imagine" && inputImages.length > 1);
  const roleOptions = [
    { label: "Reference", value: "reference" as const },
    {
      disabled: frameRoleDisabled,
      label: "Start frame",
      title: frameRoleDisabled
        ? "Start-frame mode supports one image, or a two-frame pair when available."
        : undefined,
      value: "starting" as const,
    },
    ...(supportsEndFrame
      ? [{
          disabled: inputImages.length !== 2,
          label: "End frame",
          title: inputImages.length === 2
            ? undefined
            : "Add exactly two images to assign start and end frames.",
          value: "ending" as const,
        }]
      : []),
  ];

  return (
    <section
      className={`panel generation-panel video-generation-panel${fileDropActive ? " file-drop-active" : ""}`}
      data-file-drop-zone="generation"
    >
      <PanelHeader icon={<SlidersHorizontal size={16} />} title="Video Generation" />
      <div className="tabs video-tabs">
        {VIDEO_PROVIDER_IDS.map((providerId) => {
          const config = getVideoProviderConfig(providerId);
          return (
            <button
              aria-label={config.label}
              className={provider === providerId ? "active" : ""}
              key={providerId}
              onClick={() => setProvider(providerId)}
              type="button"
            >
              <ProviderTabIcon provider={providerId} />
              <span className="provider-tab-label">{config.label}</span>
            </button>
          );
        })}
      </div>
      <div className="form-grid">
        <Field className="field-full" label="Model">
          <select
            disabled={providerConfig.models.length === 1}
            value={model}
            onChange={(event) => setModel(event.target.value)}
          >
            {providerConfig.models.map((option) => (
              <option key={option.id} value={option.id}>{option.productName}</option>
            ))}
          </select>
        </Field>
        <Field className="field-full video-duration-field" label="Duration">
          <div className="video-duration-control">
            <output htmlFor="video-duration">{duration}s</output>
            <input
              aria-label="Duration"
              disabled={allowedDurations.length === 1}
              id="video-duration"
              max={maxDuration}
              min={minDuration}
              onChange={(event) => setDuration(Number(event.target.value))}
              step={durationStep}
              style={{ "--video-duration-progress": `${progress}%` } as CSSProperties}
              type="range"
              value={duration}
            />
          </div>
          <div aria-hidden="true" className="video-duration-limits">
            <span>{minDuration}s</span>
            <span>{maxDuration}s</span>
          </div>
        </Field>
        <Field label="Aspect Ratio">
          <select value={aspectRatio} onChange={(event) => setAspectRatio(event.target.value)}>
            {providerConfig.aspectRatios.map((ratio) => (
              <option key={ratio} value={ratio}>{ratio}</option>
            ))}
          </select>
        </Field>
        <Field label="Resolution">
          <select value={resolution} onChange={(event) => setResolution(event.target.value)}>
            {modelConfig.resolutions.map((value) => (
              <option key={value} value={value}>{value}</option>
            ))}
          </select>
        </Field>
        {providerConfig.supportsAudioControl ? (
          <div className="generation-toggle-row field-full">
            <ToggleSwitch
              checked={generateAudio}
              label="Generate audio"
              onChange={setGenerateAudio}
            />
          </div>
        ) : null}
      </div>
      <ReferenceImagesField
        acceptedMimeTypes={
          provider === "google-veo" ? ["image/png", "image/jpeg"] : undefined
        }
        fileDropActive={fileDropActive}
        images={inputImages}
        imageRoles={inputImageRoles}
        itemLabel="input image"
        label="Input Images"
        maxImages={providerConfig.maxInputImages}
        onImageRoleChange={setInputImageRole}
        onPreviewImages={onPreviewImages}
        roleOptions={roleOptions}
        setImages={setInputImages}
      />
      {providerConfig.audioAlwaysGenerated ? (
        <p className="video-provider-note">Veo generates synchronized audio automatically.</p>
      ) : null}
      <p className="video-stop-note">
        Stop ends local monitoring only; the provider may continue processing and charging.
      </p>
    </section>
  );
}

function ProviderTabIcon({ provider }: { provider: VideoProviderId }) {
  if (provider === "grok-imagine") {
    return <img alt="" className="provider-tab-icon" src={grokIconUrl} />;
  }
  const Icon = provider === "seedance" ? Film : Sparkles;
  return <Icon aria-hidden="true" className="provider-tab-icon" />;
}
