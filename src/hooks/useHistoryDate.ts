import { useMemo, useState } from "react";
import type { GenerationBatch } from "../types";
import { localDateString } from "../utils/dates";

export function useHistoryDate(batches: GenerationBatch[]) {
  const [historyDate, setHistoryDate] = useState<string>(() =>
    localDateString(new Date().toISOString()),
  );
  const filteredBatches = useMemo(
    () => batches.filter((batch) => localDateString(batch.createdAt) === historyDate),
    [batches, historyDate],
  );

  return { filteredBatches, historyDate, setHistoryDate };
}
