import { ChevronLeft, ChevronRight, X } from "lucide-react";
import { useEffect } from "react";

export type LightboxImage = {
  id: string;
  alt: string;
  src: string;
};

export type LightboxState = {
  images: LightboxImage[];
  index: number;
};

export function ImageLightbox({
  state,
  onClose,
  onIndexChange,
}: {
  state: LightboxState;
  onClose: () => void;
  onIndexChange: (index: number) => void;
}) {
  const image = state.images[state.index];
  const canNavigate = state.images.length > 1;

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
      if (event.key === "ArrowLeft" && canNavigate) {
        onIndexChange(wrapIndex(state.index - 1, state.images.length));
      }
      if (event.key === "ArrowRight" && canNavigate) {
        onIndexChange(wrapIndex(state.index + 1, state.images.length));
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [canNavigate, onClose, onIndexChange, state.images.length, state.index]);

  if (!image) {
    return null;
  }

  return (
    <div className="image-lightbox" onClick={onClose} role="dialog" aria-modal="true">
      <button className="lightbox-close" onClick={onClose} title="Close" type="button">
        <X size={20} />
      </button>
      {canNavigate ? (
        <button
          className="lightbox-nav lightbox-prev"
          onClick={(event) => {
            event.stopPropagation();
            onIndexChange(wrapIndex(state.index - 1, state.images.length));
          }}
          title="Previous image"
          type="button"
        >
          <ChevronLeft size={34} />
        </button>
      ) : null}
      <img
        alt={image.alt}
        className="lightbox-image"
        onClick={(event) => event.stopPropagation()}
        src={image.src}
      />
      {canNavigate ? (
        <button
          className="lightbox-nav lightbox-next"
          onClick={(event) => {
            event.stopPropagation();
            onIndexChange(wrapIndex(state.index + 1, state.images.length));
          }}
          title="Next image"
          type="button"
        >
          <ChevronRight size={34} />
        </button>
      ) : null}
    </div>
  );
}

function wrapIndex(index: number, length: number): number {
  return (index + length) % length;
}
