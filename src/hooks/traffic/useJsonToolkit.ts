import { useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function useJsonToolkit() {
  const [toolkitJson, setToolkitJson] = useState('{\n  "status": "waiting",\n  "message": "Send a JSON payload here to begin"\n}');

  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const updateToolkitJson = (newVal: string) => {
    setToolkitJson(newVal);

    if (debounceRef.current) clearTimeout(debounceRef.current);

    debounceRef.current = setTimeout(async () => {
      try {
        await invoke('save_state', { key: 'toolkit_json', value: newVal });
      } catch (e) {
        console.error('Failed to save toolkit JSON:', e);
      }
    }, 500);
  };

  return {
    toolkitJson,
    setToolkitJson: updateToolkitJson,
    _initToolkitJson: setToolkitJson // Internal use only for initial boot
  };
}
