import { useEffect } from 'react';

interface HotkeyConfig {
  key: string | string[];
  handler: (e: KeyboardEvent) => void;
  meta?: boolean;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  ignoreInputs?: boolean;
  preventDefault?: boolean;
  stopPropagation?: boolean;
  enabled?: boolean;
}

/**
 * Utility to check if an element is an input/editable
 */
export const isInputTarget = (target: HTMLElement) => {
  return (
    target.tagName === 'INPUT' || 
    target.tagName === 'TEXTAREA' || 
    target.isContentEditable ||
    target.closest('.monaco-editor')
  );
};

/**
 * A reusable hook for component-level keyboard shortcuts.
 */
export function useHotkeys(hotkeys: HotkeyConfig[], deps: any[] = [], capture = true) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      
      for (const hotkey of hotkeys) {
        if (hotkey.enabled === false) continue;
        
        // By default, ignore inputs unless explicitly set to false
        if (hotkey.ignoreInputs !== false && isInputTarget(target)) continue;

        const keys = Array.isArray(hotkey.key) ? hotkey.key : [hotkey.key];
        const matchesKey = keys.some(k => k.toLowerCase() === e.key.toLowerCase());

        if (matchesKey) {
          const matchesMeta = hotkey.meta === undefined || hotkey.meta === e.metaKey;
          const matchesCtrl = hotkey.ctrl === undefined || hotkey.ctrl === e.ctrlKey;
          const matchesShift = hotkey.shift === undefined || hotkey.shift === e.shiftKey;
          const matchesAlt = hotkey.alt === undefined || hotkey.alt === e.altKey;

          if (matchesMeta && matchesCtrl && matchesShift && matchesAlt) {
            if (hotkey.preventDefault !== false) e.preventDefault();
            if (hotkey.stopPropagation) e.stopPropagation();
            hotkey.handler(e);
            return; // Only handle the first matching hotkey
          }
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown, capture);
    return () => window.removeEventListener('keydown', handleKeyDown, capture);
  }, [deps]);
}
