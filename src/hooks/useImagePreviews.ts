import { useEffect, useMemo, useState } from "react";
import { readImageDataUrl } from "../api";
import type { GenerationBatch } from "../types";

export function useImagePreviews({
  expandedBatch,
  previewBatch,
}: {
  expandedBatch?: GenerationBatch;
  previewBatch?: GenerationBatch;
}) {
  const [imageDataUrls, setImageDataUrls] = useState<Record<string, string>>({});
  const [failedImagePaths, setFailedImagePaths] = useState<Set<string>>(new Set());

  const imagesToLoad = useMemo(() => {
    const seen = new Set<string>();
    for (const img of previewBatch?.images ?? []) seen.add(img.path);
    for (const img of expandedBatch?.images ?? []) seen.add(img.path);
    return [...seen];
  }, [previewBatch, expandedBatch]);

  useEffect(() => {
    for (const path of imagesToLoad) {
      if (imageDataUrls[path] || failedImagePaths.has(path)) {
        continue;
      }
      void readImageDataUrl(path)
        .then((url) => setImageDataUrls((cur) => ({ ...cur, [path]: url })))
        .catch(() => setFailedImagePaths((cur) => new Set([...cur, path])));
    }
  }, [imagesToLoad, imageDataUrls, failedImagePaths]);

  return { failedImagePaths, imageDataUrls };
}
