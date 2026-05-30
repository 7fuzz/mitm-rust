import { useState, useEffect, createContext, useContext, ReactNode, useRef } from 'react';
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
    // === 1. CORE STATE LOAD ===
    fetch('/api/state').then(r => r.json()).then(state => {
      if (state.preferences) config.initConfig.setPrefs(state.preferences);
      if (state.limits && state.preferences?.limits !== false) {
        config.initConfig.setIsLimitEnabled(state.limits.enabled);
        config.initConfig.setHistoryLimit(state.limits.value);
      }
      if (state.intercept && state.preferences?.intercept !== false) {
        config.initConfig.setIsIntercepting(state.intercept.enabled);
        config.initConfig.setInterceptMode(state.intercept.mode);
        config.initConfig.setIgnoredMethods(state.intercept.ignored);
        config.initConfig.setUrlFilter(state.intercept.url_filter || '');
      }
      if (state.ui_layout) config.initConfig.setUiLayout(state.ui_layout);
      if (state.toolkit_json) jsonToolkit._initToolkitJson(state.toolkit_json);
      if (state.queue && state.queue.length > 0) trafficData.setTraffic(prev => [...state.queue, ...prev]);

      // === FIXED: Consolidated Initial Load based on Saved State! ===
      const savedGroupId = state.active_repeater_group || 'All';
      repeater.initActiveGroup(savedGroupId);

      // We need a fallback envId if one isn't active, but switchWorkspace handles activation
      // If we don't have an active env yet, we'll fetch from /api/variables first to get the activeId
      fetch('/api/variables').then(r => r.json()).then(data => {
        if (data.variables) {
          variables.switchWorkspace(data.activeEnvironmentId, savedGroupId, (wsData) => {
            repeater.bulkSync(wsData.groups, wsData.requests);
          });
        }
      });

      // === 2. PARALLEL BACKGROUND LOADS ===
      fetch('/api/history').then(r => r.json()).then(hist => {
        if (hist && hist.length > 0) {
          trafficData.setTraffic(prev => {
            const next = [...prev, ...hist.reverse()];
            if (config.limitRef.current.enabled) return next.slice(0, config.limitRef.current.value);
            return next;
          });
        }
      });

      setIsStateLoaded(true);
    });

    // === 3. SSE CONNECTION ===
    const connectSSE = () => {
      const eventSource = new EventSource('/api/traffic');
      
      eventSource.onmessage = (e) => {
        const data: Traffic = JSON.parse(e.data);
        trafficData.setTraffic((prev) => {
          const filtered = prev.filter(t => t.id !== data.id);
          const next = [data, ...filtered];
          if (config.limitRef.current.enabled) return next.slice(0, config.limitRef.current.value);
          return next;
        });
      };

      eventSource.onerror = () => {
        console.error("SSE Connection interrupted. Browser will attempt to reconnect automatically.");
        eventSource.close();
        // Manually trigger a reconnect after a short delay if the browser's native retry isn't enough
        setTimeout(connectSSE, 3000);
      };

      return eventSource;
    };

    const es = connectSSE();

    return () => es.close();
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
