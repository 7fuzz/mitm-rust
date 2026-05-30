import { useState, useRef, useEffect } from 'react';
import { UILayout } from './types';
import { invoke } from '@tauri-apps/api/core';

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
  const [filterConfig, setFilterConfig] = useState<{ rules: Array<{ id: string, is_active: boolean, rule_type: string, mode: 'whitelist' | 'blacklist', pattern: string }> }>({ 
    rules: [] 
  });

  const limitRef = useRef({ enabled: isLimitEnabled, value: historyLimit });
  const prefsRef = useRef(prefs);

  const updateConfig = async (enabled: boolean, mode: 'both' | 'request' | 'response', ignored: string[], filter: string) => {
    setIsIntercepting(enabled); setInterceptMode(mode); setIgnoredMethods(ignored); setUrlFilter(filter);
    
    try {
      await invoke('update_state', { config: {
        enabled,
        mode,
        ignored_methods: ignored,
        url_filter: filter
      }});
    } catch (e) {
      console.error('Failed to update intercept config:', e);
    }
  };

  const updatePrefs = async (newPrefs: typeof prefs) => {
    setPrefs(newPrefs);
    // TODO: implement update_prefs in rust if needed
  };

  const updateUILayout = (updates: Partial<UILayout>) => {
    const next = { ...uiLayout, ...updates };
    setUiLayout(next);
    // TODO: implement update_ui_layout in rust if needed
  };

  const updateFilterConfig = async (config: typeof filterConfig) => {
    setFilterConfig(config);
    try {
      await invoke('update_filter_config', { config });
    } catch (e) {
      console.error('Failed to update filter config:', e);
    }
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
    setUiLayout,
    setFilterConfig
  };

  return {
    prefs, updatePrefs, prefsRef,
    simpleMode: prefs.simpleMode,
    isIntercepting, interceptMode, ignoredMethods, urlFilter, updateConfig,
    isLimitEnabled, setIsLimitEnabled, historyLimit, setHistoryLimit, limitRef,
    uiLayout, updateUILayout, 
    filterConfig, updateFilterConfig,
    initConfig
  };
}
