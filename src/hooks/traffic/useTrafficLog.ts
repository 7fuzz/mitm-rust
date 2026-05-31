import { useState } from 'react';
import { Traffic } from '@/types/traffic';
import { invoke } from '@/lib/utils/tauri';

export interface ResumeData {
  method?: string;
  url?: string;
  headers?: [string, string][];
  body?: string;
  status_code?: number;
  variables?: Record<string, string>;
  drop?: boolean;
}

export function useTrafficLog() {
  const [traffic, setTraffic] = useState<Traffic[]>([]);

  const resumeRequest = async (id: string, modifiedData: ResumeData) => {
    try {
      await invoke('resume_flow', { id, action: modifiedData });
    } catch (e) {
      console.error('Failed to resume flow:', e);
    }
    setTraffic(prev => prev.map((t) => (t.id === id ? { ...t, is_intercepted: false } : t)));
  };

  return { traffic, setTraffic, resumeRequest };
}
