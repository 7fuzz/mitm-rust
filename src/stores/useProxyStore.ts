import { create } from 'zustand';
import type { TrafficItem, InterceptConfig, InterceptRule, PendingFlow, ProxyStatus } from '../types';
import {
  setProxyMode,
  updateInterceptConfig,
  updateFilterConfig,
  resumeFlow,
  isTauriAvailable,
} from '../services/tauri/ipc';
import { subscribeTrafficCaptured, getHistoryLogs, clearHistoryLogs } from '../services/tauri/bridge';

const mapHeaders = (headers: any): { key: string; value: string }[] => {
  if (!headers) return [];
  if (Array.isArray(headers)) {
    return headers.map((h: any) => {
      if (Array.isArray(h) && h.length >= 2) {
        return { key: String(h[0]), value: String(h[1]) };
      }
      if (typeof h === 'object' && h !== null && 'key' in h) {
        return { key: String(h.key), value: String(h.value) };
      }
      return { key: String(h), value: '' };
    });
  }
  return [];
};

export const mapHistoryEntryToTrafficItem = (item: any): TrafficItem => {
  const reqHeaders = mapHeaders(item.requestHeaders || item.request_headers);
  const resHeaders = mapHeaders(item.responseHeaders || item.response_headers);

  return {
    id: String(item.id),
    method: item.method || 'GET',
    host: item.host || '',
    path: item.path || item.url || '/',
    url: item.url || '',
    statusCode: item.statusCode ?? item.status_code ?? 200,
    contentType: item.contentType || item.content_type || 'text/plain',
    size: item.responseSize ?? item.size ?? 0,
    durationMs: item.durationMs ?? item.duration_ms ?? 0,
    timestamp: item.createdAt ? Date.parse(item.createdAt) : item.timestamp || Date.now(),
    requestHeaders: reqHeaders,
    responseHeaders: resHeaders,
    requestBody: item.requestBody ?? item.request_body ?? '',
    responseBody: item.responseBody ?? item.response_body ?? '',
    isIntercepted: item.isIntercepted || item.is_intercepted || false,
  };
};

const SAMPLE_TRAFFIC: TrafficItem[] = [
  {
    id: 'req-101',
    method: 'POST',
    host: 'api.github.com',
    path: '/graphql',
    url: 'https://api.github.com/graphql',
    statusCode: 200,
    contentType: 'application/json; charset=utf-8',
    size: 1420,
    durationMs: 84,
    timestamp: Date.now() - 12000,
    requestHeaders: [
      { key: 'Host', value: 'api.github.com' },
      { key: 'User-Agent', value: 'MITM-Developer-Studio/2.0' },
      { key: 'Authorization', value: 'Bearer ghp_v188274918237498127391' },
      { key: 'Content-Type', value: 'application/json' },
    ],
    requestBody: JSON.stringify({ query: 'query { viewer { login email repositories(first: 5) { nodes { name } } } }' }, null, 2),
    responseHeaders: [
      { key: 'HTTP/1.1', value: '200 OK' },
      { key: 'Content-Type', value: 'application/json; charset=utf-8' },
      { key: 'X-RateLimit-Limit', value: '5000' },
      { key: 'X-RateLimit-Remaining', value: '4982' },
      { key: 'Cache-Control', value: 'private, max-age=0, must-revalidate' },
    ],
    responseBody: JSON.stringify({
      data: {
        viewer: {
          login: 'octocat',
          email: 'octocat@github.com',
          repositories: {
            nodes: [
              { name: 'mitm-rust' },
              { name: 'tauri-security-suite' },
              { name: 'api-vault' },
            ],
          },
        },
      },
    }, null, 2),
    ip: '140.82.121.4',
  },
];

interface ProxyState {
  proxyStatus: ProxyStatus;
  traffic: TrafficItem[];
  selectedTrafficId: string | null;
  
  // Filtering
  searchQuery: string;
  selectedMethods: string[];
  methodFilters: Record<string, 'include' | 'exclude' | 'neutral'>;
  statusCodeRange: 'all' | '2xx' | '3xx' | '4xx' | '5xx';
  onlyIntercepted: boolean;

  // Intercept
  interceptConfig: InterceptConfig;
  interceptRules: InterceptRule[];
  pendingQueue: PendingFlow[];

  // Actions
  initStore: () => Promise<void>;
  setProxyMode: (mode: 'normal' | 'intercept' | 'off') => Promise<void>;
  addTrafficItem: (item: TrafficItem) => void;
  selectTrafficItem: (id: string | null) => void;
  clearTraffic: () => Promise<void>;
  deleteTrafficItem: (id: string) => void;
  setSearchQuery: (query: string) => void;
  setSelectedMethods: (methods: string[]) => void;
  setMethodFilter: (method: string, state: 'include' | 'exclude' | 'neutral') => void;
  setStatusCodeRange: (range: 'all' | '2xx' | '3xx' | '4xx' | '5xx') => void;
  setOnlyIntercepted: (val: boolean) => void;

  // Intercept actions
  toggleIntercept: (enabled?: boolean) => Promise<void>;
  setInterceptDirection: (direction: 'request' | 'response' | 'both') => Promise<void>;
  addInterceptRule: (rule: Omit<InterceptRule, 'id'>) => Promise<void>;
  toggleInterceptRule: (id: string) => Promise<void>;
  deleteInterceptRule: (id: string) => Promise<void>;
  forwardPendingFlow: (id: string, modifiedBody?: string, modifiedHeaders?: Record<string, string>) => Promise<void>;
  dropPendingFlow: (id: string) => Promise<void>;
  forwardAllPending: () => Promise<void>;
}

export const useProxyStore = create<ProxyState>((set, get) => ({
  proxyStatus: {
    mode: 'normal',
    bindings: ['127.0.0.1:8080'],
    activeCount: 1,
  },
  traffic: SAMPLE_TRAFFIC,
  selectedTrafficId: 'req-101',

  searchQuery: '',
  selectedMethods: [],
  methodFilters: {},
  statusCodeRange: 'all',
  onlyIntercepted: false,

  interceptConfig: {
    enabled: false,
    mode: 'all',
    direction: 'request',
    autoTimeoutSeconds: 30,
  },
  interceptRules: [
    { id: 'rule-1', target: 'domain', pattern: '*.stripe.internal', action: 'intercept', enabled: true },
    { id: 'rule-2', target: 'path', pattern: '/api/v1/auth/*', action: 'intercept', enabled: true },
  ],
  pendingQueue: [],

  initStore: async () => {
    if (isTauriAvailable()) {
      try {
        const historyLogs = await getHistoryLogs(1, 1000);
        if (historyLogs && historyLogs.length > 0) {
          const mapped = historyLogs.map(mapHistoryEntryToTrafficItem);
          set({ traffic: mapped, selectedTrafficId: mapped[0]?.id || null });
        }
      } catch (e) {
        console.warn('Failed to fetch initial HTTP history via IPC:', e);
      }
    }

    // Subscribe to live captured traffic from Rust backend
    subscribeTrafficCaptured((event: any) => {
      const raw = event?.entry || event;
      if (raw && raw.id) {
        const mappedItem = mapHistoryEntryToTrafficItem(raw);
        get().addTrafficItem(mappedItem);
      }
    });
  },

  setProxyMode: async (mode) => {
    try {
      await setProxyMode(mode);
      set((state) => ({
        proxyStatus: { ...state.proxyStatus, mode },
      }));
    } catch (err) {
      console.error('Failed to set proxy mode:', err);
    }
  },

  addTrafficItem: (item) => {
    set((state) => {
      const filtered = state.traffic.filter((t) => t.id !== item.id);
      return {
        traffic: [item, ...filtered].slice(0, 10000),
      };
    });
  },

  selectTrafficItem: (id) => set({ selectedTrafficId: id }),

  clearTraffic: async () => {
    try {
      await clearHistoryLogs();
    } catch (e) {
      console.warn('Failed to clear backend history via IPC');
    }
    set({ traffic: [], selectedTrafficId: null });
  },

  deleteTrafficItem: (id) => {
    set((state) => ({
      traffic: state.traffic.filter((t) => t.id !== id),
      selectedTrafficId: state.selectedTrafficId === id ? null : state.selectedTrafficId,
    }));
  },

  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setSelectedMethods: (selectedMethods) => set({ selectedMethods }),
  setMethodFilter: (method, filterState) => {
    set((state) => {
      const nextFilters = { ...state.methodFilters };
      if (filterState === 'neutral') {
        delete nextFilters[method];
      } else {
        nextFilters[method] = filterState;
      }
      return { methodFilters: nextFilters };
    });
  },
  setStatusCodeRange: (statusCodeRange) => set({ statusCodeRange }),
  setOnlyIntercepted: (onlyIntercepted) => set({ onlyIntercepted }),

  toggleIntercept: async (enabled) => {
    const nextEnabled = enabled !== undefined ? enabled : !get().interceptConfig.enabled;
    const nextConfig: InterceptConfig = { ...get().interceptConfig, enabled: nextEnabled };
    try {
      await updateInterceptConfig(nextConfig);
      set({ interceptConfig: nextConfig });
    } catch (err) {
      console.error('Failed to toggle intercept config:', err);
    }
  },

  setInterceptDirection: async (direction) => {
    const nextConfig: InterceptConfig = { ...get().interceptConfig, direction };
    try {
      await updateInterceptConfig(nextConfig);
      set({ interceptConfig: nextConfig });
    } catch (err) {
      console.error('Failed to update intercept direction:', err);
    }
  },

  addInterceptRule: async (rule) => {
    const newRule: InterceptRule = { ...rule, id: 'rule-' + Date.now() };
    const nextRules = [...get().interceptRules, newRule];
    set({ interceptRules: nextRules });
    await updateFilterConfig({ rules: nextRules });
  },

  toggleInterceptRule: async (id) => {
    const nextRules = get().interceptRules.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r));
    set({ interceptRules: nextRules });
    await updateFilterConfig({ rules: nextRules });
  },

  deleteInterceptRule: async (id) => {
    const nextRules = get().interceptRules.filter((r) => r.id !== id);
    set({ interceptRules: nextRules });
    await updateFilterConfig({ rules: nextRules });
  },

  forwardPendingFlow: async (id, modifiedBody, modifiedHeaders) => {
    try {
      await resumeFlow(id, { action: 'forward', modifiedBody, modifiedHeaders });
      set((state) => ({
        pendingQueue: state.pendingQueue.filter((p) => p.id !== id),
      }));
    } catch (err) {
      console.error('Failed to forward pending flow:', err);
    }
  },

  dropPendingFlow: async (id) => {
    try {
      await resumeFlow(id, { action: 'drop' });
      set((state) => ({
        pendingQueue: state.pendingQueue.filter((p) => p.id !== id),
      }));
    } catch (err) {
      console.error('Failed to drop pending flow:', err);
    }
  },

  forwardAllPending: async () => {
    const pending = get().pendingQueue;
    for (const flow of pending) {
      try {
        await resumeFlow(flow.id, { action: 'forward' });
      } catch (err) {
        console.error('Failed to forward pending flow:', flow.id, err);
      }
    }
    set({ pendingQueue: [] });
  },
}));
