import React, { useCallback, useEffect, useState } from 'react';
import { queryDbTable, type DbRows, type DbTable } from '../../../../services/tauri/bridge';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { SegmentedControl } from '../../../common/ui';
import { DbGrid, type DbSort } from './DbGrid';
import { DbRowDetail } from './DbRowDetail';

const PAGE_SIZES = [50, 100, 250, 500];

interface DbBrowsePanelProps {
  table: DbTable;
  /** Bumped by the parent's refresh button */
  refreshKey: number;
}

/** Paged, sortable, searchable view of one table, plus its schema. */
export const DbBrowsePanel: React.FC<DbBrowsePanelProps> = ({ table, refreshKey }) => {
  const [view, setView] = useState<'data' | 'schema'>('data');
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(100);
  const [sort, setSort] = useState<DbSort | null>(null);
  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');
  const [data, setData] = useState<DbRows | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  // Debounce typing into the row search
  useEffect(() => {
    const timer = setTimeout(() => {
      setSearch(searchInput);
      setPage(0);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchInput]);

  const load = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const rows = await queryDbTable({
        table: table.name,
        offset: page * pageSize,
        limit: pageSize,
        orderBy: sort?.column,
        orderDesc: sort?.desc,
        search,
      });
      setData(rows);
    } catch (err) {
      setError(String(err));
      setData(null);
    } finally {
      setIsLoading(false);
    }
  }, [table.name, page, pageSize, sort, search]);

  useEffect(() => {
    load();
    setSelectedIndex(null);
  }, [load, refreshKey]);

  // Header clicks cycle ascending → descending → unsorted
  const handleSort = (column: string) => {
    setPage(0);
    setSort((s) => (s?.column !== column ? { column, desc: false } : !s.desc ? { column, desc: true } : null));
  };

  const total = data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const from = total === 0 ? 0 : page * pageSize + 1;
  const to = Math.min(total, (page + 1) * pageSize);
  const selectedRow = selectedIndex !== null ? data?.rows[selectedIndex] : undefined;

  const navButtonClass =
    'p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed';

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Toolbar */}
      <div className="h-10 px-3 bg-header border-b border-border flex items-center gap-3 shrink-0">
        <span className="font-mono font-semibold text-foreground truncate">{table.name}</span>
        <SegmentedControl
          value={view}
          onChange={(v) => setView(v as typeof view)}
          options={[
            { value: 'data', label: 'Data' },
            { value: 'schema', label: `Schema (${table.columns.length})` },
          ]}
        />
        {view === 'data' && (
          <>
            <div className="relative flex-1 max-w-xs ml-auto">
              <MingCuteIcon name="search_line" size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <input
                type="text"
                value={searchInput}
                onChange={(e) => setSearchInput(e.target.value)}
                placeholder="Search all columns..."
                className="w-full bg-background border border-border rounded pl-7 pr-2 py-1 text-[11px] font-mono text-foreground focus:outline-none focus:border-primary"
              />
            </div>
            <div className="flex items-center gap-1 font-mono text-[11px] text-muted-foreground shrink-0">
              {isLoading && <MingCuteIcon name="loading_line" size={12} className="animate-spin text-primary" />}
              <span className="tabular-nums">
                {from.toLocaleString()}–{to.toLocaleString()} of {total.toLocaleString()}
              </span>
              <button onClick={() => setPage(page - 1)} disabled={page === 0} className={navButtonClass} title="Previous page">
                <MingCuteIcon name="chevron_left_line" size={14} />
              </button>
              <button onClick={() => setPage(page + 1)} disabled={page >= pageCount - 1} className={navButtonClass} title="Next page">
                <MingCuteIcon name="chevron_right_line" size={14} />
              </button>
              <select
                value={pageSize}
                onChange={(e) => {
                  setPageSize(Number(e.target.value));
                  setPage(0);
                }}
                className="bg-background border border-border rounded px-1 py-0.5 text-[11px] text-foreground cursor-pointer focus:outline-none"
                title="Rows per page"
              >
                {PAGE_SIZES.map((size) => (
                  <option key={size} value={size}>
                    {size} / page
                  </option>
                ))}
              </select>
            </div>
          </>
        )}
      </div>

      {view === 'schema' ? (
        <div className="flex-1 overflow-auto p-3 space-y-3">
          <table className="w-full text-left font-mono text-[11px] border border-border rounded">
            <thead className="bg-header text-muted-foreground font-sans">
              <tr>
                <th className="px-3 py-1.5 font-semibold">Column</th>
                <th className="px-3 py-1.5 font-semibold">Type</th>
                <th className="px-3 py-1.5 font-semibold">Constraints</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {table.columns.map((col) => (
                <tr key={col.name}>
                  <td className="px-3 py-1.5 text-foreground font-semibold">{col.name}</td>
                  <td className="px-3 py-1.5 text-sky-500">{col.declType || '—'}</td>
                  <td className="px-3 py-1.5 text-muted-foreground">
                    {[col.primaryKey && 'PRIMARY KEY', col.notNull && 'NOT NULL'].filter(Boolean).join(', ') || '—'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {table.sql && (
            <pre className="font-mono text-[11px] text-muted-foreground whitespace-pre-wrap bg-background border border-border rounded p-2 select-text">
              {table.sql}
            </pre>
          )}
        </div>
      ) : (
        <div className="flex-1 min-h-0 flex">
          <div className="flex-1 min-w-0">
            {error ? (
              <div className="m-3 p-2 rounded border border-rose-500/30 bg-rose-500/10 text-rose-500 font-mono text-[11px] break-all">{error}</div>
            ) : (
              <DbGrid
                columns={data?.columns ?? table.columns.map((c) => c.name)}
                rows={data?.rows ?? []}
                firstRowNumber={from || 1}
                sort={sort}
                onSort={handleSort}
                selectedIndex={selectedIndex}
                onRowClick={(i) => setSelectedIndex(i === selectedIndex ? null : i)}
                emptyMessage={isLoading ? 'Loading...' : search ? 'No rows match the search' : 'Table is empty'}
              />
            )}
          </div>
          {selectedRow && selectedIndex !== null && (
            <div className="w-96 shrink-0 border-l border-border">
              <DbRowDetail
                table={table.name}
                columns={table.columns}
                values={selectedRow}
                rowid={data?.rowids?.[selectedIndex]}
                onClose={() => setSelectedIndex(null)}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
};
