export function textareaIndexFromPoint(
  textarea: HTMLTextAreaElement,
  clientX: number,
  clientY: number,
) {
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
