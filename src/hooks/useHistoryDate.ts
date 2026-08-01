import { useMemo, useState } from "react";
import type { GenerationBatch, GenerationMediaType } from "../types";
import { localDateString } from "../utils/dates";

export function useHistoryDate(batches: GenerationBatch[], mediaType: GenerationMediaType = "image") {
  const [historyDate, setHistoryDate] = useState<string>(() =>
    localDateString(new Date().toISOString()),
  );
  const filteredBatches = useMemo(
    () => batches.filter((batch) => batch.mediaType === mediaType && localDateString(batch.createdAt) === historyDate),
    [batches, historyDate, mediaType],
  );

  return { filteredBatches, historyDate, setHistoryDate };
}
