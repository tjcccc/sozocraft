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

export function validateOutputTemplate(template: string): string[] {
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
  let match;
  while ((match = tokenRegex.exec(template)) !== null) {
    const token = match[1];
    if (token.startsWith("datetime:")) {
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
