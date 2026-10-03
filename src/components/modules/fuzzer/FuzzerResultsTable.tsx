import React, { useEffect, useMemo, useState } from 'react';
import { useFuzzerStore, type FuzzRow } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { StatusBadge } from '../../common/StatusBadge';
import { RunDetailsDialog } from '../../common/RunDetailsDialog';
import { Input, SegmentedControl, TriStateFilter, type TriState } from '../../common/ui';
import { searchFuzzResponses } from '../../../services/tauri/bridge';
import type { RunRecord } from '../../../utils/repeaterTraffic';

type SortKey = 'idx' | 'payloads' | 'statusCode' | 'responseSize' | 'durationMs' | string;

const formatSize = (b: number) => (b < 1024 ? `${b} B` : `${(b / 1024).toFixed(1)} KB`);

const STATUS_OPTIONS = [
  { value: '2xx', label: '2xx' },
  { value: '3xx', label: '3xx' },
  { value: '4xx', label: '4xx' },
  { value: '5xx', label: '5xx' },
  { value: 'err', label: 'ERR' },
] as const;

type SearchScope = 'table' | 'response';

const SCOPE_OPTIONS = [
  { value: 'table', label: 'Table' },
  { value: 'response', label: 'Response' },
] as const;

const statusClass = (row: FuzzRow) => (row.error ? 'err' : `${Math.floor(row.statusCode / 100)}xx`);

const mostCommonSize = (rows: FuzzRow[]) => {
  const counts = new Map<number, number>();
  let best: number | null = null;
  let bestCount = 0;
  for (const row of rows) {
    if (row.error) continue;
    const n = (counts.get(row.responseSize) ?? 0) + 1;
    counts.set(row.responseSize, n);
    if (n > bestCount) {
      best = row.responseSize;
      bestCount = n;
    }
  }
  return best;
};

export const FuzzerResultsTable: React.FC<{ onHide: () => void }> = ({ onHide }) => {
  const { rows, total, phase, runId, config, variables, fetchResultDetail } = useFuzzerStore();
  const matchNames = config.matchRules.map((r) => r.name);
  const varNames = variables.map((v) => v.name);
  const labelPayloads = (payloads: string[]) =>
    payloads.map((value, i) => (varNames[i] ? `${varNames[i]}=${value}` : value)).join('  ');

  const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: 'idx', desc: false });
  const [detail, setDetail] = useState<RunRecord | null>(null);
  const [search, setSearch] = useState('');
  const [statusFilters, setStatusFilters] = useState<Record<string, TriState>>({});
  const [hideCommonSize, setHideCommonSize] = useState(false);
  const [searchScope, setSearchScope] = useState<SearchScope>('table');
  const [responseHits, setResponseHits] = useState<Set<number> | null>(null);

  const query = search.trim();
  const searchingResponses = searchScope === 'response' && query !== '';

  useEffect(() => {
    if (!searchingResponses || !runId) {
      setResponseHits(null);
      return;
    }
    let cancelled = false;
    const run = () =>
      searchFuzzResponses(runId, query)
        .then((idxs) => !cancelled && setResponseHits(new Set(idxs)))
        .catch((err) => console.error('Response search failed:', err));
    const debounce = setTimeout(run, 250);
    // Results keep arriving while running, so re-query periodically instead of on every row
    const poll = phase === 'running' ? setInterval(run, 1500) : undefined;
    return () => {
      cancelled = true;
      clearTimeout(debounce);
      clearInterval(poll);
    };
  }, [searchingResponses, runId, query, phase]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    const included = Object.keys(statusFilters).filter((k) => statusFilters[k] === 'include');
    const excluded = Object.keys(statusFilters).filter((k) => statusFilters[k] === 'exclude');
    const commonSize = hideCommonSize ? mostCommonSize(rows) : null;
    return rows.filter((row) => {
      const cls = statusClass(row);
      if (included.length > 0 && !included.includes(cls)) return false;
      if (excluded.includes(cls)) return false;
      if (commonSize !== null && !row.error && row.responseSize === commonSize) return false;
      if (!q) return true;
      if (searchingResponses) return responseHits?.has(row.idx) ?? false;
      const haystack = [
        ...row.payloads,
        ...row.matches.map((m) => m.value),
        String(row.statusCode),
        String(row.responseSize),
        row.error ?? '',
      ];
      return haystack.some((s) => s.toLowerCase().includes(q));
    });
  }, [rows, query, searchingResponses, responseHits, statusFilters, hideCommonSize]);

  const isFiltering = query !== '' || hideCommonSize || Object.values(statusFilters).some((v) => v !== 'neutral');

  const sorted = useMemo(() => {
    const matchVal = (row: FuzzRow, name: string) => row.matches.find((m) => m.name === name)?.value ?? '';
    const val = (row: FuzzRow): string | number => {
      if (sort.key === 'payloads') return row.payloads.join(', ');
      if (matchNames.includes(sort.key as string)) {
        const v = matchVal(row, sort.key as string);
        const n = Number(v);
        return Number.isNaN(n) ? v : n;
      }
      return (row as any)[sort.key] ?? 0;
    };
    return [...filtered].sort((a, b) => {
      const av = val(a);
      const bv = val(b);
      const cmp = typeof av === 'number' && typeof bv === 'number' ? av - bv : String(av).localeCompare(String(bv));
      return sort.desc ? -cmp : cmp;
    });
  }, [filtered, sort, matchNames]);

  const openDetail = async (idx: number) => {
    const full = await fetchResultDetail(idx);
    if (!full) return;
    setDetail({
      id: full.idx,
      method: full.method,
      url: full.url,
      requestHeaders: full.requestHeaders,
      requestBody: full.requestBody,
      statusCode: full.statusCode,
      responseHeaders: full.responseHeaders,
      responseBody: full.responseBody ?? full.error ?? '',
      durationMs: full.durationMs,
      executedAtMs: Date.now(),
    });
  };

  const header = (key: SortKey, label: string, extra = '') => (
    <th
      onClick={() => setSort((s) => ({ key, desc: s.key === key ? !s.desc : false }))}
      className={`px-2 py-1.5 font-semibold font-sans cursor-pointer select-none hover:text-foreground whitespace-nowrap ${extra}`}
    >
      <span className="inline-flex items-center gap-1">
        {label}
        {sort.key === key && <MingCuteIcon name={sort.desc ? 'arrow_down_line' : 'arrow_up_line'} size={10} className="text-primary" />}
      </span>
    </th>
  );

  return (
    <div className="h-full flex flex-col overflow-hidden">
      <div className="h-8 px-3 border-b border-border flex items-center gap-2 text-2xs text-muted-foreground shrink-0 font-mono">
        <div className="w-56 shrink-0">
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={searchScope === 'response' ? 'Search response headers, body...' : 'Search payload, match, status...'}
            leftIcon="search_line"
            rightIcon={search ? 'close_line' : undefined}
            onRightIconClick={() => setSearch('')}
            className="py-0.5 text-2xs"
          />
        </div>
        <SegmentedControl options={SCOPE_OPTIONS} value={searchScope} onChange={setSearchScope} />
        <TriStateFilter
          items={STATUS_OPTIONS}
          values={statusFilters}
          onChange={(value, next) => setStatusFilters((f) => ({ ...f, [value]: next }))}
        />
        <button
          onClick={() => setHideCommonSize((v) => !v)}
          title="Hide responses with the most frequent length"
          className={`px-2 py-0.5 rounded border font-sans font-semibold cursor-pointer transition-colors ${
            hideCommonSize
              ? 'bg-primary/10 border-primary/40 text-primary'
              : 'border-transparent hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          Hide common length
        </button>
        {isFiltering && (
          <button
            onClick={() => {
              setSearch('');
              setStatusFilters({});
              setHideCommonSize(false);
            }}
            className="font-sans underline cursor-pointer hover:text-foreground"
          >
            Reset
          </button>
        )}
        <span className="ml-auto flex items-center gap-2 whitespace-nowrap">
          {phase === 'running' && <MingCuteIcon name="loading_line" size={12} className="animate-spin text-primary" />}
          {phase === 'stopped' && <span className="text-amber-500">stopped</span>}
          <span>
            {isFiltering && (
              <>
                <span className="text-foreground font-semibold tabular-nums">{filtered.length}</span> of{' '}
              </>
            )}
            <span className={`tabular-nums ${isFiltering ? '' : 'text-foreground font-semibold'}`}>{rows.length}</span>
            {total > 0 ? ` / ${total}` : ''} results
          </span>
          <button
            onClick={onHide}
            className="p-0.5 rounded cursor-pointer hover:text-foreground hover:bg-neutral-subtle"
            title="Hide results"
          >
            <MingCuteIcon name="down_line" size={13} />
          </button>
        </span>
      </div>

      <div className="flex-1 overflow-auto">
        <table className="w-max min-w-full text-left font-mono text-2xs border-collapse">
          <thead className="bg-header text-muted-foreground sticky top-0 z-10 text-3xs uppercase tracking-wider">
            <tr className="border-b border-border">
              {header('idx', '#', 'w-12')}
              {header('payloads', 'Payload')}
              {header('statusCode', 'Status', 'w-20')}
              {header('responseSize', 'Length', 'w-20')}
              {header('durationMs', 'Time', 'w-16')}
              {matchNames.map((name) => header(name, name, 'w-24'))}
            </tr>
          </thead>
          <tbody>
            {sorted.length === 0 ? (
              <tr>
                <td colSpan={5 + matchNames.length} className="px-3 py-12 text-center text-muted-foreground italic font-sans">
                  {rows.length > 0
                    ? 'No results match the filters.'
                    : phase === 'running'
                    ? 'Running...'
                    : 'No results yet. Add {{variables}} and payloads, then Start.'}
                </td>
              </tr>
            ) : (
              sorted.map((row) => (
                <tr
                  key={row.idx}
                  onClick={() => openDetail(row.idx)}
                  className="border-b border-border/50 cursor-pointer hover:bg-neutral-subtle/60"
                >
                  <td className="px-2 py-1 text-muted-foreground tabular-nums">{row.idx + 1}</td>
                  <td className="px-2 py-1 text-foreground max-w-[280px] truncate font-mono" title={labelPayloads(row.payloads)}>
                    {labelPayloads(row.payloads)}
                  </td>
                  <td className="px-2 py-1">
                    {row.error ? <span className="text-rose-500" title={row.error}>ERR</span> : <StatusBadge code={row.statusCode} />}
                  </td>
                  <td className="px-2 py-1 text-muted-foreground tabular-nums">{formatSize(row.responseSize)}</td>
                  <td className="px-2 py-1 text-muted-foreground tabular-nums">{row.durationMs}ms</td>
                  {matchNames.map((name) => (
                    <td key={name} className="px-2 py-1 text-foreground truncate max-w-[160px]">
                      {row.matches.find((m) => m.name === name)?.value ?? ''}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {detail && (
        <RunDetailsDialog
          runs={[detail]}
          index={0}
          onIndexChange={() => {}}
          onClose={() => setDetail(null)}
        />
      )}
    </div>
  );
};
