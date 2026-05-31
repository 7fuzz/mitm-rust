import { useState, useCallback } from 'react';

export interface DebugLogEntry {
  id: string;
  timestamp: number;
  type: 'invoke' | 'event' | 'response' | 'error';
  target: string; // command name or event name
  payload: any;
  direction: 'to-backend' | 'from-backend';
}

export function useDebugLog() {
  const [logs, setLogs] = useState<DebugLogEntry[]>([]);
  const [maxLogs] = useState(1000);

  const addLog = useCallback((entry: Omit<DebugLogEntry, 'id' | 'timestamp'>) => {
    const newEntry: DebugLogEntry = {
      ...entry,
      id: Math.random().toString(36).substring(2, 9),
      timestamp: Date.now(),
    };

    setLogs(prev => {
      const next = [newEntry, ...prev];
      if (next.length > maxLogs) {
        return next.slice(0, maxLogs);
      }
      return next;
    });
  }, [maxLogs]);

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  return {
    logs,
    addLog,
    clearLogs
  };
}
