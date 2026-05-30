import { useState, useRef } from 'react';

export function useJsonToolkit() {
  const [toolkitJson, setToolkitJson] = useState('{\n  "status": "waiting",\n  "message": "Send a JSON payload here to begin"\n}');

  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const updateToolkitJson = (newVal: string) => {
    setToolkitJson(newVal);

    if (debounceRef.current) clearTimeout(debounceRef.current);

    debounceRef.current = setTimeout(() => {
      fetch('/api/state', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ toolkit_json: newVal })
      }).catch(() => { }); // Fail silently if network drops
    }, 500);
  };

  return {
    toolkitJson,
    setToolkitJson: updateToolkitJson,
    _initToolkitJson: setToolkitJson // Internal use only for initial boot
  };
}
