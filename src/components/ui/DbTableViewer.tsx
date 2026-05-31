import { useState } from 'react';
import { Modal } from './Modal';
import JsonViewer from './JsonViewer';

interface TableData {
  table: string;
  columns: string[];
  row_count: number;
  rows: any[];
  limit: number;
  offset?: number;
}

interface DbTableViewerProps {
  data: TableData;
  onPageChange?: (offset: number) => void;
}

export default function DbTableViewer({ data, onPageChange }: DbTableViewerProps) {
  const [selectedRow, setSelectedRow] = useState<any | null>(null);
  
  const pageSize = data.limit || 100;
  const currentPage = Math.floor((data.offset || 0) / pageSize) + 1;
  const totalPages = Math.ceil(data.row_count / pageSize);

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

  const handlePageClick = (newPage: number) => {
    if (onPageChange) {
      onPageChange((newPage - 1) * pageSize);
    }
  };

  return (
    <div className="w-full h-full flex flex-col min-h-0">
      <div className="mb-3 flex items-center justify-between shrink-0">
        <div className="text-[10px] text-zinc-500 font-mono uppercase tracking-widest">
          Showing <span className="text-emerald-400 font-bold">{data.rows.length}</span> of <span className="text-emerald-400 font-bold">{data.row_count}</span> rows
        </div>
        <div className="flex items-center gap-2">
          <button onClick={downloadCsv} className="px-3 py-1 text-[9px] font-black uppercase tracking-widest bg-emerald-600/10 border border-emerald-600/30 text-emerald-500 hover:bg-emerald-600/20 rounded transition-all">Export CSV</button>
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-auto border border-zinc-800 rounded bg-zinc-950/50 relative">
        <table className="min-w-full text-xs border-collapse">
          <thead className="sticky top-0 bg-zinc-900 shadow-md z-10">
            <tr>
              {data.columns.map((c) => (
                <th key={c} className="px-3 py-2 text-left font-mono text-[10px] font-black uppercase tracking-widest text-zinc-500 border-b border-zinc-800 whitespace-nowrap">{c}</th>
              ))}
              <th className="px-3 py-2 text-left font-mono text-[10px] font-black uppercase tracking-widest text-zinc-500 border-b border-zinc-800 sticky right-0 bg-zinc-900">_actions</th>
            </tr>
          </thead>
          <tbody>
            {data.rows.map((r, i) => (
              <tr key={i} className="border-b border-zinc-800/50 hover:bg-zinc-900/40 transition-colors group">
                {data.columns.map((c) => (
                  <td key={c} className="px-3 py-2 align-top font-mono text-[11px] text-zinc-400 max-w-xs truncate">
                    {typeof r[c] === 'object' ? (
                      <span className="text-blue-400/80 italic">{"{JSON}"}</span>
                    ) : (
                      String(r[c] ?? '')
                    )}
                  </td>
                ))}
                <td className="px-3 py-2 align-top sticky right-0 bg-zinc-950/80 group-hover:bg-zinc-900 transition-colors backdrop-blur-sm">
                  <div className="flex gap-2">
                    <button 
                      onClick={() => setSelectedRow(r)} 
                      className="px-2 py-0.5 text-[9px] font-bold uppercase tracking-tighter bg-purple-500/10 border border-purple-500/30 text-purple-400 hover:bg-purple-500/20 rounded transition-all"
                    >
                      Details
                    </button>
                    <button 
                      onClick={() => copyRowJson(r)} 
                      className="px-2 py-0.5 text-[9px] font-bold uppercase tracking-tighter bg-zinc-800 border border-zinc-700 text-zinc-400 hover:text-zinc-200 rounded transition-all"
                    >
                      JSON
                    </button>
                  </div>
                </td>
              </tr>
            ))}
            {data.rows.length === 0 && (
              <tr>
                <td colSpan={data.columns.length + 1} className="px-4 py-20 text-center text-zinc-600 font-mono text-[10px] uppercase tracking-[0.2em]">No data found in table</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="mt-4 flex items-center justify-between shrink-0">
        <div className="text-[10px] font-mono text-zinc-500 uppercase tracking-widest">Page <span className="text-zinc-300 font-bold">{currentPage}</span> / <span className="text-zinc-300 font-bold">{totalPages || 1}</span></div>
        <div className="flex items-center gap-1">
          <button 
            onClick={() => handlePageClick(1)} 
            disabled={currentPage === 1} 
            className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-[9px] font-black uppercase tracking-widest text-zinc-500 hover:text-zinc-300 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
          >
            First
          </button>
          <button 
            onClick={() => handlePageClick(currentPage - 1)} 
            disabled={currentPage === 1} 
            className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-[9px] font-black uppercase tracking-widest text-zinc-500 hover:text-zinc-300 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
          >
            Prev
          </button>
          
          <div className="mx-2 flex gap-1">
            {[...Array(Math.min(5, totalPages))].map((_, i) => {
              let pageNum = currentPage;
              if (totalPages <= 5) pageNum = i + 1;
              else if (currentPage <= 3) pageNum = i + 1;
              else if (currentPage >= totalPages - 2) pageNum = totalPages - 4 + i;
              else pageNum = currentPage - 2 + i;
              
              return (
                <button
                  key={pageNum}
                  onClick={() => handlePageClick(pageNum)}
                  className={`w-6 h-6 flex items-center justify-center rounded text-[10px] font-bold border transition-all ${currentPage === pageNum ? 'bg-emerald-500/10 border-emerald-500/50 text-emerald-400' : 'bg-transparent border-zinc-800 text-zinc-500 hover:border-zinc-600 hover:text-zinc-300'}`}
                >
                  {pageNum}
                </button>
              );
            })}
          </div>

          <button 
            onClick={() => handlePageClick(currentPage + 1)} 
            disabled={currentPage >= totalPages} 
            className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-[9px] font-black uppercase tracking-widest text-zinc-500 hover:text-zinc-300 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
          >
            Next
          </button>
          <button 
            onClick={() => handlePageClick(totalPages)} 
            disabled={currentPage >= totalPages} 
            className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-[9px] font-black uppercase tracking-widest text-zinc-500 hover:text-zinc-300 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
          >
            Last
          </button>
        </div>
      </div>

      <Modal
        isOpen={!!selectedRow}
        onClose={() => setSelectedRow(null)}
        title={`Row Details: ${data.table}`}
        maxWidth="lg"
      >
        <div className="p-4 space-y-4 max-h-[70vh] overflow-auto custom-scrollbar">
          <div className="grid grid-cols-1 gap-3">
            {selectedRow && data.columns.map(col => (
              <div key={col} className="space-y-1 bg-zinc-900/30 p-3 rounded border border-zinc-800/50">
                <label className="text-[9px] font-black uppercase tracking-[0.2em] text-zinc-500 block">{col}</label>
                <div className="min-h-[1.5rem]">
                  {typeof selectedRow[col] === 'object' ? (
                    <div className="mt-2 bg-zinc-950 p-2 rounded border border-zinc-800/50">
                      <JsonViewer value={selectedRow[col]} path={`row-${col}`} />
                    </div>
                  ) : (
                    <div className="text-[11px] font-mono text-zinc-300 break-all whitespace-pre-wrap">{String(selectedRow[col] ?? 'NULL')}</div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="p-4 border-t border-zinc-800 flex justify-end">
          <button 
            onClick={() => copyRowJson(selectedRow)} 
            className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-[10px] font-bold uppercase tracking-widest rounded transition-all"
          >
            Copy Full JSON
          </button>
        </div>
      </Modal>
    </div>
  );
}
