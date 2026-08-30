// ─── Serialize visible JSON tree state ────────────────────────────────────────
// Produces a JSON string that matches the visual collapsed/truncated state of the tree.
export function serializeVisible(
  value: unknown,
  collapsed: Set<string>,
  expandedArrays: Set<string>,
  path: string,
  indent: number
): string {
  const pad = '  '.repeat(indent);
  const innerPad = '  '.repeat(indent + 1);

  if (value === null) return 'null';
  if (typeof value === 'boolean' || typeof value === 'number') return String(value);
  if (typeof value === 'string') return JSON.stringify(value);

  if (Array.isArray(value)) {
    if (collapsed.has(path)) return `[...] // ${value.length} items`;
    if (value.length === 0) return '[]';
    const showAll = expandedArrays.has(path) || value.length <= 1;
    const visible = showAll ? value : value.slice(0, 1);
    const lines = visible.map((item, i) =>
      `${innerPad}${serializeVisible(item, collapsed, expandedArrays, `${path}-${i}`, indent + 1)}`
    );
    if (!showAll && value.length > 1) {
      lines.push(`${innerPad}// ... ${value.length - 1} more items`);
    }
    return `[\n${lines.join(',\n')}\n${pad}]`;
  }

  if (typeof value === 'object' && value !== null) {
    if (collapsed.has(path)) return `{...} // ${Object.keys(value as object).length} keys`;
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return '{}';
    const lines = entries.map(([k, v]) =>
      `${innerPad}${JSON.stringify(k)}: ${serializeVisible(v, collapsed, expandedArrays, `${path}-${encodeURIComponent(k)}`, indent + 1)}`
    );
    return `{\n${lines.join(',\n')}\n${pad}}`;
  }

  return String(value);
}

// ─── Deep search helper ───────────────────────────────────────────────────────
export const deepSearch = (obj: unknown, term: string): boolean => {
  if (!term) return false;
  if (obj === null || typeof obj !== 'object') {
    return String(obj).toLowerCase().includes(term);
  }
  if (Array.isArray(obj)) {
    return obj.some((item) => deepSearch(item, term));
  }
  return Object.entries(obj as Record<string, unknown>).some(
    ([k, v]) => k.toLowerCase().includes(term) || deepSearch(v, term)
  );
};

// ─── Double-click to select text without quotes ────────────────────────────────
export function selectTextOf(el: HTMLElement) {
  const text = el.textContent || '';
  const unquoted = text.startsWith('"') && text.endsWith('"')
    ? text.slice(1, -1)
    : text;
  if (!unquoted) return;

  const sel = window.getSelection();
  if (!sel) return;
  sel.removeAllRanges();

  const range = document.createRange();
  const node = el.firstChild;
  if (!node) return;

  const children = Array.from(el.childNodes);
  if (children.length === 0) return;

  try {
    if (children.length >= 3 &&
      children[0].textContent === '"' &&
      children[children.length - 1].textContent === '"') {
      range.setStartBefore(children[1]);
      range.setEndAfter(children[children.length - 2]);
    } else if (children.length === 1) {
      range.selectNodeContents(el);
    } else {
      range.selectNodeContents(el);
    }
    sel.addRange(range);
  } catch {
    navigator.clipboard.writeText(unquoted).catch(() => {});
  }
}
