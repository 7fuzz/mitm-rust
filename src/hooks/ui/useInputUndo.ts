import { useCallback, useRef, useEffect, KeyboardEvent } from 'react';

interface UndoOptions {
  maxHistory?: number;
  debounceMs?: number;
}

export function useInputUndo<T extends HTMLInputElement | HTMLTextAreaElement>(
  value: string,
  onChange: (val: string) => void,
  options: UndoOptions = {}
) {
  const { maxHistory = 100, debounceMs = 250 } = options;

  const historyRef = useRef<string[]>([value || '']);
  const pointerRef = useRef<number>(0);
  const isInternalUpdate = useRef<boolean>(false);
  const debounceTimerRef = useRef<NodeJS.Timeout | null>(null);

  // Keep history synced if value is changed externally (e.g. selecting a different request)
  useEffect(() => {
    if (isInternalUpdate.current) {
      isInternalUpdate.current = false;
      return;
    }

    const currentHistory = historyRef.current;
    const currentPointer = pointerRef.current;
    const currentVal = currentHistory[currentPointer] || '';

    if (value !== currentVal) {
      // If significant difference or completely new value, re-initialize history stack
      if (Math.abs(value.length - currentVal.length) > 30 || !currentHistory.includes(value)) {
        historyRef.current = [value || ''];
        pointerRef.current = 0;
      }
    }
  }, [value]);

  const pushState = useCallback((newValue: string) => {
    const history = historyRef.current;
    const pointer = pointerRef.current;

    if (history[pointer] === newValue) return;

    // Truncate any redo stack
    const newHistory = history.slice(0, pointer + 1);
    newHistory.push(newValue);

    if (newHistory.length > maxHistory) {
      newHistory.shift();
    }

    historyRef.current = newHistory;
    pointerRef.current = newHistory.length - 1;
  }, [maxHistory]);

  const handleKeyDown = useCallback((e: KeyboardEvent<T>) => {
    const isMac = typeof navigator !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const isCmdOrCtrl = isMac ? e.metaKey : e.ctrlKey;

    if (!isCmdOrCtrl) return;

    const key = e.key.toLowerCase();

    if (key === 'z') {
      e.preventDefault();
      e.stopPropagation();

      const history = historyRef.current;
      if (e.shiftKey) {
        // Redo (Ctrl + Shift + Z / Cmd + Shift + Z)
        if (pointerRef.current < history.length - 1) {
          pointerRef.current += 1;
          const nextVal = history[pointerRef.current];
          isInternalUpdate.current = true;
          onChange(nextVal);
        }
      } else {
        // Undo (Ctrl + Z / Cmd + Z)
        if (pointerRef.current > 0) {
          pointerRef.current -= 1;
          const prevVal = history[pointerRef.current];
          isInternalUpdate.current = true;
          onChange(prevVal);
        }
      }
    } else if (key === 'y' && !isMac) {
      // Redo (Ctrl + Y)
      e.preventDefault();
      e.stopPropagation();
      const history = historyRef.current;
      if (pointerRef.current < history.length - 1) {
        pointerRef.current += 1;
        const nextVal = history[pointerRef.current];
        isInternalUpdate.current = true;
        onChange(nextVal);
      }
    }
  }, [onChange]);

  const handleChange = useCallback((newVal: string) => {
    onChange(newVal);

    if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    debounceTimerRef.current = setTimeout(() => {
      pushState(newVal);
    }, debounceMs);
  }, [onChange, pushState, debounceMs]);

  return {
    handleKeyDown,
    handleChange,
    pushState
  };
}
