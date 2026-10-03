/** Pretty-prints `text` with 2-space indentation if it is a JSON object or array; otherwise returns it unchanged. */
export const tryPrettifyJson = (text: string): string => {
  if (!text || !text.trim()) return text;
  const trimmed = text.trim();
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try {
      const parsed = JSON.parse(trimmed);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return text;
    }
  }
  return text;
};
