import React, { useEffect, useState } from 'react';
import { getDbRow, type DbCell, type DbColumn } from '../../../../services/tauri/bridge';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { DbCellValue } from './DbGrid';

interface DbRowDetailProps {
  table: string;
  columns: DbColumn[];
  /** Values as shown in the grid (long text is cut there) */
  values: DbCell[];
  /** When known, the full row is loaded by rowid */
  rowid?: number;
  onClose: () => void;
}

/** JSON text pretty-printed, anything else as is */
const formatValue = (value: string) => {
  const trimmed = value.trim();
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
    try {
      return JSON.stringify(JSON.parse(trimmed), null, 2);
    } catch {
      // not JSON
    }
  }
  return value;
};

/** Every column of one row, with full values and copy buttons. */
export const DbRowDetail: React.FC<DbRowDetailProps> = ({ table, columns, values, rowid, onClose }) => {
  const [fullValues, setFullValues] = useState<DbCell[] | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    setFullValues(null);
    if (rowid === undefined) return;
    getDbRow(table, rowid)
      .then(setFullValues)
      .catch((err) => console.error('Failed to load row:', err));
  }, [table, rowid]);

  const shown = fullValues ?? values;

  const copy = (key: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(key);
    setTimeout(() => setCopied((c) => (c === key ? null : c)), 1500);
  };

  const rowAsJson = () =>
    JSON.stringify(Object.fromEntries(columns.map((c, i) => [c.name, shown[i] ?? null])), null, 2);

  return (
    <div className="h-full flex flex-col bg-surface overflow-hidden">
      <div className="h-9 px-3 bg-header border-b border-border flex items-center gap-2 shrink-0">
        <span className="font-semibold text-foreground">Row</span>
        {rowid !== undefined && <span className="font-mono text-2xs text-muted-foreground">rowid {rowid}</span>}
        <div className="ml-auto flex items-center gap-1">
          <button
            onClick={() => copy('__row', rowAsJson())}
            className="flex items-center gap-1 px-1.5 py-0.5 rounded text-2xs text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
            title="Copy the row as a JSON object"
          >
            <MingCuteIcon name={copied === '__row' ? 'check_line' : 'copy_line'} size={12} />
            <span>JSON</span>
          </button>
          <button
            onClick={onClose}
            className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
            title="Close"
          >
            <MingCuteIcon name="close_line" size={14} />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto divide-y divide-border">
        {columns.map((col, i) => {
          const value = shown[i] ?? null;
          return (
            <div key={col.name} className="group px-3 py-2 space-y-1">
              <div className="flex items-center gap-1.5">
                <span className="font-mono font-semibold text-foreground text-2xs">{col.name}</span>
                {col.declType && <span className="font-mono text-3xs text-muted-foreground">{col.declType}</span>}
                {col.primaryKey && <span className="text-3xs font-bold px-1 rounded bg-amber-500/15 text-amber-500">PK</span>}
                {value !== null && (
                  <button
                    onClick={() => copy(col.name, String(value))}
                    className="ml-auto opacity-0 group-hover:opacity-100 p-0.5 rounded text-muted-foreground hover:text-foreground cursor-pointer transition-opacity"
                    title="Copy value"
                  >
                    <MingCuteIcon name={copied === col.name ? 'check_line' : 'copy_line'} size={12} />
                  </button>
                )}
              </div>
              {typeof value === 'string' ? (
                <pre className="font-mono text-2xs text-foreground whitespace-pre-wrap break-all bg-background border border-border rounded px-2 py-1.5 max-h-80 overflow-auto select-text">
                  {formatValue(value)}
                </pre>
              ) : (
                <div className="font-mono text-2xs px-2">
                  <DbCellValue value={value} />
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
