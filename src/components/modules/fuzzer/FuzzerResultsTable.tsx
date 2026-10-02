import React, { useMemo, useState } from 'react';
import { useFuzzerStore, type FuzzRow } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { StatusBadge } from '../../common/StatusBadge';
import { RunDetailsDialog } from '../../common/RunDetailsDialog';
import type { RunRecord } from '../../../utils/repeaterTraffic';

type SortKey = 'idx' | 'payloads' | 'statusCode' | 'responseSize' | 'durationMs' | string;

const formatSize = (b: number) => (b < 1024 ? `${b} B` : `${(b / 1024).toFixed(1)} KB`);

export const FuzzerResultsTable: React.FC = () => {
  const { rows, total, phase, config, fetchResultDetail } = useFuzzerStore();
  const matchNames = config.matchRules.map((r) => r.name);

  const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: 'idx', desc: false });
  const [detail, setDetail] = useState<RunRecord | null>(null);

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
    return [...rows].sort((a, b) => {
      const av = val(a);
      const bv = val(b);
      const cmp = typeof av === 'number' && typeof bv === 'number' ? av - bv : String(av).localeCompare(String(bv));
      return sort.desc ? -cmp : cmp;
    });
  }, [rows, sort, matchNames]);

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
        <span>
          <span className="text-foreground font-semibold tabular-nums">{rows.length}</span>
          {total > 0 ? ` / ${total}` : ''} results
        </span>
        {phase === 'running' && <MingCuteIcon name="loading_line" size={12} className="animate-spin text-primary" />}
        {phase === 'stopped' && <span className="text-amber-500">stopped</span>}
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
                  {phase === 'running' ? 'Running...' : 'No results yet. Configure positions and payloads, then Start.'}
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
                  <td className="px-2 py-1 text-foreground max-w-[280px] truncate" title={row.payloads.join(' , ')}>
                    {row.payloads.join(' , ')}
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
