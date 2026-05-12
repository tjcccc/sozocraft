import {
  ArrowDownAZ,
  ChevronDown,
  ChevronRight,
  Clock3,
  Download,
  Eye,
  EyeOff,
  FilePlus2,
  PenLine,
  ScrollText,
  Search,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, ReactNode, RefObject } from "react";
import type { PromptListItem, PromptPreviewPlacement } from "../types";
import { clamp } from "../utils/math";
import { PanelHeader, ToggleSwitch } from "./common";
import type { PromptSaveState, PromptSortMode } from "../hooks/usePromptLibrary";

type TagNode = {
  children: Map<string, TagNode>;
  prompts: PromptListItem[];
};

export function PromptColumn({
  items,
  prompt,
  query,
  renderedPrompt,
  saveState,
  selectedPromptId,
  sortMode,
  tagsText,
  title,
  dslEnabled,
  previewPlacement,
  setDslEnabled,
  setPreviewPlacement,
  setQuery,
  setSortMode,
  setTagsText,
  setTitle,
  onCommitMetadata,
  onCreatePrompt,
  onDeletePrompt,
  onPromptIncludeDragEnd,
  onPromptIncludeDragStart,
  onPromptIncludeDropHandled,
  onRenameTag,
  defaultExportPath,
  fileDropActive,
  onExportRenderedPrompt,
  onPromptChange,
  onSelectPrompt,
}: {
  items: PromptListItem[];
  prompt: string;
  query: string;
  renderedPrompt: string;
  saveState: PromptSaveState;
  selectedPromptId: string | null;
  sortMode: PromptSortMode;
  tagsText: string;
  title: string;
  dslEnabled: boolean;
  previewPlacement: PromptPreviewPlacement;
  setDslEnabled: (enabled: boolean) => void;
  setPreviewPlacement: (placement: PromptPreviewPlacement) => void;
  setQuery: (value: string) => void;
  setSortMode: (value: PromptSortMode) => void;
  setTagsText: (value: string) => void;
  setTitle: (value: string) => void;
  onCommitMetadata: () => void;
  onCreatePrompt: () => void;
  onDeletePrompt: (id?: string) => void;
  onPromptIncludeDragEnd?: () => void;
  onPromptIncludeDragStart?: (token: string) => void;
  onPromptIncludeDropHandled?: () => void;
  onRenameTag: (oldTagPath: string, newTagPath: string) => void;
  defaultExportPath: string;
  fileDropActive?: boolean;
  onExportRenderedPrompt: (outputPath: string) => void;
  onPromptChange: (value: string) => void;
  onSelectPrompt: (id: string) => void;
}) {
  const editorWrapRef = useRef<HTMLDivElement>(null);
  const highlightRef = useRef<HTMLPreElement>(null);
  const promptBodyRef = useRef<HTMLDivElement>(null);
  const [previewHeight, setPreviewHeight] = useState(260);
  const [previewWidth, setPreviewWidth] = useState(340);
  const [libraryWidth, setLibraryWidth] = useState(230);
  const [isPreviewResizing, setIsPreviewResizing] = useState(false);
  const [isRightPreviewResizing, setIsRightPreviewResizing] = useState(false);
  const [isLibraryResizing, setIsLibraryResizing] = useState(false);
  const [previewMenuOpen, setPreviewMenuOpen] = useState(false);
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  const [exportPath, setExportPath] = useState(defaultExportPath);
  const [collapsedTags, setCollapsedTags] = useState<Set<string>>(new Set());
  const initializedCollapsedTagsRef = useRef(false);
  const [renamingTagPath, setRenamingTagPath] = useState<string | null>(null);
  const [renamingTagName, setRenamingTagName] = useState("");
  const draggedPromptIncludeRef = useRef<string | null>(null);
  const tree = useMemo(() => buildTagTree(items), [items]);
  const effectivePreviewPlacement: PromptPreviewPlacement = dslEnabled ? previewPlacement : "hidden";

  useEffect(() => {
    if (initializedCollapsedTagsRef.current || items.length === 0 || !selectedPromptId) {
      return;
    }
    initializedCollapsedTagsRef.current = true;
    setCollapsedTags(getInitialCollapsedTags(items, selectedPromptId));
  }, [items, selectedPromptId]);

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

  const startLibraryResize = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const bounds = promptBodyRef.current?.getBoundingClientRect();
    if (!bounds) {
      return;
    }

    event.preventDefault();
    setIsLibraryResizing(true);
    document.body.classList.add("is-column-resizing");

    const onMove = (moveEvent: PointerEvent) => {
      setLibraryWidth(clamp(moveEvent.clientX - bounds.left, 160, Math.max(180, bounds.width - 260)));
    };

    const onUp = () => {
      setIsLibraryResizing(false);
      document.body.classList.remove("is-column-resizing");
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
  }, []);

  const startRightPreviewResize = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const bounds = editorWrapRef.current?.getBoundingClientRect();
    if (!bounds) {
      return;
    }

    event.preventDefault();
    setIsRightPreviewResizing(true);
    document.body.classList.add("is-column-resizing");

    const onMove = (moveEvent: PointerEvent) => {
      const nextWidth = bounds.right - moveEvent.clientX;
      setPreviewWidth(clamp(nextWidth, 180, Math.max(180, Math.floor(bounds.width * 0.48))));
    };

    const onUp = () => {
      setIsRightPreviewResizing(false);
      document.body.classList.remove("is-column-resizing");
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
  }, []);

  const toggleTag = useCallback((path: string) => {
    setCollapsedTags((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const startRenameTag = useCallback((path: string, name: string) => {
    setRenamingTagPath(path);
    setRenamingTagName(name);
  }, []);

  const cancelRenameTag = useCallback(() => {
    setRenamingTagPath(null);
    setRenamingTagName("");
  }, []);

  const commitRenameTag = useCallback(
    (path: string) => {
      const nextName = renamingTagName.trim();
      const parts = path.split("/");
      const currentName = parts[parts.length - 1] ?? "";
      setRenamingTagPath(null);
      if (!nextName || nextName === currentName || nextName.includes("/")) {
        setRenamingTagName("");
        return;
      }
      const parentPath = parts.slice(0, -1).join("/");
      const nextPath = parentPath ? `${parentPath}/${nextName}` : nextName;
      setCollapsedTags((current) => {
        const next = new Set(current);
        if (next.delete(path)) {
          next.add(nextPath);
        }
        return next;
      });
      setRenamingTagName("");
      onRenameTag(path, nextPath);
    },
    [onRenameTag, renamingTagName],
  );

  return (
    <section className="panel prompt-panel">
      <PanelHeader
        icon={<ScrollText size={16} />}
        title="Prompt Editor"
        actions={
          <div className="prompt-header-actions">
            <ToggleSwitch checked={dslEnabled} label="DSL" onChange={setDslEnabled} />
            {dslEnabled ? (
              <div className="preview-menu">
                <button
                  className="icon-button"
                  title="Preview placement"
                  onClick={() => setPreviewMenuOpen((value) => !value)}
                  type="button"
                >
                  {previewPlacement === "hidden" ? <EyeOff size={16} /> : <Eye size={16} />}
                </button>
                {previewMenuOpen ? (
                  <div className="preview-menu-popover">
                    {(["bottom", "right", "hidden"] as PromptPreviewPlacement[]).map((placement) => (
                      <button
                        className={previewPlacement === placement ? "active" : ""}
                        key={placement}
                        onClick={() => {
                          setPreviewPlacement(placement);
                          setPreviewMenuOpen(false);
                        }}
                        type="button"
                      >
                        {placement[0].toUpperCase() + placement.slice(1)}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        }
      />
      <div
        className="prompt-body"
        ref={promptBodyRef}
        style={{ gridTemplateColumns: `${libraryWidth}px 8px minmax(0, 1fr)` }}
      >
        <aside className="prompt-library">
          <div className="search-box">
            <Search size={15} />
            <input
              placeholder="Search prompts..."
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </div>
          <div className="prompt-library-actions">
            <button className="icon-button" title="New prompt" onClick={onCreatePrompt}>
              <FilePlus2 size={16} />
            </button>
            <div className="prompt-sort">
              <button
                className={sortMode === "updated" ? "active" : ""}
                title="Sort by updated"
                onClick={() => setSortMode("updated")}
              >
                <Clock3 size={15} />
              </button>
              <button
                className={sortMode === "name" ? "active" : ""}
                title="Sort by name"
                onClick={() => setSortMode("name")}
              >
                <ArrowDownAZ size={16} />
              </button>
            </div>
          </div>
          <nav className="prompt-list">
            {items.length === 0 ? (
              <div className="prompt-empty">No prompts</div>
            ) : (
              renderTagNode({
                collapsedTags,
                node: tree,
                onDeletePrompt,
                onRenameTag: startRenameTag,
                onPromptIncludeDragEnd: () => {
                  draggedPromptIncludeRef.current = null;
                  onPromptIncludeDragEnd?.();
                },
                onPromptIncludeDragStart: (token) => {
                  draggedPromptIncludeRef.current = token;
                  onPromptIncludeDragStart?.(token);
                },
                onSelectPrompt,
                renamingTagName,
                renamingTagPath,
                selectedPromptId,
                setRenamingTagName,
                cancelRenameTag,
                commitRenameTag,
                toggleTag,
              })
            )}
          </nav>
        </aside>
        <div
          aria-label="Resize prompt library and editor"
          aria-orientation="vertical"
          className={`prompt-library-resizer ${isLibraryResizing ? "active" : ""}`}
          onPointerDown={startLibraryResize}
          role="separator"
          tabIndex={0}
        />
        <div
          className={`editor-wrap preview-${effectivePreviewPlacement}`}
          ref={editorWrapRef}
        >
          <div className="editor-topline">
            <input
              className="prompt-title-input"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              onBlur={onCommitMetadata}
            />
            <span className="prompt-save-state">{saveStateLabel(saveState)}</span>
          </div>
          <div className="prompt-editor-main">
            <div
              className={`prompt-editor-area${fileDropActive ? " file-drop-active" : ""}`}
              data-file-drop-zone="prompt-editor"
            >
              <PromptEditor
                dslEnabled={dslEnabled}
                getDraggedPromptInclude={() => draggedPromptIncludeRef.current}
                highlightRef={highlightRef}
                onPromptIncludeDropHandled={onPromptIncludeDropHandled}
                onPromptChange={onPromptChange}
                prompt={prompt}
              />
              <input
                className="prompt-tags-input"
                placeholder="#"
                value={tagsText}
                onChange={(event) => setTagsText(event.target.value)}
                onBlur={onCommitMetadata}
              />
            </div>
            {effectivePreviewPlacement === "bottom" ? (
              <div
                aria-label="Resize prompt editor and preview"
                aria-orientation="horizontal"
                className={`preview-resizer ${isPreviewResizing ? "active" : ""}`}
                onPointerDown={startPreviewResize}
                role="separator"
                tabIndex={0}
              />
            ) : null}
            {effectivePreviewPlacement !== "hidden" ? (
              <>
                {effectivePreviewPlacement === "right" ? (
                  <div
                    aria-label="Resize prompt editor and preview"
                    aria-orientation="vertical"
                    className={`preview-resizer-vertical ${isRightPreviewResizing ? "active" : ""}`}
                    onPointerDown={startRightPreviewResize}
                    role="separator"
                    tabIndex={0}
                  />
                ) : null}
              <div
                className="preview-box"
                style={
                  effectivePreviewPlacement === "bottom"
                    ? { flexBasis: previewHeight }
                    : { flexBasis: previewWidth }
                }
              >
                <div className="preview-heading">
                  <strong>Preview</strong>
                  <div className="export-menu">
                    <button
                      className="icon-button"
                      title="Export rendered prompt"
                      onClick={() => {
                        setExportPath(defaultExportPath);
                        setExportMenuOpen((value) => !value);
                      }}
                      type="button"
                    >
                      <Download size={15} />
                    </button>
                    {exportMenuOpen ? (
                      <div className="export-menu-popover">
                        <input
                          value={exportPath}
                          onChange={(event) => setExportPath(event.target.value)}
                        />
                        <button
                          className="primary-button"
                          onClick={() => {
                            onExportRenderedPrompt(exportPath);
                            setExportMenuOpen(false);
                          }}
                          type="button"
                        >
                          Export
                        </button>
                      </div>
                    ) : null}
                  </div>
                </div>
                <p>{renderedPrompt.trim()}</p>
              </div>
              </>
            ) : null}
          </div>
        </div>
      </div>
    </section>
  );
}

function PromptEditor({
  dslEnabled,
  getDraggedPromptInclude,
  highlightRef,
  onPromptIncludeDropHandled,
  onPromptChange,
  prompt,
}: {
  dslEnabled: boolean;
  getDraggedPromptInclude: () => string | null;
  highlightRef: RefObject<HTMLPreElement | null>;
  onPromptIncludeDropHandled?: () => void;
  onPromptChange: (value: string) => void;
  prompt: string;
}) {
  const stackRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [isComposing, setIsComposing] = useState(false);
  const highlightedPrompt = useMemo(
    () => (dslEnabled ? highlightPromptDsl(prompt) : null),
    [prompt, dslEnabled],
  );

  const syncHighlightViewport = useCallback(() => {
    const stack = stackRef.current;
    const textarea = textareaRef.current;
    const highlight = highlightRef.current;
    if (!stack || !textarea) {
      return;
    }

    const scrollbarWidth = Math.max(0, textarea.offsetWidth - textarea.clientWidth);
    const scrollbarHeight = Math.max(0, textarea.offsetHeight - textarea.clientHeight);
    stack.style.setProperty("--prompt-textarea-scrollbar-width", `${scrollbarWidth}px`);
    stack.style.setProperty("--prompt-textarea-scrollbar-height", `${scrollbarHeight}px`);

    if (highlight) {
      highlight.scrollTop = textarea.scrollTop;
      highlight.scrollLeft = textarea.scrollLeft;
    }
  }, [highlightRef]);

  useLayoutEffect(() => {
    syncHighlightViewport();
  }, [dslEnabled, prompt, syncHighlightViewport]);

  useLayoutEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }

    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(syncHighlightViewport);
    resizeObserver?.observe(textarea);
    window.addEventListener("resize", syncHighlightViewport);

    return () => {
      resizeObserver?.disconnect();
      window.removeEventListener("resize", syncHighlightViewport);
    };
  }, [syncHighlightViewport]);

  const handlePromptChange = useCallback(
    (event: ChangeEvent<HTMLTextAreaElement>) => {
      onPromptChange(event.target.value);
      requestAnimationFrame(syncHighlightViewport);
    },
    [onPromptChange, syncHighlightViewport],
  );

  return (
    <div
      className={[
        "prompt-editor-stack",
        dslEnabled ? "dsl-highlight-enabled" : "",
        isComposing ? "is-composing" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      ref={stackRef}
    >
      {dslEnabled ? (
        <pre aria-hidden="true" className="prompt-highlight" ref={highlightRef}>
          {highlightedPrompt}
          {prompt.endsWith("\n") ? " " : null}
        </pre>
      ) : null}
      <textarea
        className="prompt-textarea"
        ref={textareaRef}
        value={prompt}
        onChange={handlePromptChange}
        onBlur={() => setIsComposing(false)}
        onCompositionEnd={(event) => {
          setIsComposing(false);
          onPromptChange(event.currentTarget.value);
          requestAnimationFrame(syncHighlightViewport);
        }}
        onCompositionStart={() => setIsComposing(true)}
        onDragOver={(event) => {
          if (getDraggedPromptInclude() || hasPromptIncludeDragData(event.dataTransfer)) {
            event.preventDefault();
            event.dataTransfer.dropEffect = "copy";
          }
        }}
        onDrop={(event) => {
          const token = getDraggedPromptInclude() ?? promptIncludeTokenFromDrop(event.dataTransfer);
          if (!token) {
            return;
          }
          event.preventDefault();
          const textarea = event.currentTarget;
          const index = textareaIndexFromPoint(textarea, event.clientX, event.clientY);
          const nextPrompt = `${prompt.slice(0, index)}${token}${prompt.slice(index)}`;
          onPromptIncludeDropHandled?.();
          onPromptChange(nextPrompt);
          requestAnimationFrame(() => {
            textarea.focus();
            textarea.selectionStart = index + token.length;
            textarea.selectionEnd = index + token.length;
            syncHighlightViewport();
          });
        }}
        onScroll={(event) => {
          if (highlightRef.current) {
            highlightRef.current.scrollTop = event.currentTarget.scrollTop;
            highlightRef.current.scrollLeft = event.currentTarget.scrollLeft;
          }
          syncHighlightViewport();
        }}
        autoCapitalize="off"
        autoCorrect="off"
        spellCheck={false}
      />
    </div>
  );
}

function highlightPromptDsl(source: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const lines = source.split("\n");
  lines.forEach((line, lineIndex) => {
    const trimmed = line.trimStart();
    if (trimmed.startsWith("#") || trimmed.startsWith("//")) {
      nodes.push(
        <span className="dsl-comment" key={`comment:${lineIndex}`}>
          {line}
        </span>,
      );
    } else {
      nodes.push(...highlightDslLine(line, lineIndex));
    }
    if (lineIndex < lines.length - 1) {
      nodes.push("\n");
    }
  });
  return nodes;
}

function highlightDslLine(line: string, lineIndex: number): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern =
    /(\{#[^}]*\})|(\{[A-Za-z_][\w.-]*\})|(^\s*prompt(?=\s*=\s*\{))|(^\s*[A-Za-z_][\w.-]*(?=\s*=))/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(line))) {
    if (match.index > cursor) {
      nodes.push(line.slice(cursor, match.index));
    }
    const value = match[0];
    const className = match[1]
      ? "dsl-include"
      : match[2]
        ? "dsl-variable"
        : match[3]
          ? "dsl-keyword"
          : "dsl-assignment";
    nodes.push(
      <span className={className} key={`${lineIndex}:${match.index}`}>
        {value}
      </span>,
    );
    cursor = match.index + value.length;
  }

  if (cursor < line.length) {
    nodes.push(line.slice(cursor));
  }
  return nodes;
}

function buildTagTree(items: PromptListItem[]) {
  const root: TagNode = { children: new Map(), prompts: [] };
  for (const item of items) {
    if (item.tags.length === 0) {
      root.prompts.push(item);
      continue;
    }
    for (const tag of item.tags) {
      let node = root;
      for (const part of tag.split("/").filter(Boolean)) {
        if (!node.children.has(part)) {
          node.children.set(part, { children: new Map(), prompts: [] });
        }
        node = node.children.get(part)!;
      }
      node.prompts.push(item);
    }
  }
  return root;
}

function getInitialCollapsedTags(items: PromptListItem[], selectedPromptId: string) {
  const openPaths = new Set<string>();
  const selectedItem = items.find((item) => item.id === selectedPromptId);
  const selectedTag = selectedItem?.tags.find((tag) => tag.split("/").filter(Boolean).length > 0);

  if (selectedTag) {
    const parts = selectedTag.split("/").filter(Boolean);
    parts.forEach((_, index) => openPaths.add(parts.slice(0, index + 1).join("/")));
  }

  const collapsed = new Set<string>();
  for (const path of getTagFolderPaths(items)) {
    if (!openPaths.has(path)) {
      collapsed.add(path);
    }
  }
  return collapsed;
}

function getTagFolderPaths(items: PromptListItem[]) {
  const paths = new Set<string>();
  for (const item of items) {
    for (const tag of item.tags) {
      const parts = tag.split("/").filter(Boolean);
      parts.forEach((_, index) => paths.add(parts.slice(0, index + 1).join("/")));
    }
  }
  return paths;
}

function renderTagNode({
  collapsedTags,
  depth = 0,
  node,
  onSelectPrompt,
  onDeletePrompt,
  onRenameTag,
  onPromptIncludeDragEnd,
  onPromptIncludeDragStart,
  path = "",
  renamingTagName,
  renamingTagPath,
  selectedPromptId,
  setRenamingTagName,
  cancelRenameTag,
  commitRenameTag,
  toggleTag,
}: {
  collapsedTags: Set<string>;
  depth?: number;
  node: TagNode;
  onSelectPrompt: (id: string) => void;
  onDeletePrompt: (id?: string) => void;
  onRenameTag: (path: string, name: string) => void;
  onPromptIncludeDragEnd: () => void;
  onPromptIncludeDragStart: (token: string) => void;
  path?: string;
  renamingTagName: string;
  renamingTagPath: string | null;
  selectedPromptId: string | null;
  setRenamingTagName: (value: string) => void;
  cancelRenameTag: () => void;
  commitRenameTag: (path: string) => void;
  toggleTag: (path: string) => void;
}) {
  const rows: ReactNode[] = [];
  for (const [name, child] of [...node.children.entries()].sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    const childPath = path ? `${path}/${name}` : name;
    const collapsed = collapsedTags.has(childPath);
    const isRenaming = renamingTagPath === childPath;
    rows.push(
      <div
        className="prompt-tag-row"
        key={`tag:${childPath}`}
        role="button"
        style={{ paddingLeft: 8 + depth * 14 }}
        onClick={() => toggleTag(childPath)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            toggleTag(childPath);
          }
        }}
        tabIndex={0}
      >
        {collapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
        {isRenaming ? (
          <input
            autoFocus
            className="prompt-tag-rename-input"
            value={renamingTagName}
            onBlur={() => commitRenameTag(childPath)}
            onChange={(event) => setRenamingTagName(event.target.value)}
            onClick={(event) => event.stopPropagation()}
            onKeyDown={(event) => {
              event.stopPropagation();
              if (event.key === "Enter") {
                event.preventDefault();
                commitRenameTag(childPath);
              }
              if (event.key === "Escape") {
                event.preventDefault();
                cancelRenameTag();
              }
            }}
          />
        ) : (
          <span className="prompt-tag-name">{name}</span>
        )}
        <button
          className="prompt-tag-rename"
          title="Rename tag folder"
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            onRenameTag(childPath, name);
          }}
          onKeyDown={(event) => event.stopPropagation()}
        >
          <PenLine size={12} />
        </button>
      </div>,
    );
    if (!collapsed) {
      rows.push(
        ...renderTagNode({
          collapsedTags,
          depth: depth + 1,
          node: child,
          onDeletePrompt,
          onRenameTag,
          onPromptIncludeDragEnd,
          onPromptIncludeDragStart,
          onSelectPrompt,
          path: childPath,
          renamingTagName,
          renamingTagPath,
          selectedPromptId,
          setRenamingTagName,
          cancelRenameTag,
          commitRenameTag,
          toggleTag,
        }),
      );
    }
  }
  const seen = new Set<string>();
  for (const item of node.prompts) {
    if (seen.has(item.id)) {
      continue;
    }
    seen.add(item.id);
    rows.push(
      <button
        className={`prompt-list-item ${item.id === selectedPromptId ? "active" : ""}`}
        draggable
        key={`prompt:${path}:${item.id}`}
        style={{ paddingLeft: 14 + depth * 14 }}
        onDragEnd={onPromptIncludeDragEnd}
        onDragStart={(event) => {
          const token = promptIncludeToken(item, path);
          onPromptIncludeDragStart(token);
          event.dataTransfer.effectAllowed = "copy";
          event.dataTransfer.setData("text/plain", token);
          event.dataTransfer.setData(
            "application/x-sozocraft-prompt",
            JSON.stringify({
              id: item.id,
              name: item.name,
              tagPath: path,
              token,
            }),
          );
        }}
        onClick={() => onSelectPrompt(item.id)}
      >
        <span className="prompt-list-name">{item.name}</span>
        <span
          className="prompt-list-delete"
          role="button"
          tabIndex={0}
          title="Delete prompt"
          onClick={(event) => {
            event.stopPropagation();
            onDeletePrompt(item.id);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              event.stopPropagation();
              onDeletePrompt(item.id);
            }
          }}
        >
          <Trash2 size={13} />
        </span>
      </button>,
    );
  }
  return rows;
}

function promptIncludeToken(item: PromptListItem, tagPath: string) {
  if (!tagPath) {
    return `{# ${quoteIncludePart(item.name)}}`;
  }
  return `{# ${quoteTagPath(tagPath)}:${quoteIncludePart(item.name)}}`;
}

function quoteTagPath(tagPath: string) {
  return tagPath
    .split("/")
    .filter(Boolean)
    .map(quoteIncludePart)
    .join("/");
}

function quoteIncludePart(value: string) {
  return /^[A-Za-z0-9_.-]+$/.test(value) ? value : JSON.stringify(value);
}

function hasPromptIncludeDragData(dataTransfer: DataTransfer) {
  const types = [...dataTransfer.types];
  return types.includes("application/x-sozocraft-prompt") || types.includes("text/plain");
}

function promptIncludeTokenFromDrop(dataTransfer: DataTransfer) {
  const raw = dataTransfer.getData("application/x-sozocraft-prompt");
  if (raw) {
    try {
      const parsed: unknown = JSON.parse(raw);
      if (isPlainObject(parsed) && typeof parsed.token === "string") {
        return parsed.token;
      }
    } catch {
      return "";
    }
  }
  return dataTransfer.getData("text/plain");
}

function textareaIndexFromPoint(textarea: HTMLTextAreaElement, clientX: number, clientY: number) {
  const rect = textarea.getBoundingClientRect();
  const style = window.getComputedStyle(textarea);
  const paddingLeft = parseFloat(style.paddingLeft) || 0;
  const paddingRight = parseFloat(style.paddingRight) || 0;
  const paddingTop = parseFloat(style.paddingTop) || 0;
  const lineHeight = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.7 || 20;
  const charWidth = measureTextareaCharWidth(style);
  const contentWidth = Math.max(1, textarea.clientWidth - paddingLeft - paddingRight);
  const charsPerLine = Math.max(1, Math.floor(contentWidth / charWidth));
  const x = Math.max(0, clientX - rect.left - paddingLeft + textarea.scrollLeft);
  const y = Math.max(0, clientY - rect.top - paddingTop + textarea.scrollTop);
  const targetVisualLine = Math.max(0, Math.floor(y / lineHeight));
  const targetColumn = Math.max(0, Math.round(x / charWidth));
  const lines = textarea.value.split("\n");
  let sourceIndex = 0;
  let visualLine = 0;

  for (const line of lines) {
    const wrappedLines = Math.max(1, Math.ceil(Math.max(1, line.length) / charsPerLine));
    if (targetVisualLine < visualLine + wrappedLines) {
      const wrappedLine = targetVisualLine - visualLine;
      return sourceIndex + Math.min(line.length, wrappedLine * charsPerLine + targetColumn);
    }
    visualLine += wrappedLines;
    sourceIndex += line.length + 1;
  }

  return textarea.value.length;
}

function measureTextareaCharWidth(style: CSSStyleDeclaration) {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) {
    return 8;
  }
  context.font = style.font;
  return Math.max(1, context.measureText("0000000000").width / 10);
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function saveStateLabel(saveState: PromptSaveState) {
  switch (saveState) {
    case "dirty":
      return "Unsaved";
    case "loading":
      return "Loading";
    case "saving":
      return "Saving";
    case "error":
      return "Save error";
    case "saved":
      return "Saved";
  }
}
