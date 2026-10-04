import React, { useEffect, useMemo, useRef, useState } from 'react';
import { MingCuteIcon } from '../MingCuteIcon';
import { SegmentedControl } from '../ui';
import { UrlEncodedEditor } from '../UrlEncodedEditor';
import { MultipartEditor } from '../MultipartEditor';
import { BinaryBodyEditor } from './BinaryBodyEditor';
import { readFileAsBase64 } from '../../../services/tauri/bridge';
import type { MultipartField } from '../../../types';
import {
  BODY_KIND_LABELS,
  BINARY_PREFIX,
  detectBodyKind,
  multipartBoundary,
  parseMultipart,
  parseUrlEncoded,
  serializeMultipart,
  serializeUrlEncoded,
} from '../../../utils/bodyFormat';
import { tryPrettifyJson } from '../../../utils/prettifyJson';

interface RawBodyEditorProps {
  /** Raw wire body; non-UTF-8 bodies are `base64:` encoded. */
  value: string;
  contentType: string;
  onChange: (value: string) => void;
}

/**
 * Keeps a parsed copy of the raw body so rows that don't serialize (disabled or blank)
 * survive edits; re-parses only when the raw text changes from outside.
 */
function useParsedRows<T>(raw: string, parse: (raw: string) => T[], serialize: (rows: T[]) => string, emit: (raw: string) => void) {
  const [rows, setRows] = useState<T[]>(() => parse(raw));
  const lastEmitted = useRef(raw);

  useEffect(() => {
    if (raw !== lastEmitted.current) {
      lastEmitted.current = raw;
      setRows(parse(raw));
    }
  }, [raw]);

  const update = (next: T[]) => {
    setRows(next);
    const serialized = serialize(next);
    lastEmitted.current = serialized;
    emit(serialized);
  };
  return [rows, update] as const;
}

const UrlEncodedRows: React.FC<{ value: string; onChange: (v: string) => void }> = ({ value, onChange }) => {
  const [params, setParams] = useParsedRows(value, parseUrlEncoded, serializeUrlEncoded, onChange);
  return <UrlEncodedEditor params={params} onChange={setParams} />;
};

const MultipartRows: React.FC<{ value: string; boundary: string; onChange: (v: string) => void }> = ({ value, boundary, onChange }) => {
  const [fields, setFields] = useParsedRows(
    value,
    (raw) => parseMultipart(raw, boundary),
    (rows) => serializeMultipart(rows, boundary),
    onChange
  );

  const handleChange = async (next: MultipartField[]) => {
    const resolved = await Promise.all(
      next.map(async (f) => (f.type === 'file_path' ? { ...f, type: 'base64' as const, value: await readFileAsBase64(f.value) } : f))
    );
    setFields(resolved);
  };
  return <MultipartEditor fields={fields} onChange={handleChange} />;
};

const VIEW_OPTIONS = [
  { value: 'form', label: 'Form' },
  { value: 'raw', label: 'Raw' },
] as const;

export const RawBodyEditor: React.FC<RawBodyEditorProps> = ({ value, contentType, onChange }) => {
  const kind = useMemo(() => detectBodyKind(contentType, value), [contentType, value]);
  const boundary = useMemo(() => (kind === 'multipart' ? multipartBoundary(contentType, value) : null), [kind, contentType, value]);
  const isForm = kind === 'urlencoded' || (kind === 'multipart' && boundary !== null);
  const isBinary = value.startsWith(BINARY_PREFIX);
  const [view, setView] = useState<'form' | 'raw'>('form');
  const showForm = isForm && (view === 'form' || isBinary);

  return (
    <div className="flex flex-col h-full gap-2">
      <div className="flex items-center gap-2 font-sans">
        <span className="px-1.5 py-0.5 rounded text-3xs font-mono font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30">
          {BODY_KIND_LABELS[kind]}
        </span>
        <div className="ml-auto flex items-center gap-2">
          {isForm && !isBinary && (
            <SegmentedControl
              value={view}
              onChange={setView}
              options={VIEW_OPTIONS}
              sizeVariant="sm"
            />
          )}
          {kind === 'json' && (
            <button
              type="button"
              onClick={() => onChange(tryPrettifyJson(value))}
              className="flex items-center gap-1.5 px-2.5 py-1 bg-surface hover:bg-neutral-subtle border border-border text-foreground rounded text-xs transition-colors cursor-pointer font-medium"
              title="Format JSON with 2-space indentation"
            >
              <MingCuteIcon name="code_line" size={13} className="text-amber-500" />
              <span>Prettify</span>
            </button>
          )}
        </div>
      </div>

      {showForm && kind === 'urlencoded' ? (
        <UrlEncodedRows value={value} onChange={onChange} />
      ) : showForm && boundary ? (
        <MultipartRows value={value} boundary={boundary} onChange={onChange} />
      ) : isBinary ? (
        <div className="flex-1 min-h-[350px]">
          <BinaryBodyEditor value={value} onChange={onChange} />
        </div>
      ) : (
        <textarea
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="Enter raw request/response body payload..."
          spellCheck={false}
          className="flex-1 min-h-[350px] w-full bg-surface border border-border focus:border-primary rounded-lg p-3 text-foreground outline-none resize-none font-mono text-xs leading-relaxed"
        />
      )}
    </div>
  );
};
