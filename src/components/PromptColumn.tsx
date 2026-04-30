import { ScrollText, Search } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { clamp } from "../utils/math";
import { PanelHeader, TreeGroup } from "./common";

export function PromptColumn({
  prompt,
  onPromptChange,
}: {
  prompt: string;
  onPromptChange: (value: string) => void;
}) {
  const editorWrapRef = useRef<HTMLDivElement>(null);
  const [previewHeight, setPreviewHeight] = useState(260);
  const [isPreviewResizing, setIsPreviewResizing] = useState(false);

  const startPreviewResize = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const bounds = editorWrapRef.current?.getBoundingClientRect();
    if (!bounds) {
      return;
    }

    event.preventDefault();
    setIsPreviewResizing(true);
    document.body.classList.add("is-panel-resizing");

    const onMove = (moveEvent: PointerEvent) => {
      const nextHeight = bounds.bottom - moveEvent.clientY;
      setPreviewHeight(clamp(nextHeight, 120, Math.max(120, bounds.height - 130)));
    };

    const onUp = () => {
      setIsPreviewResizing(false);
      document.body.classList.remove("is-panel-resizing");
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
  }, []);

  return (
    <section className="panel prompt-panel">
      <PanelHeader icon={<ScrollText size={16} />} title="Prompt Editor" />
      <div className="prompt-body">
        <aside className="prompt-library">
          <div className="search-box">
            <Search size={15} />
            <input placeholder="Search prompts..." />
          </div>
          <nav className="tree">
            <TreeGroup title="nanobanana" items={["photorealistic", "film_look"]} active="photorealistic" />
            <TreeGroup title="identity" items={["face_lock"]} />
            <TreeGroup title="gpt-image" items={["portrait", "ui_mockup"]} />
            <TreeGroup title="grok" items={["fury!"]} />
          </nav>
        </aside>
        <div className="editor-wrap" ref={editorWrapRef}>
          <div className="editor-topline">
            <strong>photorealistic</strong>
            <label className="toggle">
              <span>DSL</span>
              <input disabled type="checkbox" />
            </label>
          </div>
          <textarea
            className="prompt-textarea"
            value={prompt}
            onChange={(event) => onPromptChange(event.target.value)}
            spellCheck={false}
          />
          <div
            aria-label="Resize prompt editor and preview"
            aria-orientation="horizontal"
            className={`preview-resizer ${isPreviewResizing ? "active" : ""}`}
            onPointerDown={startPreviewResize}
            role="separator"
            tabIndex={0}
          />
          <div className="preview-box" style={{ flexBasis: previewHeight }}>
            <div className="preview-heading">
              <strong>Preview</strong>
              <span>Plain text</span>
            </div>
            <p>{prompt.trim() || "Enter a prompt to preview the final text."}</p>
          </div>
        </div>
      </div>
    </section>
  );
}
