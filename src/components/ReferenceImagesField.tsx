import {
  Check,
  Ellipsis,
  FilePlus2,
  Image as ImageIcon,
  PanelLeft,
  PanelRight,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { ReferenceImageInput, VideoInputImageRole } from "../types";
import { fileToReferenceImage } from "../utils/referenceImages";
import type { LightboxImage } from "./ImageLightbox";

const DEFAULT_IMAGE_MIME_TYPES = ["image/png", "image/jpeg", "image/webp"] as const;

export type ImageRoleOption = {
  disabled?: boolean;
  label: string;
  title?: string;
  value: VideoInputImageRole;
};

export function ReferenceImagesField({
  acceptedMimeTypes = DEFAULT_IMAGE_MIME_TYPES,
  fileDropActive,
  images,
  imageRoles,
  itemLabel,
  label,
  maxImages,
  onImageRoleChange,
  onPreviewImages,
  roleOptions,
  setImages,
}: {
  acceptedMimeTypes?: readonly string[];
  fileDropActive?: boolean;
  images: ReferenceImageInput[];
  imageRoles?: Record<string, VideoInputImageRole>;
  itemLabel: string;
  label: string;
  maxImages: number;
  onImageRoleChange?: (imageId: string, role: VideoInputImageRole) => void;
  onPreviewImages: (images: LightboxImage[], index: number) => void;
  roleOptions?: readonly ImageRoleOption[];
  setImages: Dispatch<SetStateAction<ReferenceImageInput[]>>;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isDropActive, setIsDropActive] = useState(false);
  const [openRoleMenuId, setOpenRoleMenuId] = useState<string | null>(null);
  const canAddImages = images.length < maxImages;
  const supportedMimeTypes = new Set(acceptedMimeTypes.map((value) => value.toLowerCase()));

  useEffect(() => {
    if (!openRoleMenuId) {
      return;
    }

    const closeRoleMenuOutside = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element) || !target.closest(".video-image-role-control")) {
        setOpenRoleMenuId(null);
      }
    };

    document.addEventListener("pointerdown", closeRoleMenuOutside);
    return () => document.removeEventListener("pointerdown", closeRoleMenuOutside);
  }, [openRoleMenuId]);

  async function addFiles(files: FileList | null) {
    if (!files || files.length === 0) {
      return;
    }

    const remaining = maxImages - images.length;
    const nextFiles = [...files]
      .filter((file) => supportedMimeTypes.has(file.type.toLowerCase()))
      .slice(0, Math.max(0, remaining));
    const nextImages = await Promise.all(nextFiles.map(fileToReferenceImage));

    setImages((current) => [...current, ...nextImages].slice(0, maxImages));
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  }

  return (
    <div className="reference-field">
      <span className="reference-label">{label}</span>
      <div className="reference-images">
        <button
          aria-label={`Add ${itemLabel}`}
          className={`reference-add ${isDropActive || fileDropActive ? "drop-active" : ""}`}
          disabled={!canAddImages}
          onDragEnter={(event) => {
            if (!canAddImages) {
              return;
            }
            event.preventDefault();
            setIsDropActive(true);
          }}
          onDragLeave={() => setIsDropActive(false)}
          onDragOver={(event) => {
            if (!canAddImages) {
              return;
            }
            event.preventDefault();
            event.dataTransfer.dropEffect = "copy";
            setIsDropActive(true);
          }}
          onDrop={(event) => {
            if (!canAddImages) {
              return;
            }
            event.preventDefault();
            setIsDropActive(false);
            void addFiles(event.dataTransfer.files);
          }}
          onClick={() => fileInputRef.current?.click()}
          title={
            canAddImages
              ? `Add ${itemLabel}`
              : `Maximum ${maxImages} ${itemLabel}${maxImages === 1 ? "" : "s"}`
          }
          type="button"
        >
          <FilePlus2 size={24} />
          <span>{`${images.length} / ${maxImages}`}</span>
        </button>
        <input
          aria-label={`Choose ${itemLabel}`}
          ref={fileInputRef}
          accept={acceptedMimeTypes.join(",")}
          multiple={maxImages > 1}
          onChange={(event) => void addFiles(event.target.files)}
          type="file"
        />
        {images.map((image, index) => {
          const role = imageRoles?.[image.id] ?? "reference";
          const hasRoleMenu = Boolean(
            !image.assetId && roleOptions?.length && onImageRoleChange,
          );
          return (
            <div
              className={`reference-thumb${hasRoleMenu ? " has-role-menu" : ""}${openRoleMenuId === image.id ? " role-menu-open" : ""}`}
              key={image.id}
              title={image.name}
            >
              {image.assetId ? (
                <div className="reference-asset-placeholder" title={image.assetId}>
                  <ImageIcon aria-hidden="true" size={22} />
                  <span>Ark asset</span>
                </div>
              ) : (
                <button
                  className="reference-preview-button"
                  onClick={() => {
                    const previewable = images.filter((item) => !item.assetId);
                    const previewIndex = previewable.findIndex((item) => item.id === image.id);
                    onPreviewImages(
                      previewable.map((item) => ({
                        id: item.id,
                        alt: item.name,
                        src: item.dataUrl,
                      })),
                      previewIndex,
                    );
                  }}
                  type="button"
                >
                  <img alt={image.name} src={image.dataUrl} />
                </button>
              )}
              <button
                aria-label={`Remove ${image.name}`}
                className="reference-remove-button"
                onClick={() =>
                  setImages((current) => current.filter((item) => item.id !== image.id))
                }
                type="button"
              >
                <X size={12} />
              </button>
              {hasRoleMenu ? (
                <div
                  className="video-image-role-control"
                  onBlur={(event) => {
                    if (!event.currentTarget.contains(event.relatedTarget)) {
                      setOpenRoleMenuId(null);
                    }
                  }}
                >
                  <button
                    aria-expanded={openRoleMenuId === image.id}
                    aria-haspopup="menu"
                    aria-label={`Set use for ${image.name}`}
                    className="video-image-role-trigger"
                    onClick={() =>
                      setOpenRoleMenuId((current) => current === image.id ? null : image.id)
                    }
                    type="button"
                  >
                    <Ellipsis size={16} />
                  </button>
                  {openRoleMenuId === image.id ? (
                    <div
                      aria-label={`Use ${image.name} as`}
                      className="video-image-role-menu"
                      role="menu"
                    >
                      <span>Use as</span>
                      {roleOptions?.map((option) => (
                        <button
                          aria-checked={role === option.value}
                          disabled={option.disabled}
                          key={option.value}
                          onClick={() => {
                            onImageRoleChange?.(image.id, option.value);
                            setOpenRoleMenuId(null);
                          }}
                          role="menuitemradio"
                          title={option.title}
                          type="button"
                        >
                          <RoleIcon role={option.value} />
                          <span>{option.label}</span>
                          {role === option.value ? (
                            <Check aria-hidden="true" size={14} />
                          ) : null}
                        </button>
                      ))}
                    </div>
                  ) : null}
                </div>
              ) : null}
              {role === "starting" || role === "ending" ? (
                <span className="video-image-role-badge">
                  {role === "starting" ? "Start" : "End"}
                </span>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function RoleIcon({ role }: { role: VideoInputImageRole }) {
  if (role === "starting") {
    return <PanelLeft aria-hidden="true" size={15} />;
  }
  if (role === "ending") {
    return <PanelRight aria-hidden="true" size={15} />;
  }
  return <ImageIcon aria-hidden="true" size={15} />;
}
