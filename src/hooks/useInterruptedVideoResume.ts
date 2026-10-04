import { useEffect, useRef } from "react";
import type { GenerationBatch } from "../types";
import { canResumeVideoBatch } from "../utils/history";

/**
 * After history first loads, resumes Higgsfield video jobs that were still
 * running when the app last closed. Runs once per app session.
 */
export function useInterruptedVideoResume({
  batches,
  loaded,
  resumeVideoBatch,
}: {
  batches: GenerationBatch[];
  loaded: boolean;
  resumeVideoBatch: (batchId: string) => void;
}) {
  const handledRef = useRef(false);

  useEffect(() => {
    if (!loaded || handledRef.current) {
      return;
    }
    handledRef.current = true;
    batches
      .filter((batch) => batch.status === "running" && canResumeVideoBatch(batch))
      .forEach((batch) => resumeVideoBatch(batch.id));
  }, [batches, loaded, resumeVideoBatch]);
}
