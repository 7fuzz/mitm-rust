import React, { useLayoutEffect, useRef, useState } from 'react';
import { MingCuteIcon } from './MingCuteIcon';

interface Cookie {
  id: string;
  name: string;
  value: string;
  enabled: boolean;
}

const newId = () => 'ck-' + Math.random().toString(36).substring(2, 9);

export const isCookieHeader = (key: string) => key.trim().toLowerCase() === 'cookie';

export function parseCookies(header: string): Cookie[] {
  return header
    .split(';')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const eq = part.indexOf('=');
      return {
        id: newId(),
        name: eq === -1 ? part : part.slice(0, eq).trim(),
        value: eq === -1 ? '' : part.slice(eq + 1).trim(),
        enabled: true,
      };
    });
}

function serializeCookies(cookies: Cookie[]): string {
  return cookies
    .filter((c) => c.enabled && (c.name || c.value))
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
}

export const CookieToggle: React.FC<{ value: string; expanded: boolean; onToggle: () => void }> = ({
  value,
  expanded,
  onToggle,
}) => (
  <button
    type="button"
    onClick={onToggle}
    title={expanded ? 'Collapse cookies' : 'Edit cookies individually'}
    className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle transition-colors shrink-0"
  >
    <MingCuteIcon name={expanded ? 'down_line' : 'right_line'} size={12} />
    <span className="text-3xs font-mono tabular-nums">{parseCookies(value).length}</span>
  </button>
);

const AutoGrowTextarea: React.FC<{
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}> = ({ value, onChange, placeholder, className = '' }) => {
  const ref = useRef<HTMLTextAreaElement>(null);

  const fit = () => {
    const el = ref.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight + el.offsetHeight - el.clientHeight}px`;
  };

  useLayoutEffect(fit, [value]);

  useLayoutEffect(() => {
    if (!ref.current) return;
    let lastWidth = 0;
    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width === lastWidth) return;
      lastWidth = entry.contentRect.width;
      fit();
    });
    observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  return (
    <textarea
      ref={ref}
      rows={1}
      value={value}
      placeholder={placeholder}
      spellCheck={false}
      onChange={(e) => onChange(e.target.value.replace(/[\r\n]/g, ''))}
      className={`min-w-0 resize-none overflow-hidden break-all ${className}`}
    />
  );
};

interface CookieEditorProps {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
}

export const CookieEditor: React.FC<CookieEditorProps> = ({ value, onChange, readOnly = false }) => {
  const [cookies, setCookies] = useState(() => parseCookies(value));
  const [syncedValue, setSyncedValue] = useState(value);

  // Keep local rows (including disabled ones) unless the header changed from outside.
  if (value !== syncedValue) {
    setCookies(parseCookies(value));
    setSyncedValue(value);
  }

  const commit = (next: Cookie[]) => {
    const serialized = serializeCookies(next);
    setCookies(next);
    setSyncedValue(serialized);
    onChange(serialized);
  };

  const update = (id: string, patch: Partial<Cookie>) =>
    commit(cookies.map((c) => (c.id === id ? { ...c, ...patch } : c)));

  const inputClass =
    'bg-transparent px-1.5 py-1 font-mono text-xs focus:outline-none border border-transparent focus:border-border rounded';

  return (
    <div className="border-l-2 border-primary/30 ml-2 pl-2 py-1 space-y-0.5">
      {cookies.map((c) => {
        const textClass = c.enabled ? 'text-foreground' : 'line-through text-muted-foreground';
        return (
          <div key={c.id} className="flex items-start gap-1.5 group">
            {!readOnly && (
              <input
                type="checkbox"
                checked={c.enabled}
                onChange={() => update(c.id, { enabled: !c.enabled })}
                className="mt-1.5 rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer shrink-0"
              />
            )}
            {readOnly ? (
              <>
                <span className="w-40 shrink-0 px-1.5 py-1 font-mono text-xs text-primary break-all">{c.name}</span>
                <span className="flex-1 px-1.5 py-1 font-mono text-xs text-foreground break-all">{c.value}</span>
              </>
            ) : (
              <>
                <input
                  type="text"
                  value={c.name}
                  placeholder="name"
                  spellCheck={false}
                  onChange={(e) => update(c.id, { name: e.target.value })}
                  className={`w-40 shrink-0 ${inputClass} ${c.enabled ? 'text-primary' : textClass}`}
                />
                <AutoGrowTextarea
                  value={c.value}
                  placeholder="value"
                  onChange={(v) => update(c.id, { value: v })}
                  className={`flex-1 ${inputClass} ${textClass}`}
                />
                <button
                  type="button"
                  onClick={() => commit(cookies.filter((x) => x.id !== c.id))}
                  className="mt-0.5 p-1 text-muted-foreground hover:text-rose-500 rounded transition-colors shrink-0"
                  title="Delete cookie"
                >
                  <MingCuteIcon name="delete_2_line" size={14} />
                </button>
              </>
            )}
          </div>
        );
      })}
      {!readOnly && (
        <button
          type="button"
          onClick={() => setCookies([...cookies, { id: newId(), name: '', value: '', enabled: true }])}
          className="inline-flex items-center gap-1 px-1.5 py-1 text-muted-foreground hover:text-foreground text-xs transition-colors"
        >
          <MingCuteIcon name="plus_line" size={12} />
          Add cookie
        </button>
      )}
    </div>
  );
};
