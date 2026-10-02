import React, { useEffect, useMemo, useRef, useState } from 'react';
import { MingCuteIcon } from './MingCuteIcon';

export interface StandardHeader {
  key: string;
  value: string;
}

interface StandardHeaderPickerProps {
  headers: StandardHeader[];
  /** Header names already present; matched case-insensitively and hidden from the list */
  existingKeys: string[];
  onPick: (header: StandardHeader) => void;
}

/** Button that opens a searchable list of standard headers not yet added. Enter picks the highlighted one. */
export const StandardHeaderPicker: React.FC<StandardHeaderPickerProps> = ({ headers, existingKeys, onPick }) => {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [highlight, setHighlight] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const available = useMemo(() => {
    const existing = new Set(existingKeys.map((k) => k.trim().toLowerCase()));
    const q = query.trim().toLowerCase();
    return headers.filter((h) => !existing.has(h.key.toLowerCase()) && (!q || h.key.toLowerCase().includes(q)));
  }, [headers, existingKeys, query]);

  useEffect(() => {
    setHighlight(0);
  }, [query]);

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open]);

  useEffect(() => {
    listRef.current?.children[highlight]?.scrollIntoView({ block: 'nearest' });
  }, [highlight]);

  const close = () => {
    setOpen(false);
    setQuery('');
  };

  const pick = (header: StandardHeader | undefined) => {
    if (!header) return;
    onPick(header);
    close();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      pick(available[highlight]);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setHighlight((i) => Math.min(i + 1, available.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setHighlight((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  };

  return (
    <div ref={rootRef} className="relative inline-block">
      <button
        type="button"
        onClick={() => (open ? close() : setOpen(true))}
        className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-neutral-subtle border border-border text-foreground hover:bg-neutral-subtle/80 text-xs font-medium transition-colors cursor-pointer"
      >
        <MingCuteIcon name="plus_line" size={14} />
        Add Standard Header
      </button>

      {open && (
        <div className="absolute left-0 top-full mt-1 z-30 w-72 bg-surface border border-border rounded-lg shadow-lg overflow-hidden">
          <div className="p-1.5 border-b border-border">
            <input
              autoFocus
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Search headers..."
              className="w-full bg-background border border-border focus:border-primary rounded px-2 py-1 text-xs text-foreground outline-none"
            />
          </div>
          {available.length === 0 ? (
            <div className="px-3 py-2 text-2xs text-muted-foreground italic">
              {query.trim() ? 'No matching headers' : 'All standard headers already added'}
            </div>
          ) : (
            <ul ref={listRef} className="max-h-60 overflow-y-auto py-1">
              {available.map((h, i) => (
                <li
                  key={h.key}
                  onMouseEnter={() => setHighlight(i)}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    pick(h);
                  }}
                  className={`px-3 py-1 cursor-pointer ${i === highlight ? 'bg-primary/15' : ''}`}
                >
                  <div className="font-mono text-xs text-foreground">{h.key}</div>
                  {h.value && <div className="font-mono text-3xs text-muted-foreground truncate">{h.value}</div>}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
};
