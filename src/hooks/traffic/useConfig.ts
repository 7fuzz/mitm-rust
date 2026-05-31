import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { UILayout } from './types';
import { invoke } from '@/lib/utils/tauri';

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
    debugMode: false,
    theme: 'dark' as 'dark' | 'light',
    shortcuts: DEFAULT_SHORTCUTS
  });
  const [isProxyActive, setIsProxyActive] = useState(true);
  const [isIntercepting, setIsIntercepting] = useState(false);
  const [interceptMode, setInterceptMode] = useState<'both' | 'request' | 'response'>('both');
  const [ignoredMethods, setIgnoredMethods] = useState<string[]>(['OPTIONS']);
  const [urlFilter, setUrlFilter] = useState('');
  const [autoFocus, setAutoFocus] = useState(true);
  const [isLimitEnabled, setIsLimitEnabled] = useState(true);
  const [historyLimit, setHistoryLimit] = useState(100);
  const [uiLayout, setUiLayout] = useState<UILayout>({ isListOpen: true, sidebarWidth: 350, splitMode: 'vertical' });
  const [filterConfig, setFilterConfig] = useState<{ rules: Array<{ id: string, is_active: boolean, rule_type: string, mode: 'whitelist' | 'blacklist', pattern: string }> }>({ 
    rules: [] 
  });

  const limitRef = useRef({ enabled: isLimitEnabled, value: historyLimit });
  const prefsRef = useRef(prefs);

  const updateConfig = useCallback(async (enabled: boolean, mode: 'both' | 'request' | 'response', ignored: string[], filter: string, autoFocusVal: boolean) => {
    setIsIntercepting(enabled); setInterceptMode(mode); setIgnoredMethods(ignored); setUrlFilter(filter); setAutoFocus(autoFocusVal);
    
    try {
      await invoke('update_state', { config: {
        enabled,
        mode,
        ignored_methods: ignored,
        url_filter: filter,
        auto_focus: autoFocusVal
      }});
    } catch (e) {
      console.error('Failed to update intercept config:', e);
    }
  }, []);

  const updatePrefs = useCallback(async (newPrefs: typeof prefs) => {
    setPrefs(newPrefs);
    try {
      await invoke('update_prefs', { prefs: newPrefs });
    } catch (e) {
      console.error('Failed to update prefs:', e);
    }
  }, [prefs]);

  const updateUILayout = useCallback(async (updates: Partial<UILayout>) => {
    setUiLayout(prev => {
      const next = { ...prev, ...updates };
      invoke('update_ui_layout', { layout: next }).catch(e => console.error(e));
      return next;
    });
  }, []);

  const updateFilterConfig = useCallback(async (config: typeof filterConfig) => {
    setFilterConfig(config);
    try {
      await invoke('update_filter_config', { config });
    } catch (e) {
      console.error('Failed to update filter config:', e);
    }
  }, []);

  useEffect(() => {
    limitRef.current = { enabled: isLimitEnabled, value: historyLimit };
  }, [isLimitEnabled, historyLimit]);

  useEffect(() => {
    prefsRef.current = prefs;
  }, [prefs]);

  const initConfig = useMemo(() => ({ 
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
    setIsProxyActive,
    setIsIntercepting, 
    setInterceptMode, 
    setIgnoredMethods, 
    setUrlFilter, 
    setAutoFocus,
    setUiLayout,
    setFilterConfig
  }), []);

  return useMemo(() => ({
    prefs, updatePrefs, prefsRef,
    simpleMode: prefs.simpleMode,
    isProxyActive, setIsProxyActive,
    isIntercepting, interceptMode, ignoredMethods, urlFilter, autoFocus, updateConfig,
    isLimitEnabled, setIsLimitEnabled, historyLimit, setHistoryLimit, limitRef,
    uiLayout, updateUILayout, 
    filterConfig, updateFilterConfig,
    initConfig
  }), [
    prefs, updatePrefs, isProxyActive, isIntercepting, interceptMode, ignoredMethods,
    urlFilter, autoFocus, updateConfig, isLimitEnabled, historyLimit, uiLayout,
    updateUILayout, filterConfig, updateFilterConfig, initConfig
  ]);
}
