import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type QuickTab = { id: string; number: number; source: string };
export type QuickState = {
  enabled: boolean;
  pending: boolean;
  activeId: string;
  title: string;
  tags: string;
  tabs: QuickTab[];
};

export function newQuickTab(tabs: QuickTab[]): QuickTab {
  return { id: crypto.randomUUID(), number: [1, 2, 3, 4, 5, 6, 7, 8].find(n => !tabs.some(t => t.number === n))!, source: "" };
}

export function useQuickPrompts(reportError: (message: string) => void) {
  const [state, setState] = useState<QuickState | null>(null);
  const current = useRef<QuickState | null>(null);
  const [saveState, setSaveState] = useState<"loading" | "saved" | "dirty" | "saving" | "error">("loading");
  const [busy, setBusy] = useState(false);
  const savingLibrary = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const queue = useRef<Promise<void>>(Promise.resolve());
  const errorRef = useRef(reportError);
  errorRef.current = reportError;

  const persist = useCallback((snapshot: QuickState) => {
    clearTimeout(timer.current);
    const operation = queue.current.catch(() => undefined).then(async () => {
      setSaveState("saving");
      await invoke("save_quick_prompts", { state: snapshot });
      if (current.current === snapshot) setSaveState("saved");
    });
    queue.current = operation;
    void operation.catch(error => {
      setSaveState("error");
      errorRef.current(`Quick draft could not be saved: ${String(error)}`);
    });
    return operation;
  }, []);

  const flush = useCallback(async () => {
    if (current.current) await persist(current.current);
  }, [persist]);

  const change = useCallback((update: (state: QuickState) => QuickState, immediate = false) => {
    if (!current.current) return;
    const next = update(current.current);
    current.current = next;
    setState(next);
    setSaveState("dirty");
    clearTimeout(timer.current);
    if (immediate) void persist(next).catch(() => undefined);
    else timer.current = setTimeout(() => void persist(next).catch(() => undefined), 500);
  }, [persist]);

  useEffect(() => {
    let alive = true;
    void invoke<QuickState>("load_quick_prompts").then(value => {
      if (!alive) return;
      current.current = value;
      setState(value);
      setSaveState("saved");
    }).catch(error => {
      if (!alive) return;
      setSaveState("error");
      errorRef.current(`Quick drafts could not be loaded: ${String(error)}`);
    });
    return () => { alive = false; clearTimeout(timer.current); };
  }, []);

  useEffect(() => {
    const blur = () => { void flush().catch(() => undefined); };
    window.addEventListener("blur", blur);
    // Prevent closing before the last keystrokes reach disk; keep the window open on failure.
    let disposed = false;
    let unlisten: (() => void) | undefined;
    try {
      void getCurrentWindow().onCloseRequested(async event => {
        event.preventDefault();
        try { await flush(); await getCurrentWindow().destroy(); } catch { /* Error is shown by persist. */ }
      }).then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(() => undefined);
    } catch { /* Browser preview has no native window. */ }
    return () => { disposed = true; unlisten?.(); window.removeEventListener("blur", blur); };
  }, [flush]);

  const setSource = useCallback((source: string) => {
    if (savingLibrary.current) return;
    change(s => ({ ...s, tabs: s.tabs.map(t => t.id === s.activeId ? { ...t, source } : t) }));
  }, [change]);

  const active = state?.tabs.find(tab => tab.id === state.activeId);
  return {
    state, active, saveState, busy, flush,
    editing: !!state && (state.enabled || state.pending),
    setSource,
    setTitle: (title: string) => change(s => ({ ...s, title })),
    setTags: (tags: string) => change(s => ({ ...s, tags })),
    toggle: (enabled: boolean) => change(s => ({ ...s, enabled, pending: !enabled }), true),
    returnToLibrary: () => change(s => ({ ...s, enabled: false, pending: false }), true),
    select: (id: string) => change(s => s.tabs.some(t => t.id === id) ? { ...s, activeId: id, title: id === s.activeId ? s.title : "", tags: id === s.activeId ? s.tags : "" } : s, true),
    add: () => change(s => {
      if (s.tabs.length >= 8) return s;
      const tab = newQuickTab(s.tabs);
      return { ...s, tabs: [...s.tabs, tab], activeId: tab.id, title: "", tags: "" };
    }, true),
    close: (id: string) => change(s => {
      let tabs = s.tabs.filter(t => t.id !== id);
      if (!tabs.length) tabs = [newQuickTab([])];
      return { ...s, tabs, activeId: s.activeId === id ? tabs[0].id : s.activeId };
    }, true),
    retry: () => { void flush().catch(() => undefined); },
    saveToLibrary: async (save: (name: string, source: string, tags: string[]) => Promise<void>) => {
      const snapshot = current.current;
      if (!snapshot || savingLibrary.current) return;
      savingLibrary.current = true;
      setBusy(true);
      try {
        await flush();
        const tab = snapshot.tabs.find(t => t.id === snapshot.activeId)!;
        await save(snapshot.title.trim() || "Untitled Prompt", tab.source,
          snapshot.tags.split(/[\s,]+/).map(t => t.replace(/^#/, "")).filter(Boolean));
        change(s => ({ ...s, pending: false, enabled: false, title: "", tags: "",
          tabs: s.tabs.map(t => t.id === tab.id ? { ...t, source: "" } : t) }), true);
      } catch (error) { errorRef.current(String(error)); }
      finally { savingLibrary.current = false; setBusy(false); }
    },
  };
}

export type QuickPrompts = ReturnType<typeof useQuickPrompts>;
