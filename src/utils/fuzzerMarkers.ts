import type { FuzzTemplate, FuzzPosition } from '../services/tauri/bridge';

export const MARKER = '§';

/** Wraps the current selection in the focused input/textarea with §…§. Returns false if none is focused. */
export const insertMarkerInFocused = (): boolean => {
  const el = document.activeElement as HTMLInputElement | HTMLTextAreaElement | null;
  if (!el || (el.tagName !== 'INPUT' && el.tagName !== 'TEXTAREA')) return false;

  const start = el.selectionStart ?? el.value.length;
  const end = el.selectionEnd ?? el.value.length;
  const value = el.value;
  const next = `${value.slice(0, start)}${MARKER}${value.slice(start, end)}${MARKER}${value.slice(end)}`;

  // React tracks the value via its own setter, so set through the native one and fire a real input event
  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, 'value')?.set?.call(el, next);
  el.dispatchEvent(new Event('input', { bubbles: true }));

  const caret = end + 2;
  requestAnimationFrame(() => el.setSelectionRange(caret, caret));
  return true;
};

const markersIn = (value: string): string[] => {
  const parts = value.split(MARKER);
  const out: string[] = [];
  for (let i = 1; i < parts.length; i += 2) out.push(parts[i]);
  return out;
};

/** Positions in the same scan order as the Rust backend: url, params, headers, body. */
export const parsePositions = (t: FuzzTemplate): FuzzPosition[] => {
  const fields: Array<[string, string]> = [['url', t.url]];
  for (const p of t.params) if (p.enabled) fields.push([`param:${p.key}`, p.value]);
  for (const h of t.headers) if (h.enabled) fields.push([`header:${h.key}`, h.value]);
  fields.push(['body', t.body ?? '']);

  const positions: FuzzPosition[] = [];
  for (const [field, value] of fields) {
    for (const base of markersIn(value)) {
      positions.push({ index: positions.length, field, base });
    }
  }
  return positions;
};
