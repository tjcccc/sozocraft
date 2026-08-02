const SUPPORTED_DATE_TOKENS = new Set([
  "yyyyMMdd_HHmmss",
  "yyyyMMdd",
  "yyMMdd_HHmmss",
  "yyMMdd",
  "yyyy",
  "yy",
  "MM",
  "dd",
  "HH",
  "mm",
  "ss",
]);

export function validateOutputTemplate(
  template: string,
  options: { higgsfield?: boolean } = {},
): string[] {
  const issues: string[] = [];
  if (!template.trim()) {
    issues.push("Template cannot be empty.");
    return issues;
  }
  const tokenRegex = /\{([^}]+)\}/g;
  const known = new Set([
    "provider",
    "model",
    "id",
    "batch_id",
    "extension",
    "datetime",
    ...SUPPORTED_DATE_TOKENS,
  ]);
  if (options.higgsfield) {
    known.add("higgsfield_filename");
  }
  let match;
  while ((match = tokenRegex.exec(template)) !== null) {
    const token = match[1];
    if (token.startsWith("datetime:")) {
      continue;
    }
    if (/^id:\d+$/.test(token)) {
      const width = Number(token.slice("id:".length));
      if (width < 1 || width > 12) {
        issues.push(`{${token}} — ID width must be between 1 and 12.`);
      }
      continue;
    }
    if (!known.has(token)) {
      const looksLikeDate = /^[YMDHhmsS_]+$/.test(token);
      if (looksLikeDate) {
        issues.push(`{${token}} — use lowercase tokens, e.g. {yyyyMMdd_HHmmss}`);
      } else {
        issues.push(`{${token}} is not a recognised token and will render literally.`);
      }
    }
  }
  return issues;
}
