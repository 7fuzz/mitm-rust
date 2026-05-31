import { useState, useEffect, createContext, useContext, ReactNode, useRef } from 'react';
import { listen, invoke } from '@/lib/utils/tauri';
import { Traffic } from '@/types/traffic';
import { SyncData, SyncStatus } from './types';
import { useNotification } from '@/components/ui/NotificationProvider';

// Import our segmented hooks
import { useSelection } from './useSelection';
import { useVariables } from './useVariables';
import { useConfig } from './useConfig';
import { useRepeater } from './useRepeater';
import { useTrafficLog } from './useTrafficLog';
import { useJsonToolkit } from './useJsonToolkit';
import { useReplacements } from './useReplacements';
import { useDebugLog } from './useDebugLog';
import { setDebugLogger } from '@/lib/utils/tauri';

// Re-export types so other components can still import them from '@/hooks/traffic'
export * from './types';
export * from './useDebugLog';

// ============================================================================
// MAIN ROOT HOOK
// ============================================================================
function useTrafficState() {
  const selections = useSelection();
  const config = useConfig();
  const variables = useVariables(config.prefs);
  const repeater = useRepeater(variables.activeEnvId);
  const trafficData = useTrafficLog();
  const replacements = useReplacements();
  const debugLog = useDebugLog();
  const { notify } = useNotification();

  // Sync debug logger with current state
  useEffect(() => {
    setDebugLogger(debugLog.addLog, config.prefs.debugMode);
  }, [debugLog.addLog, config.prefs.debugMode]);
  
  const isFirstSync = useRef(true);
  const jsonToolkit = useJsonToolkit();

  const [isStateLoaded, setIsStateLoaded] = useState(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({ is_syncing: false, last_sync: null, error: null });

  const syncAll = async (silent = false) => {
    setSyncStatus(prev => ({ ...prev, is_syncing: true, error: null }));
    if (!silent) notify.info('Synchronizing data with backend...');
    
    try {
      const data = await invoke<SyncData>('sync_data');
      
      // Update each hook with fresh data
      trafficData.setTraffic(data.history || []);
      repeater._setRawGroups(data.repeaterGroups || []);
      repeater._setRawRepeater(data.repeaterRequests || []);
      variables.loadVariables(data.variables || [], data.environments || [], data.environments?.find(e => e.is_active)?.id || data.environments?.[0]?.id || 'default-env-id');
      
      if (data.prefs) config.initConfig.setPrefs(data.prefs);
      if (data.uiLayout) config.initConfig.setUiLayout(data.uiLayout as any);
      if (data.toolkitJson) jsonToolkit._initToolkitJson(data.toolkitJson);
      if (data.historyLimits) {
        const hl = data.historyLimits as any;
        if (typeof hl.enabled === 'boolean') config.initConfig.setIsLimitEnabled(hl.enabled);
        if (typeof hl.value === 'number') config.initConfig.setHistoryLimit(hl.value);
      }

      // Group replacements for the segmented hook
      const groupedReplacements = {
        URL_REPLACEMENTS: {} as Record<string, string>,
        HEADER_REPLACEMENTS: {} as Record<string, string>,
        BODY_KEY_REPLACEMENTS: {} as Record<string, string>,
        URL_PARAM_REPLACEMENTS: {} as Record<string, string>,
        TEXT_REPLACEMENTS: {} as Record<string, string>
      };
      (data.replacements || []).forEach(r => {
        if (r.is_active && groupedReplacements[r.type as keyof typeof groupedReplacements]) {
          groupedReplacements[r.type as keyof typeof groupedReplacements][r.pattern] = r.replacement;
        }
      });
      replacements._setRawReplacements(groupedReplacements, data.replacements || []);

      setSyncStatus({ is_syncing: false, last_sync: Date.now(), error: null });
      if (!silent) notify.success('Data synchronized successfully');
      setIsStateLoaded(true);
    } catch (e) {
      const errorMsg = String(e);
      setSyncStatus({ is_syncing: false, last_sync: null, error: errorMsg });
      notify.error(`Sync failed: ${errorMsg}`);
    }
  };

  useEffect(() => {
    // Initial sync
    syncAll(true);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!isStateLoaded) return;

    // === NEW: Skip the POST request on the very first render after loading ===
    if (isFirstSync.current) {
      isFirstSync.current = false;
      return;
    }

    if (config.prefs.limits) {
      invoke('save_state', { 
        key: 'history_limits', 
        value: JSON.stringify({ enabled: config.isLimitEnabled, value: config.historyLimit }) 
      }).catch(() => { });
    }

    if (config.isLimitEnabled) {
      trafficData.setTraffic(prev => {
        if (prev.length > config.historyLimit) {
          return prev.slice(0, config.historyLimit);
        }
        return prev;
      });
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config.isLimitEnabled, config.historyLimit, isStateLoaded, config.prefs.limits]);

  useEffect(() => {
    // === 1. TAURI EVENT LISTENER ===
    const unlisten = listen<Traffic>('traffic_captured', (event) => {
      const data = event.payload;
      trafficData.setTraffic((prev) => {
        const filtered = prev.filter(t => t.id !== data.id);
        const next = [data, ...filtered];
        if (config.limitRef.current.enabled) return next.slice(0, config.limitRef.current.value);
        return next;
      });
    });

    return () => {
      unlisten.then(f => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const { initConfig: _initConfig, prefsRef: _prefsRef, limitRef: _limitRef, ...configRest } = config; // eslint-disable-line @typescript-eslint/no-unused-vars
  const { _initToolkitJson: _itj, ...jsonToolkitRest } = jsonToolkit; // eslint-disable-line @typescript-eslint/no-unused-vars

  return {
    ...selections,
    ...variables,
    setActiveEnvironment: (id: string) => variables.switchWorkspace(id, repeater.activeGroupId, (data) => {
      repeater.bulkSync(data.groups, data.requests);
    }),
    // Strip out internal config tools
    ...configRest,
    // Export internal repeater boot tools for optimistic updates
    ...repeater,
    ...jsonToolkitRest,
    ...trafficData,
    ...replacements,
    ...debugLog,
    syncAll,
    syncStatus,
    selectedReq: trafficData.traffic.find((r) => r.id === selections.selectedId) || null,
  };
}

// ============================================================================
// CONTEXT & PROVIDERS
// ============================================================================
type TrafficContextType = ReturnType<typeof useTrafficState>;
const TrafficContext = createContext<TrafficContextType | null>(null);

export function TrafficProvider({ children }: { children: ReactNode }) {
  const state = useTrafficState();
  return (
    <TrafficContext.Provider value={state} >
      {children}
    </TrafficContext.Provider>
  );
}

export function useTraffic() {
  const context = useContext(TrafficContext);
  if (!context) {
    throw new Error("useTraffic must be used within a TrafficProvider");
  }
  return context;
}
