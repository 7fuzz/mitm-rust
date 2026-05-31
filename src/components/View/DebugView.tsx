import { useState } from 'react';
import { useTraffic } from '@/hooks/traffic';
import JsonViewer from '@/components/ui/JsonViewer';

export function DebugView() {
  const { logs, clearLogs, prefs } = useTraffic();
  const [selectedLogId, setSelectedLogId] = useState<string | null>(null);

  const selectedLog = logs.find(l => l.id === selectedLogId);

  return (
    <div className="flex flex-col h-full bg-zinc-950 overflow-hidden">
      <div className="flex items-center justify-between px-4 py-2 border-b border-zinc-800 bg-zinc-900/50">
        <div className="flex items-center gap-4">
          <h2 className="text-xs font-black uppercase tracking-widest text-zinc-400">Backend_Communication_Logs</h2>
          <span className="text-[10px] text-zinc-500 bg-zinc-800 px-2 py-0.5 rounded-full">{logs.length} Entries</span>
        </div>
        <div className="flex gap-2">
          <button
            onClick={clearLogs}
            className="px-3 py-1 text-[10px] font-bold uppercase tracking-wider bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded transition-all"
          >
            Clear_Logs
          </button>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden">
        {/* Logs List */}
        <div className="w-1/2 border-r border-zinc-800 overflow-y-auto">
          <table className="w-full text-left border-collapse">
            <thead className="sticky top-0 bg-zinc-900 shadow-sm z-10">
              <tr className="text-[10px] font-black uppercase tracking-wider text-zinc-500 border-b border-zinc-800">
                <th className="px-4 py-2 w-24">Time</th>
                <th className="px-4 py-2 w-20">Type</th>
                <th className="px-4 py-2">Target</th>
                <th className="px-4 py-2 w-24">Dir</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <tr
                  key={log.id}
                  onClick={() => setSelectedLogId(log.id)}
                  className={`group cursor-pointer border-b border-zinc-800/50 hover:bg-zinc-900/50 transition-colors ${selectedLogId === log.id ? 'bg-emerald-500/10' : ''}`}
                >
                  <td className="px-4 py-2 text-[10px] text-zinc-500 font-mono">
                    {new Date(log.timestamp).toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 })}
                  </td>
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
                  <td className="px-4 py-2">
                    <span className={`text-[9px] font-bold ${log.direction === 'to-backend' ? 'text-blue-400' : 'text-emerald-400'}`}>
                      {log.direction === 'to-backend' ? '→ BACKEND' : '← FRONTEND'}
                    </span>
                  </td>
                </tr>
              ))}
              {logs.length === 0 && (
                <tr>
                  <td colSpan={4} className="px-4 py-20 text-center text-zinc-600 italic text-xs">
                    No communication logs captured yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Log Details */}
        <div className="w-1/2 flex flex-col bg-zinc-900/30 overflow-hidden">
          {selectedLog ? (
            <div className="flex flex-col h-full">
              <div className="px-4 py-2 border-b border-zinc-800 bg-zinc-900/50 flex justify-between items-center">
                <span className="text-[10px] font-black uppercase tracking-widest text-zinc-400">Payload_Data</span>
                <span className="text-[9px] font-mono text-zinc-500">{selectedLog.id}</span>
              </div>
              <div className="flex-1 overflow-auto p-4 custom-scrollbar">
                <JsonViewer value={selectedLog.payload} path="debug-payload" />
              </div>
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-zinc-600 gap-4">
              <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1" strokeLinecap="round" strokeLinejoin="round" className="opacity-20"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>
              <p className="text-xs italic uppercase tracking-widest opacity-50">Select an entry to view details</p>
            </div>
          )}
        </div>
      </div>
      
      {!prefs.debugMode && (
        <div className="bg-rose-500/10 border-t border-rose-500/20 px-4 py-2 flex items-center gap-3">
          <div className="w-2 h-2 rounded-full bg-rose-500 animate-pulse" />
          <p className="text-[10px] text-rose-400 font-bold uppercase tracking-wider">
            Debug Mode is OFF. To see live logs, enable it in Options.
          </p>
        </div>
      )}
    </div>
  );
}
