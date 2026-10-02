import React, { useEffect, useRef, useState } from 'react';
import { runDbQuery, type DbQueryResult } from '../../../../services/tauri/bridge';
import { CodeEditor } from '../../../common/CodeEditor';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { settingsButtonClass } from '../SettingsSection';
import { DbGrid } from './DbGrid';

interface DbQueryPanelProps {
  sql: string;
  onSqlChange: (sql: string) => void;
}

/** Free-form SQL against a read-only connection. */
export const DbQueryPanel: React.FC<DbQueryPanelProps> = ({ sql, onSqlChange }) => {
  const [result, setResult] = useState<DbQueryResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const run = async () => {
    if (!sql.trim() || isRunning) return;
    setIsRunning(true);
    setError(null);
    try {
      setResult(await runDbQuery(sql));
    } catch (err) {
      setError(String(err));
      setResult(null);
    } finally {
      setIsRunning(false);
    }
  };

  // Ctrl/Cmd+Enter runs the query while focus is inside this panel
  const runRef = useRef(run);
  runRef.current = run;
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && containerRef.current?.contains(document.activeElement)) {
        e.preventDefault();
        runRef.current();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div ref={containerRef} className="h-full flex flex-col overflow-hidden">
      <div className="h-36 shrink-0 border-b border-border">
        <CodeEditor value={sql} onChange={onSqlChange} language="sql" height="100%" bare />
      </div>

      <div className="h-10 px-3 bg-header border-b border-border flex items-center gap-3 shrink-0">
        <button onClick={run} disabled={isRunning || !sql.trim()} className={settingsButtonClass.primary}>
          <MingCuteIcon name={isRunning ? 'loading_line' : 'play_line'} size={13} className={isRunning ? 'animate-spin' : ''} />
          <span>Run</span>
          <span className="text-[10px] opacity-70 font-mono">Ctrl+↵</span>
        </button>
        <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
          <MingCuteIcon name="lock_line" size={12} />
          Read-only: statements that write are rejected
        </span>
        {result && (
          <span className="ml-auto font-mono text-[11px] text-muted-foreground tabular-nums">
            {result.rows.length.toLocaleString()} row{result.rows.length === 1 ? '' : 's'}
            {result.truncated && ' (first 1,000 shown)'} · {result.elapsedMs}ms
          </span>
        )}
      </div>

      <div className="flex-1 min-h-0">
        {error ? (
          <div className="m-3 p-2 rounded border border-rose-500/30 bg-rose-500/10 text-rose-500 font-mono text-[11px] break-all">{error}</div>
        ) : result ? (
          <DbGrid columns={result.columns} rows={result.rows} emptyMessage="Query returned no rows" />
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground italic">
            Write a query and press Run. Click a table on the left to start from it.
          </div>
        )}
      </div>
    </div>
  );
};
