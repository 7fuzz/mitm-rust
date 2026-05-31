import { useState, useMemo, useEffect } from 'react';
import { useTraffic } from '@/hooks/traffic';
import JsonViewer from '@/components/ui/JsonViewer';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { Button } from '../ui/Button';
import { invoke } from '@/lib/utils/tauri';

interface TableData {
  table: string;
  columns: string[];
  row_count: number;
  rows: any[];
  limit: number;
}

export function DebugView() {
  const { logs, clearLogs, prefs, uiLayout, updateUILayout } = useTraffic();
  const [selectedLogId, setSelectedLogId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'logs' | 'database'>('logs');
  
  // Database viewer state
  const [dbTables, setDbTables] = useState<string[]>([]);
  const [selectedTable, setSelectedTable] = useState<string | null>(null);
  const [tableData, setTableData] = useState<TableData | null>(null);
  const [isLoadingDb, setIsLoadingDb] = useState(false);
  const [dbLimit, setDbLimit] = useState(100);

  const selectedLog = logs.find(l => l.id === selectedLogId);

  // Load database tables on mount
  useEffect(() => {
    if (viewMode === 'database') {
      loadDatabaseTables();
    }
  }, [viewMode]);

  const loadDatabaseTables = async () => {
    try {
      const tables = await invoke<string[]>('get_database_tables', {});
      setDbTables(tables);
      if (tables.length > 0 && !selectedTable) {
        setSelectedTable(tables[0]);
      }
    } catch (error) {
      console.error('Failed to load database tables:', error);
    }
  };

  const loadTableData = async (tableName: string) => {
    if (!tableName) return;
    setIsLoadingDb(true);
    try {
      const data = await invoke<TableData>('get_table_data', { tableName, limit: dbLimit });
      setTableData(data);
    } catch (error) {
      console.error('Failed to load table data:', error);
      setTableData(null);
    } finally {
      setIsLoadingDb(false);
    }
  };

  // Load data when selected table changes
  useEffect(() => {
    if (selectedTable) {
      loadTableData(selectedTable);
    }
  }, [selectedTable, dbLimit]);

  const logsListElement = useMemo(() => (
    <div className="flex flex-col h-full bg-zinc-950">
      <div className="flex-1 overflow-y-auto">
        <table className="w-full text-left border-collapse">
          <thead className="sticky top-0 bg-zinc-900 shadow-sm z-10">
            <tr className="text-[10px] font-black uppercase tracking-wider text-zinc-500 border-b border-zinc-800">
              <th className="px-4 py-2 w-20">Type</th>
              <th className="px-4 py-2">Target</th>
              <th className="px-4 py-2 w-16 text-right">Dir</th>
            </tr>
          </thead>
          <tbody>
            {logs.map((log) => (
              <tr
                key={log.id}
                onClick={() => setSelectedLogId(log.id)}
                className={`group cursor-pointer border-b border-zinc-800/50 hover:bg-zinc-900/50 transition-colors ${selectedLogId === log.id ? 'bg-emerald-500/10' : ''}`}
              >
                <td className="px-4 py-2">
                  <span className={`text-[9px] font-black uppercase px-1.5 py-0.5 rounded ${
                    log.type === 'invoke' ? 'bg-blue-500/20 text-blue-400' :
                    log.type === 'event' ? 'bg-purple-500/20 text-purple-400' :
                    log.type === 'response' ? 'bg-emerald-500/20 text-emerald-400' :
                    'bg-rose-500/20 text-rose-400'
                  }`}>
                    {log.type}
                  </span>
                </td>
                <td className="px-4 py-2 text-[11px] font-bold text-zinc-300 truncate max-w-0">
                  {log.target}
                </td>
                <td className="px-4 py-2 text-right">
                  <span className={`text-[9px] font-bold ${log.direction === 'to-backend' ? 'text-blue-400' : 'text-emerald-400'}`}>
                    {log.direction === 'to-backend' ? '→' : '←'}
                  </span>
                </td>
              </tr>
            ))}
            {logs.length === 0 && (
              <tr>
                <td colSpan={3} className="px-4 py-20 text-center text-zinc-600 italic text-xs">
                  No communication logs.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  ), [logs, selectedLogId]);

  const databaseListElement = useMemo(() => (
    <div className="flex flex-col h-full bg-zinc-950">
      <div className="flex-1 overflow-y-auto">
        <div className="space-y-1 p-2">
          {dbTables.map((table) => (
            <button
              key={table}
              onClick={() => setSelectedTable(table)}
              className={`w-full text-left px-3 py-2 rounded text-sm font-mono transition-colors ${
                selectedTable === table
                  ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                  : 'text-zinc-400 hover:bg-zinc-800/50 border border-transparent'
              }`}
            >
              <span className="text-[11px] font-bold">{table}</span>
            </button>
          ))}
          {dbTables.length === 0 && (
            <div className="px-3 py-8 text-center text-zinc-600 italic text-xs">
              No tables found.
            </div>
          )}
        </div>
      </div>
    </div>
  ), [dbTables, selectedTable]);

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      <WorkspaceLayout
        uiLayout={uiLayout}
        onUpdateLayout={updateUILayout}
        listComponent={() => viewMode === 'logs' ? logsListElement : databaseListElement}
        toolbarLeft={
          <div className="flex items-center gap-4">
            <div className="flex gap-2 bg-zinc-800/50 p-1 rounded-lg">
              <button
                onClick={() => setViewMode('logs')}
                className={`px-3 py-1 rounded text-xs font-bold uppercase tracking-wider transition-colors ${
                  viewMode === 'logs'
                    ? 'bg-blue-500/30 text-blue-400'
                    : 'text-zinc-400 hover:text-zinc-300'
                }`}
              >
                Logs
              </button>
              <button
                onClick={() => setViewMode('database')}
                className={`px-3 py-1 rounded text-xs font-bold uppercase tracking-wider transition-colors ${
                  viewMode === 'database'
                    ? 'bg-purple-500/30 text-purple-400'
                    : 'text-zinc-400 hover:text-zinc-300'
                }`}
              >
                Database
              </button>
            </div>
            {viewMode === 'logs' ? (
              <h2 className="text-xs font-black uppercase tracking-widest text-zinc-400">Backend_Communication</h2>
            ) : (
              <h2 className="text-xs font-black uppercase tracking-widest text-zinc-400">SQLite_Database</h2>
            )}
            <span className="text-[10px] text-zinc-500 bg-zinc-800 px-2 py-0.5 rounded-full">
              {viewMode === 'logs' ? `${logs.length} Entries` : `${dbTables.length} Tables`}
            </span>
          </div>
        }
        toolbarRight={
          <div className="flex gap-2">
            {viewMode === 'logs' && (
              <Button
                variant="destructive"
                size="sm"
                onClick={clearLogs}
              >
                Clear_Logs
              </Button>
            )}
            {viewMode === 'database' && (
              <div className="flex items-center gap-2">
                <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-wider">Limit:</span>
                <select
                  value={dbLimit}
                  onChange={(e) => setDbLimit(Number(e.target.value))}
                  className="bg-zinc-800 border border-zinc-700 text-zinc-400 text-xs rounded px-2 py-1 focus:outline-none focus:border-emerald-500"
                >
                  <option value={10}>10</option>
                  <option value={50}>50</option>
                  <option value={100}>100</option>
                  <option value={500}>500</option>
                  <option value={1000}>1000</option>
                </select>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => loadTableData(selectedTable!)}
                  disabled={isLoadingDb || !selectedTable}
                >
                  Refresh
                </Button>
              </div>
            )}
          </div>
        }
        mainContent={() => (
          <div className="h-full flex flex-col">
            {viewMode === 'logs' ? (
              selectedLog ? (
                <div className="flex flex-col h-full space-y-4">
                  <header className="flex flex-col border-b border-zinc-800 pb-4">
                    <div className="flex justify-between items-center mb-2">
                      <span className={`text-[11px] font-black uppercase px-2 py-1 rounded ${
                        selectedLog.type === 'invoke' ? 'bg-blue-500/20 text-blue-400' :
                        selectedLog.type === 'event' ? 'bg-purple-500/20 text-purple-400' :
                        selectedLog.type === 'response' ? 'bg-emerald-500/20 text-emerald-400' :
                        'bg-rose-500/20 text-rose-400'
                      }`}>
                        {selectedLog.type}
                      </span>
                      <span className="text-[10px] font-mono text-zinc-500">
                        {new Date(selectedLog.timestamp).toLocaleString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 })}
                      </span>
                    </div>
                    <h1 className="text-lg font-black text-zinc-200 tracking-tight break-all">
                      {selectedLog.target}
                    </h1>
                    <div className="mt-2 flex items-center gap-2">
                      <span className={`text-[10px] font-black uppercase tracking-widest ${selectedLog.direction === 'to-backend' ? 'text-blue-400' : 'text-emerald-400'}`}>
                        {selectedLog.direction === 'to-backend' ? 'Direction: To Backend' : 'Direction: From Backend'}
                      </span>
                    </div>
                  </header>

                  <div className="flex-1 min-h-0 bg-zinc-900/20 border border-zinc-800/50 rounded-lg p-4 overflow-auto custom-scrollbar">
                    <h3 className="text-zinc-500 text-[10px] font-bold uppercase tracking-widest mb-4 flex items-center gap-2">
                      <span className="opacity-50">#</span> Payload_Data
                    </h3>
                    <JsonViewer value={selectedLog.payload} path="debug-payload" />
                  </div>
                </div>
              ) : (
                <div className="flex-1 flex flex-col items-center justify-center text-zinc-600 gap-4 opacity-30 grayscale pointer-events-none select-none">
                  <div className="text-[40px] font-black tracking-tighter">DEBUG_IDLE</div>
                </div>
              )
            ) : (
              // Database viewer
              selectedTable ? (
                <div className="flex flex-col h-full space-y-4">
                  <header className="flex flex-col border-b border-zinc-800 pb-4">
                    <div className="flex justify-between items-center mb-2">
                      <span className="text-[11px] font-black uppercase px-2 py-1 rounded bg-purple-500/20 text-purple-400">
                        Table
                      </span>
                      {tableData && (
                        <span className="text-[10px] font-mono text-zinc-500">
                          {tableData.rows.length} / {tableData.row_count} rows
                        </span>
                      )}
                    </div>
                    <h1 className="text-lg font-black text-zinc-200 tracking-tight break-all">
                      {selectedTable}
                    </h1>
                    {tableData && (
                      <div className="mt-2 flex items-center gap-3 text-[10px]">
                        <span className="text-zinc-400">
                          <span className="font-bold text-emerald-400">{tableData.columns.length}</span> columns
                        </span>
                        <span className="text-zinc-400">
                          <span className="font-bold text-emerald-400">{tableData.row_count}</span> total rows
                        </span>
                      </div>
                    )}
                  </header>

                  {isLoadingDb ? (
                    <div className="flex-1 flex items-center justify-center text-zinc-500">
                      <div className="animate-pulse text-sm">Loading table data...</div>
                    </div>
                  ) : tableData ? (
                    <div className="flex-1 min-h-0 bg-zinc-900/20 border border-zinc-800/50 rounded-lg p-4 overflow-auto custom-scrollbar">
                      <h3 className="text-zinc-500 text-[10px] font-bold uppercase tracking-widest mb-4 flex items-center gap-2">
                        <span className="opacity-50">#</span> Table_Data
                      </h3>
                      <JsonViewer value={tableData.rows} path="db-table-data" />
                    </div>
                  ) : (
                    <div className="flex-1 flex items-center justify-center text-zinc-500">
                      <div className="text-sm">Failed to load table data</div>
                    </div>
                  )}
                </div>
              ) : (
                <div className="flex-1 flex flex-col items-center justify-center text-zinc-600 gap-4 opacity-30 grayscale pointer-events-none select-none">
                  <div className="text-[40px] font-black tracking-tighter">NO_TABLE</div>
                </div>
              )
            )}
          </div>
        )}
      >
        {!prefs.debugMode && (
          <div className="bg-rose-500/10 border-b border-rose-500/20 px-4 py-2 flex items-center gap-3 shrink-0">
            <div className="w-2 h-2 rounded-full bg-rose-500 animate-pulse" />
            <p className="text-[10px] text-rose-400 font-bold uppercase tracking-wider">
              Debug Mode is OFF. To see live logs, enable it in Options.
            </p>
          </div>
        )}
      </WorkspaceLayout>
    </div>
  );
}
