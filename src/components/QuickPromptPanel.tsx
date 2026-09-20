import { ChevronLeft, ChevronRight, Plus, ScrollText, X } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";
import type { QuickPrompts } from "../hooks/useQuickPrompts";
import { PanelHeader, ToggleSwitch } from "./common";
import { PromptEditor } from "./PromptEditor";

export function QuickPromptPanel({ quick, onSave, fileDropActive }: {
  quick: QuickPrompts;
  onSave: (name: string, source: string, tags: string[]) => Promise<void>;
  fileDropActive?: boolean;
}) {
  const highlightRef = useRef<HTMLPreElement>(null);
  const [closingId, setClosingId] = useState<string | null>(null);
  const state = quick.state!;
  const tabsRef = useRef<HTMLDivElement>(null);
  const activeIndex = state.tabs.findIndex(tab => tab.id === state.activeId);
  useLayoutEffect(() => {
    const strip = tabsRef.current;
    if (!strip) return;
    const revealActive = () => {
      const tab = strip.querySelector<HTMLElement>('[aria-selected="true"]')?.parentElement;
      if (!tab) return;
      const bounds = strip.getBoundingClientRect();
      const tabBounds = tab.getBoundingClientRect();
      if (tabBounds.left < bounds.left) strip.scrollLeft += tabBounds.left - bounds.left;
      else if (tabBounds.right > bounds.right) strip.scrollLeft += tabBounds.right - bounds.right;
    };
    revealActive();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(revealActive);
    observer?.observe(strip);
    return () => observer?.disconnect();
  }, [state.activeId, state.enabled, state.tabs.length]);
  return <section className="panel prompt-panel quick-panel">
    <PanelHeader icon={<ScrollText size={16} />} title="Prompt Editor" actions={
      <div className="prompt-header-actions">
        <ToggleSwitch checked={state.enabled} label="Quick" disabled={quick.busy} onChange={quick.toggle} />
        <ToggleSwitch checked={false} label="DSL" disabled onChange={() => undefined} />
      </div>
    } />
    <fieldset className="quick-workspace" disabled={quick.busy}>
      {state.enabled ? <div className="quick-tabs-toolbar">
        <div className="quick-tab-navigation" aria-label="Quick tab navigation">
          <button type="button" className="icon-button" aria-label="Previous Quick prompt"
            title="Previous Quick prompt" disabled={activeIndex <= 0}
            onClick={() => quick.select(state.tabs[activeIndex - 1].id)}><ChevronLeft size={16} /></button>
          <button type="button" className="icon-button" aria-label="Next Quick prompt"
            title="Next Quick prompt" disabled={activeIndex >= state.tabs.length - 1}
            onClick={() => quick.select(state.tabs[activeIndex + 1].id)}><ChevronRight size={16} /></button>
        </div>
        <div className="quick-tabs" ref={tabsRef} role="tablist" aria-label="Quick prompts">
          {state.tabs.map(tab => <div className="quick-tab" key={tab.id}>
            <button type="button" role="tab" aria-selected={tab.id === state.activeId}
              aria-controls="quick-editor" id={`quick-tab-${tab.id}`}
              tabIndex={tab.id === state.activeId ? 0 : -1}
              onKeyDown={event => {
                const index = state.tabs.findIndex(t => t.id === tab.id);
                const next = event.key === "ArrowRight" ? (index + 1) % state.tabs.length
                  : event.key === "ArrowLeft" ? (index + state.tabs.length - 1) % state.tabs.length
                    : event.key === "Home" ? 0 : event.key === "End" ? state.tabs.length - 1 : -1;
                if (next < 0) return;
                event.preventDefault();
                quick.select(state.tabs[next].id);
                document.getElementById(`quick-tab-${state.tabs[next].id}`)?.focus();
              }}
              onClick={() => quick.select(tab.id)}>{`Prompt ${tab.number}`}</button>
            <button type="button" className="quick-close" aria-label={`Close Prompt ${tab.number}`}
              onClick={() => tab.source.trim() ? setClosingId(tab.id) : quick.close(tab.id)}><X size={12} /></button>
          </div>)}
        </div>
        <button type="button" className="icon-button" aria-label="Add Quick prompt" title="Add Quick prompt (up to 8)"
          disabled={state.tabs.length >= 8} onClick={quick.add}><Plus size={15} /></button>
      </div> : <div className="quick-draft-fields">
        <label className="field">Prompt name<input value={state.title} placeholder="Untitled Prompt" maxLength={512}
          onChange={event => quick.setTitle(event.target.value)} /></label>
        <label className="field">Tags<input value={state.tags} placeholder="#" maxLength={4096}
          onChange={event => quick.setTags(event.target.value)} /></label>
      </div>}
      {closingId && <div className="quick-confirm" role="alertdialog" aria-label="Close Quick prompt">
        <span>Discard this tab’s text?</span>
        <button type="button" className="secondary-button" onClick={() => setClosingId(null)}>Keep</button>
        <button type="button" className="secondary-button" onClick={() => { quick.close(closingId); setClosingId(null); }}>Discard</button>
      </div>}
      <div className={`quick-editor prompt-editor-area${fileDropActive ? " file-drop-active" : ""}`}
        id="quick-editor" role={state.enabled ? "tabpanel" : undefined}
        aria-labelledby={state.enabled ? `quick-tab-${state.activeId}` : undefined}
        data-file-drop-zone="prompt-editor"
        onKeyDownCapture={event => {
          if (!(event.target instanceof HTMLTextAreaElement) || event.nativeEvent.isComposing) return;
          if ((event.metaKey || event.ctrlKey) && !event.altKey && event.key.toLowerCase() === "z") {
            event.preventDefault();
            event.stopPropagation();
            quick.undo(event.shiftKey);
          }
        }}>
        <PromptEditor key={state.activeId} dslEnabled={false} getDraggedPromptInclude={() => null}
          highlightRef={highlightRef} onPromptChange={quick.setSource} prompt={quick.active?.source ?? ""} />
      </div>
      <div className={`quick-footer${state.pending ? " quick-draft-footer" : ""}`}>
        <span role="status" className="prompt-save-state">{quick.saveState === "saved" ? (state.pending ? "Unsaved library draft · backed up" : "Saved")
          : quick.saveState === "error" ? "Save failed" : quick.saveState === "loading" ? "Loading…" : "Saving…"}</span>
        {quick.saveState === "error" && <button type="button" className="secondary-button" onClick={quick.retry}>Retry</button>}
        {state.enabled ? <button type="button" className="secondary-button" onClick={() => quick.toggle(false)}>Save to Library…</button>
          : <>
            <button type="button" className="secondary-button" onClick={quick.returnToLibrary}>Back to Library</button>
            <button type="button" className="primary-button" onClick={() => void quick.saveToLibrary(onSave)}>Save to Library</button>
          </>}
      </div>
    </fieldset>
  </section>;
}
