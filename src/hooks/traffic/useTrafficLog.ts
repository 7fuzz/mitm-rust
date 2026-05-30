import { useState } from 'react';
import { Traffic } from '@/types/traffic';

export interface ResumeData {
  method?: string;
  url?: string;
  headers?: Record<string, string>;
  body?: string;
  status_code?: number;
  variables?: Record<string, string>;
  drop?: boolean;
}

export function useTrafficLog() {
  const [traffic, setTraffic] = useState<Traffic[]>([]);

  const resumeRequest = async (id: string, modifiedData: ResumeData) => {
    await fetch(`http://127.0.0.1:3001/resume/${id}`, { method: 'POST', body: JSON.stringify(modifiedData) });
    setTraffic(prev => prev.map((t) => (t.id === id ? { ...t, is_intercepted: false } : t)));
  };

  return { traffic, setTraffic, resumeRequest };
}
