import { useState, useCallback } from 'react';

export function useSelection() {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [repeaterSelectedId, setRepeaterSelectedId] = useState<string | null>(null);
  const [interceptSelectedId, setInterceptSelectedId] = useState<string | null>(null);

  const _setSelectedId = useCallback((id: string | null) => setSelectedId(id), []);
  const _setRepeaterSelectedId = useCallback((id: string | null) => setRepeaterSelectedId(id), []);
  const _setInterceptSelectedId = useCallback((id: string | null) => setInterceptSelectedId(id), []);

  return {
    selectedId, setSelectedId: _setSelectedId,
    repeaterSelectedId, setRepeaterSelectedId: _setRepeaterSelectedId,
    interceptSelectedId, setInterceptSelectedId: _setInterceptSelectedId
  };
}
