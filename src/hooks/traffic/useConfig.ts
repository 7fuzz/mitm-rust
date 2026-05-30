import { useState, useRef, useEffect } from 'react';
import { UILayout } from './types';

export const DEFAULT_SHORTCUTS = {
  goto_history: 'h',
  goto_intercept: 'i',
  goto_repeater: 'r',
  goto_workspace: 'w',
  goto_utilities: 'u',
  goto_options: 'o',
  open_variable_switcher: 'v',
  open_environment_switcher: 'e',
  instant_variable_switcher: 'v',
  instant_environment_switcher: 'e',
  prefix_key: 'g',
  cycle_prev: '[',
  cycle_next: ']'
};

export function useConfig() {
  const [prefs, setPrefs] = useState({ 
    history: true, 
    repeater: true, 
    bindings: true, 
    limits: true, 
    intercept: true, 
    simpleMode: true, 
    autoSave: true,
    replacementsAutoSave: true,
    theme: 'dark' as 'dark' | 'light',
    shortcuts: DEFAULT_SHORTCUTS
  });
  const [isIntercepting, setIsIntercepting] = useState(false);
  const [interceptMode, setInterceptMode] = useState<'both' | 'request' | 'response'>('both');
  const [ignoredMethods, setIgnoredMethods] = useState<string[]>(['OPTIONS']);
  const [urlFilter, setUrlFilter] = useState('');
  const [isLimitEnabled, setIsLimitEnabled] = useState(true);
  const [historyLimit, setHistoryLimit] = useState(100);
  const [uiLayout, setUiLayout] = useState<UILayout>({ isListOpen: true, sidebarWidth: 350, splitMode: 'vertical' });

  const limitRef = useRef({ enabled: isLimitEnabled, value: historyLimit });
  const prefsRef = useRef(prefs);

  const updateConfig = async (enabled: boolean, mode: 'both' | 'request' | 'response', ignored: string[], filter: string) => {
    setIsIntercepting(enabled); setInterceptMode(mode); setIgnoredMethods(ignored); setUrlFilter(filter);
    if (prefsRef.current.intercept) fetch('/api/state', { method: 'POST', body: JSON.stringify({ intercept: { enabled, mode, ignored, url_filter: filter } }) });
  };

  const updatePrefs = async (newPrefs: typeof prefs) => {
    setPrefs(newPrefs);
    await fetch('/api/state', { method: 'POST', body: JSON.stringify({ preferences: newPrefs }) });
  };

  const updateUILayout = (updates: Partial<UILayout>) => {
    const next = { ...uiLayout, ...updates };
    setUiLayout(next);
    fetch('/api/state', { method: 'POST', body: JSON.stringify({ ui_layout: next }) });
  };

  useEffect(() => {
    limitRef.current = { enabled: isLimitEnabled, value: historyLimit };
  }, [isLimitEnabled, historyLimit]);

  useEffect(() => {
    prefsRef.current = prefs;
  }, [prefs]);

  const initConfig = { 
    setPrefs: (loadedPrefs: any) => {
      setPrefs(prev => ({
        ...prev,
        ...loadedPrefs,
        shortcuts: {
          ...DEFAULT_SHORTCUTS,
          ...(loadedPrefs.shortcuts || {})
        }
      }));
    },
    setIsLimitEnabled, 
    setHistoryLimit, 
    setIsIntercepting, 
    setInterceptMode, 
    setIgnoredMethods, 
    setUrlFilter, 
    setUiLayout 
  };

  return {
    prefs, updatePrefs, prefsRef,
    simpleMode: prefs.simpleMode,
    isIntercepting, interceptMode, ignoredMethods, urlFilter, updateConfig,
    isLimitEnabled, setIsLimitEnabled, historyLimit, setHistoryLimit, limitRef,
    uiLayout, updateUILayout, initConfig
  };
}
