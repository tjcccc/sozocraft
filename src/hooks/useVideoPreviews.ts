import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { prepareVideoPreview } from "../api";
import type { GenerationBatch } from "../types";

export function useVideoPreviews({
  expandedBatch,
  previewBatch,
}: {
  expandedBatch?: GenerationBatch;
  previewBatch?: GenerationBatch;
}) {
  const [videoUrls, setVideoUrls] = useState<Record<string, string>>({});
  const [failedVideoPaths, setFailedVideoPaths] = useState<Set<string>>(new Set());

  const videosToLoad = useMemo(() => {
    const seen = new Set<string>();
    for (const video of previewBatch?.videos ?? []) seen.add(video.path);
    for (const video of expandedBatch?.videos ?? []) seen.add(video.path);
    return [...seen];
  }, [expandedBatch, previewBatch]);

  useEffect(() => {
    for (const path of videosToLoad) {
      if (videoUrls[path] || failedVideoPaths.has(path)) {
        continue;
      }
      void prepareVideoPreview(path)
        .then((allowedPath) => {
          setVideoUrls((current) => ({ ...current, [path]: convertFileSrc(allowedPath) }));
        })
        .catch(() => setFailedVideoPaths((current) => new Set([...current, path])));
    }
  }, [failedVideoPaths, videoUrls, videosToLoad]);

  return { failedVideoPaths, videoUrls };
}
