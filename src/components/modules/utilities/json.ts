import type { JsonIndent, JsonOperation } from '../../../stores/useUtilitiesStore';

export type ParseResult = { ok: true; value: unknown } | { ok: false; error: string };

export const parseJson = (text: string): ParseResult => {
  try {
    return { ok: true, value: JSON.parse(text) };
  } catch (e: any) {
    return { ok: false, error: e.message || String(e) };
  }
};

const isPlainObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

export const sortKeysDeep = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(sortKeysDeep);
  if (isPlainObject(value)) {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((k) => [k, sortKeysDeep(value[k])])
    );
  }
  return value;
};

const indentArg = (indent: JsonIndent) => (indent === 'tab' ? '\t' : Number(indent));

export const prettify = (value: unknown, indent: JsonIndent = '2', sortKeys = false) =>
  JSON.stringify(sortKeys ? sortKeysDeep(value) : value, null, indentArg(indent));

export interface FormatOptions {
  operation: JsonOperation;
  indent: JsonIndent;
  sortKeys: boolean;
}

export function formatJson(input: string, opts: FormatOptions): { output: string; error: string | null } {
  if (!input.trim()) return { output: '', error: null };

  if (opts.operation === 'escape') return { output: JSON.stringify(input), error: null };

  if (opts.operation === 'unescape') {
    const trimmed = input.trim();
    const literal = trimmed.startsWith('"') ? trimmed : `"${trimmed}"`;
    const parsed = parseJson(literal);
    if (!parsed.ok) return { output: '', error: parsed.error };
    const text = String(parsed.value);
    const inner = parseJson(text);
    return { output: inner.ok && typeof inner.value === 'object' ? prettify(inner.value, opts.indent) : text, error: null };
  }

  const parsed = parseJson(input);
  if (!parsed.ok) return { output: '', error: parsed.error };
  const value = opts.sortKeys ? sortKeysDeep(parsed.value) : parsed.value;
  return {
    output: opts.operation === 'minify' ? JSON.stringify(value) : JSON.stringify(value, null, indentArg(opts.indent)),
    error: null,
  };
}

export const describeJson = (value: unknown): string => {
  if (Array.isArray(value)) return `array · ${value.length} item${value.length === 1 ? '' : 's'}`;
  if (isPlainObject(value)) {
    const n = Object.keys(value).length;
    return `object · ${n} key${n === 1 ? '' : 's'}`;
  }
  return value === null ? 'null' : typeof value;
};

export type ChangeKind = 'added' | 'removed' | 'changed';

export interface JsonChange {
  path: string;
  kind: ChangeKind;
  before?: unknown;
  after?: unknown;
}

const IDENTIFIER = /^[A-Za-z_$][\w$]*$/;
const childPath = (path: string, key: string | number) =>
  typeof key === 'number' ? `${path}[${key}]` : IDENTIFIER.test(key) ? `${path}.${key}` : `${path}[${JSON.stringify(key)}]`;

export function diffJson(before: unknown, after: unknown, path = '$', out: JsonChange[] = []): JsonChange[] {
  if (Array.isArray(before) && Array.isArray(after)) {
    const len = Math.max(before.length, after.length);
    for (let i = 0; i < len; i++) {
      const p = childPath(path, i);
      if (i >= before.length) out.push({ path: p, kind: 'added', after: after[i] });
      else if (i >= after.length) out.push({ path: p, kind: 'removed', before: before[i] });
      else diffJson(before[i], after[i], p, out);
    }
    return out;
  }

  if (isPlainObject(before) && isPlainObject(after)) {
    for (const key of Object.keys(before)) {
      const p = childPath(path, key);
      if (!Object.hasOwn(after, key)) out.push({ path: p, kind: 'removed', before: before[key] });
      else diffJson(before[key], after[key], p, out);
    }
    for (const key of Object.keys(after)) {
      if (!Object.hasOwn(before, key)) out.push({ path: childPath(path, key), kind: 'added', after: after[key] });
    }
    return out;
  }

  if (before !== after) out.push({ path, kind: 'changed', before, after });
  return out;
}

export const previewValue = (value: unknown, max = 60) => {
  const text = JSON.stringify(value);
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
};

export const SAMPLE_JSON = {
  user: 'admin',
  roles: ['security_admin', 'auditor'],
  metadata: { active: true, login_attempts: 0 },
};

export const SAMPLE_DIFF = {
  left: {
    id: 42,
    user: 'admin',
    roles: ['security_admin', 'auditor'],
    session: { active: true, ttl: 3600 },
  },
  right: {
    id: 42,
    user: 'admin',
    roles: ['security_admin'],
    session: { active: false, ttl: 3600, mfa: 'totp' },
    debug: true,
  },
};
