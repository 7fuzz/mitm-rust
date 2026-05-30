import { useState, useEffect, createContext, useContext, ReactNode, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Traffic } from '@/types/traffic';

// Import our segmented hooks
import { useSelection } from './useSelection';
import { useVariables } from './useVariables';
import { useConfig } from './useConfig';
import { useRepeater } from './useRepeater';
import { useTrafficLog } from './useTrafficLog';
import { useJsonToolkit } from './useJsonToolkit';
import { useReplacements } from './useReplacements';

// Re-export types so other components can still import them from '@/hooks/traffic'
export * from './types';

// ============================================================================
// MAIN ROOT HOOK
// ============================================================================
function useTrafficState() {
  const selections = useSelection();
  const config = useConfig();
  const variables = useVariables(config.prefs);
  const repeater = useRepeater(variables.activeEnvId);
  const trafficData = useTrafficLog();
  const isFirstSync = useRef(true);
  const jsonToolkit = useJsonToolkit();

  const [isStateLoaded, setIsStateLoaded] = useState(false);

  useEffect(() => {
    if (!isStateLoaded) return;

    // === NEW: Skip the POST request on the very first render after loading ===
    if (isFirstSync.current) {
      isFirstSync.current = false;
      return;
    }

    if (config.prefs.limits) {
      fetch('/api/state', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ limits: { enabled: config.isLimitEnabled, value: config.historyLimit } })
      }).catch(() => { }); // Fail silently if network drops
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

    // TODO: Implement get_initial_state and get_history tauri commands
    // For now, we set loaded to true so the UI doesn't hang
    setIsStateLoaded(true);

    return () => {
      unlisten.then(f => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const replacements = useReplacements();
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
