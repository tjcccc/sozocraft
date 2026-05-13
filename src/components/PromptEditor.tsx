import {
  CaseSensitive,
  ChevronDown,
  ChevronUp,
  PenLine,
  Search,
  X,
} from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent, ReactNode, RefObject } from "react";
import { textareaIndexFromPoint } from "../utils/textareaPosition";

export function PromptEditor({
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
  const searchInputRef = useRef<HTMLInputElement>(null);
  const replaceInputRef = useRef<HTMLInputElement>(null);
  const [isComposing, setIsComposing] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [replaceOpen, setReplaceOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [replaceValue, setReplaceValue] = useState("");
  const [matchCase, setMatchCase] = useState(false);
  const [activeMatchIndex, setActiveMatchIndex] = useState<number | null>(null);
  const highlightedPrompt = useMemo(
    () => (dslEnabled ? highlightPromptDsl(prompt) : null),
    [prompt, dslEnabled],
  );
  const searchMatches = useMemo(
    () => findSearchMatches(prompt, searchQuery, matchCase),
    [matchCase, prompt, searchQuery],
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

  const focusSearchInput = useCallback(() => {
    requestAnimationFrame(() => {
      searchInputRef.current?.focus();
      searchInputRef.current?.select();
    });
  }, []);

  const selectTextareaRange = useCallback(
    (start: number, end: number, matchIndex: number | null) => {
      const textarea = textareaRef.current;
      if (!textarea) {
        return;
      }
      textarea.focus({ preventScroll: true });
      textarea.setSelectionRange(start, end);
      scrollTextareaSelectionIntoView(textarea, prompt, start);
      setActiveMatchIndex(matchIndex);
      requestAnimationFrame(syncHighlightViewport);
    },
    [prompt, syncHighlightViewport],
  );

  const openSearchPanel = useCallback(
    (showReplace: boolean) => {
      const textarea = textareaRef.current;
      const selectedText =
        textarea && textarea.selectionStart !== textarea.selectionEnd
          ? prompt.slice(textarea.selectionStart, textarea.selectionEnd)
          : "";
      if (selectedText && !selectedText.includes("\n")) {
        setSearchQuery(selectedText);
      }
      setSearchOpen(true);
      setReplaceOpen(showReplace);
      setActiveMatchIndex(null);
      focusSearchInput();
    },
    [focusSearchInput, prompt],
  );

  const closeSearchPanel = useCallback(() => {
    setSearchOpen(false);
    setReplaceOpen(false);
    setActiveMatchIndex(null);
    requestAnimationFrame(() => textareaRef.current?.focus());
  }, []);

  const findMatch = useCallback(
    (direction: 1 | -1) => {
      if (searchMatches.length === 0) {
        setActiveMatchIndex(null);
        return;
      }
      const textarea = textareaRef.current;
      const selectionStart = textarea?.selectionStart ?? 0;
      const selectionEnd = textarea?.selectionEnd ?? selectionStart;
      const selectedIndex = findSelectedMatchIndex(searchMatches, selectionStart, selectionEnd);
      const nextIndex =
        selectedIndex >= 0
          ? wrapIndex(selectedIndex + direction, searchMatches.length)
          : nearestMatchIndex(searchMatches, direction > 0 ? selectionEnd : selectionStart, direction);
      const match = searchMatches[nextIndex];
      if (!match) {
        return;
      }
      selectTextareaRange(match.start, match.end, nextIndex);
    },
    [searchMatches, selectTextareaRange],
  );

  const replaceMatchAt = useCallback(
    (matchIndex: number) => {
      const match = searchMatches[matchIndex];
      if (!match) {
        return;
      }
      const nextPrompt = `${prompt.slice(0, match.start)}${replaceValue}${prompt.slice(match.end)}`;
      const nextCursor = match.start + replaceValue.length;
      const nextMatches = findSearchMatches(nextPrompt, searchQuery, matchCase);
      const nextMatchIndex = nextMatches.findIndex((candidate) => candidate.start >= nextCursor);
      const wrappedNextMatchIndex = nextMatchIndex >= 0 ? nextMatchIndex : nextMatches.length > 0 ? 0 : null;
      onPromptChange(nextPrompt);
      requestAnimationFrame(() => {
        if (wrappedNextMatchIndex === null) {
          selectTextareaRange(nextCursor, nextCursor, null);
          return;
        }
        const nextMatch = nextMatches[wrappedNextMatchIndex];
        selectTextareaRange(nextMatch.start, nextMatch.end, wrappedNextMatchIndex);
      });
    },
    [matchCase, onPromptChange, prompt, replaceValue, searchMatches, searchQuery, selectTextareaRange],
  );

  const replaceCurrentMatch = useCallback(() => {
    if (searchMatches.length === 0) {
      return;
    }
    const textarea = textareaRef.current;
    const selectionStart = textarea?.selectionStart ?? 0;
    const selectionEnd = textarea?.selectionEnd ?? selectionStart;
    const selectedIndex = findSelectedMatchIndex(searchMatches, selectionStart, selectionEnd);
    const targetIndex =
      selectedIndex >= 0 ? selectedIndex : nearestMatchIndex(searchMatches, selectionEnd, 1);
    replaceMatchAt(targetIndex);
  }, [replaceMatchAt, searchMatches]);

  const replaceAllMatches = useCallback(() => {
    if (searchMatches.length === 0) {
      return;
    }
    const nextPrompt = replaceMatches(prompt, searchMatches, replaceValue);
    onPromptChange(nextPrompt);
    requestAnimationFrame(() => {
      const textarea = textareaRef.current;
      if (!textarea) {
        return;
      }
      textarea.focus({ preventScroll: true });
      textarea.setSelectionRange(0, 0);
      setActiveMatchIndex(null);
      syncHighlightViewport();
    });
  }, [onPromptChange, prompt, replaceValue, searchMatches, syncHighlightViewport]);

  const handleEditorShortcut = useCallback(
    (event: ReactKeyboardEvent<HTMLElement>) => {
      if (isPromptEditorShortcut(event, "f")) {
        event.preventDefault();
        event.stopPropagation();
        openSearchPanel(false);
        return true;
      }
      if (isPromptEditorShortcut(event, "r")) {
        event.preventDefault();
        event.stopPropagation();
        openSearchPanel(true);
        return true;
      }
      return false;
    },
    [openSearchPanel],
  );

  const handleSearchKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLInputElement>) => {
      if (handleEditorShortcut(event)) {
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        closeSearchPanel();
        return;
      }
      if (event.key !== "Enter") {
        return;
      }
      event.preventDefault();
      if (event.currentTarget === replaceInputRef.current && !event.shiftKey) {
        replaceCurrentMatch();
        return;
      }
      findMatch(event.shiftKey ? -1 : 1);
    },
    [closeSearchPanel, findMatch, handleEditorShortcut, replaceCurrentMatch],
  );

  useEffect(() => {
    if (activeMatchIndex !== null && activeMatchIndex >= searchMatches.length) {
      setActiveMatchIndex(searchMatches.length > 0 ? searchMatches.length - 1 : null);
    }
  }, [activeMatchIndex, searchMatches.length]);

  const matchStatus =
    searchQuery.length === 0
      ? "0/0"
      : searchMatches.length === 0
        ? "0/0"
        : `${activeMatchIndex === null ? 0 : activeMatchIndex + 1}/${searchMatches.length}`;

  return (
    <div className="prompt-editor-shell">
      {searchOpen ? (
        <div className={`editor-search-panel${replaceOpen ? " replace-open" : ""}`}>
          <div className="editor-search-row">
            <div className="editor-search-input">
              <Search size={14} />
              <input
                ref={searchInputRef}
                value={searchQuery}
                onChange={(event) => {
                  setSearchQuery(event.target.value);
                  setActiveMatchIndex(null);
                }}
                onKeyDown={handleSearchKeyDown}
                placeholder="Find"
                autoCapitalize="off"
                autoCorrect="off"
                spellCheck={false}
              />
            </div>
            <span className="editor-search-count">{matchStatus}</span>
            <button
              className={matchCase ? "editor-search-button active" : "editor-search-button"}
              onClick={() => {
                setMatchCase((value) => !value);
                setActiveMatchIndex(null);
              }}
              title="Match case"
              type="button"
            >
              <CaseSensitive size={15} />
            </button>
            <button
              className="editor-search-button"
              disabled={searchMatches.length === 0}
              onClick={() => findMatch(-1)}
              title="Previous match"
              type="button"
            >
              <ChevronUp size={15} />
            </button>
            <button
              className="editor-search-button"
              disabled={searchMatches.length === 0}
              onClick={() => findMatch(1)}
              title="Next match"
              type="button"
            >
              <ChevronDown size={15} />
            </button>
            <button
              className="editor-search-button"
              onClick={() => setReplaceOpen((value) => !value)}
              title="Replace"
              type="button"
            >
              <PenLine size={15} />
            </button>
            <button
              className="editor-search-button"
              onClick={closeSearchPanel}
              title="Close search"
              type="button"
            >
              <X size={15} />
            </button>
          </div>
          {replaceOpen ? (
            <div className="editor-search-row replace-row">
              <div className="editor-search-input">
                <PenLine size={14} />
                <input
                  ref={replaceInputRef}
                  value={replaceValue}
                  onChange={(event) => setReplaceValue(event.target.value)}
                  onKeyDown={handleSearchKeyDown}
                  placeholder="Replace"
                  autoCapitalize="off"
                  autoCorrect="off"
                  spellCheck={false}
                />
              </div>
              <button
                className="secondary-button editor-replace-button"
                disabled={searchMatches.length === 0}
                onClick={replaceCurrentMatch}
                type="button"
              >
                Replace
              </button>
              <button
                className="secondary-button editor-replace-button"
                disabled={searchMatches.length === 0}
                onClick={replaceAllMatches}
                type="button"
              >
                All
              </button>
            </div>
          ) : null}
        </div>
      ) : null}
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
          onKeyDown={(event) => {
            handleEditorShortcut(event);
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
    </div>
  );
}

export type SearchMatch = {
  end: number;
  start: number;
};

export function findSearchMatches(source: string, query: string, matchCase: boolean): SearchMatch[] {
  if (!query) {
    return [];
  }
  const haystack = matchCase ? source : source.toLocaleLowerCase();
  const needle = matchCase ? query : query.toLocaleLowerCase();
  const matches: SearchMatch[] = [];
  let cursor = 0;
  while (cursor <= haystack.length) {
    const index = haystack.indexOf(needle, cursor);
    if (index < 0) {
      break;
    }
    matches.push({ start: index, end: index + query.length });
    cursor = index + Math.max(needle.length, 1);
  }
  return matches;
}

function findSelectedMatchIndex(matches: SearchMatch[], selectionStart: number, selectionEnd: number) {
  return matches.findIndex((match) => match.start === selectionStart && match.end === selectionEnd);
}

function nearestMatchIndex(matches: SearchMatch[], position: number, direction: 1 | -1) {
  if (matches.length === 0) {
    return 0;
  }
  if (direction > 0) {
    const nextIndex = matches.findIndex((match) => match.start >= position);
    return nextIndex >= 0 ? nextIndex : 0;
  }
  for (let index = matches.length - 1; index >= 0; index -= 1) {
    if (matches[index].end <= position) {
      return index;
    }
  }
  return matches.length - 1;
}

export function replaceMatches(source: string, matches: SearchMatch[], replacement: string) {
  let cursor = 0;
  let result = "";
  matches.forEach((match) => {
    result += source.slice(cursor, match.start);
    result += replacement;
    cursor = match.end;
  });
  return result + source.slice(cursor);
}

function wrapIndex(index: number, length: number) {
  return ((index % length) + length) % length;
}

function scrollTextareaSelectionIntoView(textarea: HTMLTextAreaElement, source: string, index: number) {
  const style = window.getComputedStyle(textarea);
  const parsedLineHeight = Number.parseFloat(style.lineHeight);
  const fontSize = Number.parseFloat(style.fontSize);
  const lineHeight = Number.isFinite(parsedLineHeight) ? parsedLineHeight : Math.max(1, fontSize * 1.7);
  const lineIndex = source.slice(0, index).split("\n").length - 1;
  const targetTop = lineIndex * lineHeight;
  const visibleTop = textarea.scrollTop;
  const visibleBottom = visibleTop + textarea.clientHeight - lineHeight;
  if (targetTop < visibleTop || targetTop > visibleBottom) {
    textarea.scrollTop = Math.max(0, targetTop - textarea.clientHeight / 2);
  }
}

function isPromptEditorShortcut(event: ReactKeyboardEvent<HTMLElement>, key: "f" | "r") {
  if (event.key.toLocaleLowerCase() !== key || event.altKey || event.shiftKey) {
    return false;
  }
  const applePlatform = /mac|iphone|ipad|ipod/i.test(navigator.platform);
  return applePlatform ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
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

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
