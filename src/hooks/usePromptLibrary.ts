import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  createPrompt,
  deletePrompt,
  readPrompt,
  renderPromptSource,
  rescanPromptLibrary,
  saveCurrentPromptId,
  savePrompt,
  updatePromptMetadata,
} from "../api";
import type { AppSettings, PromptListItem } from "../types";
import type { AppStatus } from "../components/common";

export type PromptSaveState = "loading" | "saved" | "dirty" | "saving" | "error";
export type PromptSortMode = "updated" | "name";

export function usePromptLibrary({
  currentPromptId,
  prompt,
  promptDslEnabled,
  setCurrentPromptId,
  setMessage,
  setPrompt,
  setStatus,
  settings,
}: {
  currentPromptId: string | null;
  prompt: string;
  promptDslEnabled: boolean;
  setCurrentPromptId: (id: string | null) => void;
  setMessage: (message: string) => void;
  setPrompt: (prompt: string) => void;
  setStatus: (status: AppStatus) => void;
  settings: AppSettings | null;
}) {
  const [items, setItems] = useState<PromptListItem[]>([]);
  const [query, setQuery] = useState("");
  const [selectedPromptId, setSelectedPromptId] = useState<string | null>(null);
  const [renderedPrompt, setRenderedPrompt] = useState("");
  const [saveState, setSaveState] = useState<PromptSaveState>("loading");
  const [sortMode, setSortMode] = useState<PromptSortMode>("updated");
  const [title, setTitle] = useState("Untitled Prompt");
  const [tagsText, setTagsText] = useState("");
  const lastSavedSourceRef = useRef("");
  const lastSavedTitleRef = useRef("Untitled Prompt");
  const lastSavedTagsRef = useRef("");
  const selectedPromptIdRef = useRef<string | null>(null);
  const didBootstrapRef = useRef(false);
  const bootstrappedDirectoryRef = useRef<string | null>(null);

  useEffect(() => {
    selectedPromptIdRef.current = selectedPromptId;
  }, [selectedPromptId]);

  const upsertItem = useCallback((item: PromptListItem) => {
    setItems((current) =>
      [item, ...current.filter((currentItem) => currentItem.id !== item.id)].sort(compareUpdated),
    );
  }, []);

  const loadPrompt = useCallback(
    async (id: string) => {
      if (!settings) {
        return;
      }
      const document = await readPrompt(settings.promptDirectory, id);
      selectedPromptIdRef.current = id;
      setSelectedPromptId(id);
      setCurrentPromptId(id);
      void saveCurrentPromptId(id).catch(() => undefined);
      lastSavedSourceRef.current = document.source;
      lastSavedTitleRef.current = document.item.name;
      lastSavedTagsRef.current = tagsToText(document.item.tags);
      setPrompt(document.source);
      setTitle(document.item.name);
      setTagsText(tagsToText(document.item.tags));
      setRenderedPrompt(document.renderedPrompt);
      setSaveState("saved");
      upsertItem(document.item);
    },
    [setCurrentPromptId, setPrompt, settings, upsertItem],
  );

  const flushCurrentPrompt = useCallback(async () => {
    if (!settings || !selectedPromptIdRef.current || saveState === "loading") {
      return;
    }
    const promptId = selectedPromptIdRef.current;
    const sourceDirty = prompt !== lastSavedSourceRef.current;
    const metadataDirty =
      title.trim() !== lastSavedTitleRef.current || tagsText.trim() !== lastSavedTagsRef.current;
    if (!sourceDirty && !metadataDirty) {
      return;
    }
    const [document, item] = await Promise.all([
      sourceDirty ? savePrompt(settings.promptDirectory, { id: promptId, source: prompt }) : null,
      metadataDirty
        ? updatePromptMetadata(settings.promptDirectory, {
            id: promptId,
            name: title,
            tags: parseTagsText(tagsText),
          })
        : null,
    ]);
    if (document) {
      lastSavedSourceRef.current = prompt;
      setRenderedPrompt(document.renderedPrompt);
    }
    const nextItem = document?.item ?? item;
    if (nextItem) {
      lastSavedTitleRef.current = nextItem.name;
      lastSavedTagsRef.current = tagsToText(nextItem.tags);
      upsertItem(nextItem);
    }
  }, [prompt, saveState, settings, tagsText, title, upsertItem]);

  const createNewPrompt = useCallback(async () => {
    if (!settings) {
      return;
    }
    await flushCurrentPrompt();
    const document = await createPrompt(settings.promptDirectory, {
      name: "Untitled Prompt",
      tags: [],
      description: "",
    });
    upsertItem(document.item);
    await loadPrompt(document.item.id);
    setStatus("ready");
    setMessage("Prompt created");
  }, [flushCurrentPrompt, loadPrompt, setMessage, setStatus, settings, upsertItem]);

  useEffect(() => {
    if (!settings) {
      return;
    }
    if (bootstrappedDirectoryRef.current !== settings.promptDirectory) {
      didBootstrapRef.current = false;
      bootstrappedDirectoryRef.current = settings.promptDirectory;
    }
    if (didBootstrapRef.current) {
      return;
    }
    didBootstrapRef.current = true;
    setSaveState("loading");
    void rescanPromptLibrary(settings.promptDirectory)
      .then(async (libraryItems) => {
        setItems(libraryItems);
        const lastOpenExists = currentPromptId
          ? libraryItems.some((item) => item.id === currentPromptId)
          : false;
        const idToOpen = lastOpenExists ? currentPromptId : libraryItems[0]?.id;
        if (idToOpen) {
          await loadPrompt(idToOpen);
          return;
        }
        await createNewPrompt();
      })
      .catch((error) => {
        setStatus("error");
        setMessage(String(error));
        setSaveState("error");
      });
  }, [createNewPrompt, currentPromptId, loadPrompt, setMessage, setStatus, settings]);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      void renderPromptSource(prompt, settings?.promptDirectory, selectedPromptId)
        .then((result) => setRenderedPrompt(promptDslEnabled ? result.renderedPrompt : prompt.trim()))
        .catch(() => setRenderedPrompt(prompt.trim()));
    }, 150);
    return () => window.clearTimeout(handle);
  }, [prompt, promptDslEnabled, selectedPromptId, settings?.promptDirectory]);

  useEffect(() => {
    if (!settings || !selectedPromptId || saveState === "loading") {
      return;
    }
    const sourceDirty = prompt !== lastSavedSourceRef.current;
    const titleDirty = title.trim() !== lastSavedTitleRef.current;
    const tagsDirty = tagsText.trim() !== lastSavedTagsRef.current;
    if (!sourceDirty) {
      setSaveState(titleDirty || tagsDirty ? "dirty" : "saved");
      return;
    }

    setSaveState("dirty");
    const promptId = selectedPromptId;
    const source = prompt;
    const handle = window.setTimeout(() => {
      setSaveState("saving");
      void savePrompt(settings.promptDirectory, { id: promptId, source })
        .then((document) => {
          if (selectedPromptIdRef.current !== promptId) {
            return;
          }
          lastSavedSourceRef.current = source;
          setRenderedPrompt(document.renderedPrompt);
          upsertItem(document.item);
          setSaveState("saved");
        })
        .catch((error) => {
          if (selectedPromptIdRef.current === promptId) {
            setSaveState("error");
            setStatus("error");
            setMessage(String(error));
          }
        });
    }, 800);

    return () => window.clearTimeout(handle);
  }, [
    prompt,
    saveState,
    selectedPromptId,
    setMessage,
    setStatus,
    settings,
    upsertItem,
  ]);

  const commitMetadata = useCallback(async () => {
    const titleDirty = title.trim() !== lastSavedTitleRef.current;
    const tagsDirty = tagsText.trim() !== lastSavedTagsRef.current;
    if (!settings || !selectedPromptId || (!titleDirty && !tagsDirty)) {
      return;
    }
    try {
      setSaveState("saving");
      const item = await updatePromptMetadata(settings.promptDirectory, {
        id: selectedPromptId,
        name: title,
        tags: parseTagsText(tagsText),
      });
      lastSavedTitleRef.current = item.name;
      lastSavedTagsRef.current = tagsToText(item.tags);
      setTitle(item.name);
      setTagsText(tagsToText(item.tags));
      upsertItem(item);
      setSaveState("saved");
    } catch (error) {
      setSaveState("error");
      setStatus("error");
      setMessage(String(error));
    }
  }, [selectedPromptId, setMessage, setStatus, settings, tagsText, title, upsertItem]);

  const filteredItems = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const filtered = needle
      ? items.filter(
          (item) =>
            item.name.toLowerCase().includes(needle) ||
            item.description.toLowerCase().includes(needle) ||
            item.tags.some((tag) => tag.toLowerCase().includes(needle)),
        )
      : items;
    return [...filtered].sort(sortMode === "name" ? compareName : compareUpdated);
  }, [items, query, sortMode]);

  const selectPrompt = useCallback(
    async (id: string) => {
      try {
        await flushCurrentPrompt();
        await loadPrompt(id);
      } catch (error) {
        setStatus("error");
        setMessage(String(error));
      }
    },
    [flushCurrentPrompt, loadPrompt, setMessage, setStatus],
  );

  const deletePromptById = useCallback(async (id?: string) => {
    const promptId = id ?? selectedPromptId;
    if (!settings || !promptId) {
      return;
    }
    try {
      if (promptId === selectedPromptId) {
        await flushCurrentPrompt();
      }
      await deletePrompt(settings.promptDirectory, promptId);
      const nextItems = items.filter((item) => item.id !== promptId);
      setItems(nextItems);
      if (promptId === selectedPromptId) {
        const nextId = nextItems[0]?.id ?? null;
        if (nextId) {
          await loadPrompt(nextId);
        } else {
          await createNewPrompt();
        }
      }
      setStatus("ready");
      setMessage("Prompt deleted");
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [
    createNewPrompt,
    flushCurrentPrompt,
    items,
    loadPrompt,
    selectedPromptId,
    setMessage,
    setStatus,
    settings,
  ]);

  const refreshPrompts = useCallback(async () => {
    if (!settings) {
      return;
    }
    try {
      const nextItems = await rescanPromptLibrary(settings.promptDirectory);
      setItems(nextItems);
      if (selectedPromptId && !nextItems.some((item) => item.id === selectedPromptId)) {
        const nextId = nextItems[0]?.id;
        if (nextId) {
          await loadPrompt(nextId);
        } else {
          await createNewPrompt();
        }
      }
      setStatus("ready");
      setMessage("Prompt library refreshed");
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [createNewPrompt, loadPrompt, selectedPromptId, setMessage, setStatus, settings]);

  return {
    createNewPrompt: () => void createNewPrompt().catch((error) => {
      setStatus("error");
      setMessage(String(error));
    }),
    deletePromptById,
    filteredItems,
    query,
    refreshPrompts,
    renderedPrompt,
    saveState,
    selectedPromptId,
    selectPrompt,
    commitMetadata,
    setQuery,
    setSortMode,
    setTagsText,
    setTitle,
    sortMode,
    tagsText,
    title,
  };
}

function parseTagsText(value: string) {
  return value
    .split(/[\s,]+/)
    .map((tag) => tag.trim().replace(/^#/, ""))
    .filter(Boolean);
}

function tagsToText(tags: string[]) {
  return tags.map((tag) => `#${tag}`).join(" ");
}

function compareUpdated(left: PromptListItem, right: PromptListItem) {
  return (
    (right.updatedAt ?? "").localeCompare(left.updatedAt ?? "") || left.name.localeCompare(right.name)
  );
}

function compareName(left: PromptListItem, right: PromptListItem) {
  return left.name.localeCompare(right.name) || (right.updatedAt ?? "").localeCompare(left.updatedAt ?? "");
}
