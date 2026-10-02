import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { listDbTables, type DbTable } from '../../../services/tauri/bridge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl } from '../../common/ui';
import { SettingsSection } from './SettingsSection';
import { DbBrowsePanel } from './database/DbBrowsePanel';
import { DbQueryPanel } from './database/DbQueryPanel';

/** Explorer for mitm.db: table list on the left, browse or query on the right. */
export const DatabaseSection: React.FC = () => {
  const [tables, setTables] = useState<DbTable[]>([]);
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [mode, setMode] = useState<'browse' | 'query'>('browse');
  const [tableFilter, setTableFilter] = useState('');
  const [sql, setSql] = useState('');
  const [refreshKey, setRefreshKey] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const loadTables = useCallback(async () => {
    try {
      const list = await listDbTables();
      setTables(list);
      setError(null);
      setSelectedName((current) => (current && list.some((t) => t.name === current) ? current : list[0]?.name ?? null));
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    loadTables();
  }, [loadTables]);

  const refresh = () => {
    loadTables();
    setRefreshKey((k) => k + 1);
  };

  const filteredTables = useMemo(() => {
    const q = tableFilter.trim().toLowerCase();
    return q ? tables.filter((t) => t.name.toLowerCase().includes(q)) : tables;
  }, [tables, tableFilter]);

  const selectedTable = tables.find((t) => t.name === selectedName) ?? null;

  const handleTableClick = (table: DbTable) => {
    setSelectedName(table.name);
    if (mode === 'query') setSql(`SELECT * FROM ${table.name} LIMIT 100;`);
  };

  return (
    <SettingsSection
      title="Database"
      description="Explore the tables in mitm.db. Everything here is read-only."
      className="h-full flex flex-col"
      aside={
        <div className="flex items-center gap-2">
          <SegmentedControl
            sizeVariant="sm"
            value={mode}
            onChange={(v) => setMode(v as typeof mode)}
            options={[
              { value: 'browse', label: 'Browse' },
              { value: 'query', label: 'Query' },
            ]}
          />
          <button
            onClick={refresh}
            className="p-1.5 rounded border border-border bg-background text-muted-foreground hover:text-foreground cursor-pointer"
            title="Reload tables and rows"
          >
            <MingCuteIcon name="refresh_line" size={14} />
          </button>
        </div>
      }
    >
      <div className="flex-1 min-h-0 flex rounded-lg border border-border bg-surface overflow-hidden">
        {/* Table list */}
        <div className="w-56 shrink-0 border-r border-border flex flex-col">
          <div className="p-2 border-b border-border">
            <div className="relative">
              <MingCuteIcon name="search_line" size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <input
                type="text"
                value={tableFilter}
                onChange={(e) => setTableFilter(e.target.value)}
                placeholder={`Filter ${tables.length} tables...`}
                className="w-full bg-background border border-border rounded pl-7 pr-2 py-1 text-[11px] font-mono text-foreground focus:outline-none focus:border-primary"
              />
            </div>
          </div>
          <div className="flex-1 overflow-y-auto py-1">
            {filteredTables.map((table) => {
              const isActive = table.name === selectedName;
              return (
                <button
                  key={table.name}
                  onClick={() => handleTableClick(table)}
                  className={`w-full px-3 py-1.5 flex items-center gap-2 text-left font-mono text-[11px] border-l-2 cursor-pointer transition-colors ${
                    isActive
                      ? 'bg-primary/10 border-l-primary text-foreground font-semibold'
                      : 'border-l-transparent text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
                  }`}
                  title={mode === 'query' ? 'Start a query from this table' : undefined}
                >
                  <MingCuteIcon name="grid_line" size={12} className="shrink-0 opacity-60" />
                  <span className="truncate flex-1">{table.name}</span>
                  <span className="text-[10px] text-muted-foreground tabular-nums font-normal">{table.rowCount.toLocaleString()}</span>
                </button>
              );
            })}
            {error && <div className="px-3 py-2 text-rose-500 text-[11px] font-mono break-all">{error}</div>}
          </div>
        </div>

        {/* Browse / Query */}
        <div className="flex-1 min-w-0">
          {mode === 'query' ? (
            <DbQueryPanel sql={sql} onSqlChange={setSql} />
          ) : selectedTable ? (
            <DbBrowsePanel key={selectedTable.name} table={selectedTable} refreshKey={refreshKey} />
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground italic">
              {tables.length === 0 && !error ? 'Loading tables...' : 'Select a table'}
            </div>
          )}
        </div>
      </div>
    </SettingsSection>
  );
};
