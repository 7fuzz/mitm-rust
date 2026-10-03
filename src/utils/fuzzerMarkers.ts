import type { FuzzTemplate } from '../services/tauri/bridge';

/** Matches a `{{name}}` fuzz placeholder; group 1 is the variable name. Mirrors the Rust regex. */
export const MARKER_RE = /\{\{\s*([A-Za-z0-9_.\-]+)\s*\}\}/g;
const NAME_RE = /^[A-Za-z0-9_.\-]+$/;

const markedFields = (t: FuzzTemplate): string[] => [
  t.url,
  ...t.params.filter((p) => p.enabled).map((p) => p.value),
  ...t.headers.filter((h) => h.enabled).map((h) => h.value),
  t.body ?? '',
];

/** Unique variable names, in first-appearance order (url, params, headers, body). */
export const parseVariables = (t: FuzzTemplate): string[] => {
  const names: string[] = [];
  for (const value of markedFields(t)) {
    for (const match of value.matchAll(MARKER_RE)) {
      if (!names.includes(match[1])) names.push(match[1]);
    }
  }
  return names;
};

/** How many `{{name}}` occurrences exist across the whole template. */
export const countOccurrences = (t: FuzzTemplate, name: string): number =>
  markedFields(t).reduce((sum, value) => {
    let n = 0;
    for (const match of value.matchAll(MARKER_RE)) if (match[1] === name) n++;
    return sum + n;
  }, 0);

const nextVarName = (t: FuzzTemplate): string => {
  const existing = new Set(parseVariables(t));
  for (let i = 1; ; i++) {
    const name = `var${i}`;
    if (!existing.has(name)) return name;
  }
};

/**
 * Wraps the focused input/textarea selection in `{{…}}`. A selection that is a valid
 * variable name becomes `{{selection}}`; otherwise a fresh `{{varN}}` is inserted.
 * Returns the resulting value, or null if nothing is focused.
 */
export const insertMarkerInFocused = (template: FuzzTemplate): string | null => {
  const el = document.activeElement as HTMLInputElement | HTMLTextAreaElement | null;
  if (!el || (el.tagName !== 'INPUT' && el.tagName !== 'TEXTAREA')) return null;

  const start = el.selectionStart ?? el.value.length;
  const end = el.selectionEnd ?? el.value.length;
  const selected = el.value.slice(start, end);
  const name = NAME_RE.test(selected) ? selected : nextVarName(template);
  const token = `{{${name}}}`;
  const next = `${el.value.slice(0, start)}${token}${el.value.slice(end)}`;

  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, 'value')?.set?.call(el, next);
  el.dispatchEvent(new Event('input', { bubbles: true }));

  const caret = start + token.length;
  requestAnimationFrame(() => el.setSelectionRange(caret, caret));
  return next;
};

/** Replaces every `{{name}}` with its bare name, leaving readable text behind. */
export const stripMarkers = (value: string): string => value.replace(MARKER_RE, '$1');
