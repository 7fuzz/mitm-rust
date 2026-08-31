import React, { useState, useEffect } from 'react';
import { getDatabaseTables, getTableData } from '../../../services/tauri/ipc';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Select } from '../../common/ui';

export const SqliteBrowser: React.FC = () => {
  const [tables, setTables] = useState<string[]>([]);
  const [selectedTable, setSelectedTable] = useState<string>('');
  const [tableData, setTableData] = useState<{ columns: string[]; rows: any[][] }>({ columns: [], rows: [] });
  const [customSql, setCustomSql] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    async function loadTables() {
      try {
        const tbls = await getDatabaseTables();
        setTables(tbls);
        if (tbls.length > 0) {
          setSelectedTable(tbls[0]);
        }
      } catch (err) {
        console.error('Failed to load DB tables:', err);
      }
    }
    loadTables();
  }, []);

  useEffect(() => {
    if (!selectedTable) return;
    async function loadData() {
      setIsLoading(true);
      try {
        const res = await getTableData(selectedTable, 100);
        setTableData(res);
      } catch (err) {
        console.error('Failed to fetch table data:', err);
      }
      setIsLoading(false);
    }
    loadData();
  }, [selectedTable]);

  const handleExecuteCustomSql = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!customSql.trim()) return;
    setIsLoading(true);
    try {
      const res = await getTableData(selectedTable, 100, customSql.trim());
      setTableData(res);
    } catch (err) {
      console.error('Failed to execute SQL query:', err);
    }
    setIsLoading(false);
  };

  return (
    <div className="bg-surface border border-border rounded-lg p-3 space-y-3 text-xs shadow-2xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <MingCuteIcon name="storage_line" size={16} className="text-primary" />
          <span className="font-semibold text-foreground text-sm">Embedded SQLite Database Browser (mitm.db)</span>
        </div>

        {/* Table Selector */}
        <Select
          value={selectedTable}
          onChange={(e) => setSelectedTable(e.target.value)}
          options={tables.map((tbl) => ({ value: tbl, label: `Table: ${tbl}` }))}
        />
      </div>

      {/* SQL Query Bar */}
      <form onSubmit={handleExecuteCustomSql} className="flex items-center gap-2 font-mono">
        <input
          type="text"
          value={customSql}
          onChange={(e) => setCustomSql(e.target.value)}
          placeholder={`SELECT * FROM ${selectedTable || 'traffic_history'} LIMIT 100;`}
          className="flex-1 bg-background border border-border rounded px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <button
          type="submit"
          disabled={isLoading}
          className="px-4 py-1.5 bg-primary text-primary-foreground font-sans font-semibold rounded hover:bg-primary-hover transition-colors shadow-xs"
        >
          Execute Query
        </button>
      </form>

      {/* Read-Only Grid Table */}
      <div className="border border-border rounded overflow-x-auto max-h-72">
        <table className="w-full text-left font-mono text-xs border-collapse">
          <thead className="bg-header border-b border-border text-muted-foreground text-[11px] sticky top-0">
            <tr>
              {tableData.columns.map((col) => (
                <th key={col} className="px-3 py-1.5 font-bold font-sans">
                  {col}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {tableData.rows.length === 0 ? (
              <tr>
                <td colSpan={Math.max(1, tableData.columns.length)} className="px-3 py-4 text-center text-muted-foreground italic font-sans">
                  {isLoading ? 'Loading database records...' : 'No records found in table'}
                </td>
              </tr>
            ) : (
              tableData.rows.map((row, rIdx) => (
                <tr key={rIdx} className="hover:bg-neutral-subtle/50">
                  {row.map((cell, cIdx) => (
                    <td key={cIdx} className="px-3 py-1.5 truncate max-w-xs text-foreground">
                      {typeof cell === 'object' ? JSON.stringify(cell) : String(cell)}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
};
