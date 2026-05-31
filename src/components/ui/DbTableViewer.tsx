import { useState, useMemo } from 'react';

interface TableData {
  table: string;
  columns: string[];
  row_count: number;
  rows: any[];
  limit: number;
}

export default function DbTableViewer({ data }: { data: TableData }) {
  const [page, setPage] = useState(1);
  const pageSize = Math.max(10, Math.min(500, data.limit || 100));

  const totalPages = Math.max(1, Math.ceil((data.rows?.length || 0) / pageSize));

  const currentRows = useMemo(() => {
    const start = (page - 1) * pageSize;
    return data.rows.slice(start, start + pageSize);
  }, [data.rows, page, pageSize]);

  const downloadCsv = () => {
    const cols = data.columns;
    const lines = [cols.join(',')];
    for (const r of data.rows) {
      const row = cols.map((c) => {
        const v = r[c];
        if (v === null || v === undefined) return '';
        if (typeof v === 'object') return '"' + JSON.stringify(v).replace(/"/g, '""') + '"';
        const s = String(v);
        if (s.includes(',') || s.includes('\n') || s.includes('"')) return '"' + s.replace(/"/g, '""') + '"';
        return s;
      });
      lines.push(row.join(','));
    }

    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${data.table || 'table'}.csv`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  const copyRowJson = (row: any) => {
    try {
      navigator.clipboard.writeText(JSON.stringify(row, null, 2));
    } catch {
      // ignore
    }
  };

  return (
    <div className="w-full h-full overflow-auto">
      <div className="mb-3 flex items-center justify-between">
        <div className="text-sm text-zinc-400">Showing {currentRows.length} of {data.row_count} rows</div>
        <div className="flex items-center gap-2">
          <button onClick={downloadCsv} className="px-3 py-1 text-xs bg-emerald-600 hover:bg-emerald-500 text-black font-bold rounded">Export CSV</button>
        </div>
      </div>

      <div className="overflow-auto border border-zinc-800 rounded bg-zinc-950">
        <table className="min-w-full text-sm">
          <thead className="sticky top-0 bg-zinc-900 text-zinc-400">
            <tr>
              {data.columns.map((c) => (
                <th key={c} className="px-3 py-2 text-left font-mono text-xs">{c}</th>
              ))}
              <th className="px-3 py-2 text-left font-mono text-xs">_actions</th>
            </tr>
          </thead>
          <tbody>
            {currentRows.map((r, i) => (
              <tr key={i} className="border-t border-zinc-800 even:bg-zinc-900/20">
                {data.columns.map((c) => (
                  <td key={c} className="px-3 py-2 align-top font-mono text-xs text-zinc-300 max-w-xs truncate">{typeof r[c] === 'object' ? JSON.stringify(r[c]) : String(r[c] ?? '')}</td>
                ))}
                <td className="px-3 py-2 align-top">
                  <button onClick={() => copyRowJson(r)} className="px-2 py-1 text-xs bg-zinc-800 hover:bg-zinc-700 rounded">Copy JSON</button>
                </td>
              </tr>
            ))}
            {currentRows.length === 0 && (
              <tr>
                <td colSpan={data.columns.length + 1} className="px-4 py-8 text-center text-zinc-500">No rows</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="mt-3 flex items-center justify-between">
        <div className="text-xs text-zinc-400">Page {page} / {totalPages}</div>
        <div className="flex gap-2">
          <button onClick={() => setPage(1)} disabled={page === 1} className="px-2 py-1 bg-zinc-800 rounded text-xs">First</button>
          <button onClick={() => setPage(p => Math.max(1, p - 1))} disabled={page === 1} className="px-2 py-1 bg-zinc-800 rounded text-xs">Prev</button>
          <button onClick={() => setPage(p => Math.min(totalPages, p + 1))} disabled={page === totalPages} className="px-2 py-1 bg-zinc-800 rounded text-xs">Next</button>
          <button onClick={() => setPage(totalPages)} disabled={page === totalPages} className="px-2 py-1 bg-zinc-800 rounded text-xs">Last</button>
        </div>
      </div>
    </div>
  );
}
