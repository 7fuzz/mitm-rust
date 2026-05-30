import { useState } from 'react';

export function useSelection() {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [repeaterSelectedId, setRepeaterSelectedId] = useState<string | null>(null);
  const [interceptSelectedId, setInterceptSelectedId] = useState<string | null>(null);

  return {
    selectedId, setSelectedId,
    repeaterSelectedId, setRepeaterSelectedId,
    interceptSelectedId, setInterceptSelectedId
  };
}
