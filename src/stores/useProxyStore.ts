import { create } from 'zustand';
import type { TrafficItem, InterceptConfig, InterceptRule, PendingFlow, ProxyStatus } from '../types';
import {
  setProxyMode,
  updateInterceptConfig,
  updateFilterConfig,
  resumeFlow,
  getHttpHistory,
  clearHttpHistory,
  isTauriAvailable,
} from '../services/tauri/ipc';
import { tauriBridge } from '../services/tauri/bridge';

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
  {
    id: 'req-102',
    method: 'GET',
    host: 'auth.internal.dev',
    path: '/api/v1/user/session',
    url: 'https://auth.internal.dev/api/v1/user/session',
    statusCode: 401,
    contentType: 'application/json',
    size: 320,
    durationMs: 32,
    timestamp: Date.now() - 18000,
    requestHeaders: [
      { key: 'Host', value: 'auth.internal.dev' },
      { key: 'Accept', value: 'application/json' },
      { key: 'Cookie', value: 'session_id=expired_token_993' },
    ],
    requestBody: '',
    responseHeaders: [
      { key: 'HTTP/1.1', value: '401 Unauthorized' },
      { key: 'Content-Type', value: 'application/json' },
      { key: 'WWW-Authenticate', value: 'Bearer error="invalid_token"' },
    ],
    responseBody: JSON.stringify({ error: 'unauthorized', message: 'JWT signature expired at 2026-08-25T08:00:00Z' }, null, 2),
    ip: '10.0.1.42',
  },
  {
    id: 'req-103',
    method: 'PUT',
    host: 'payment.stripe.internal',
    path: '/v1/charges/ch_3M492049281',
    url: 'https://payment.stripe.internal/v1/charges/ch_3M492049281',
    statusCode: 200,
    contentType: 'application/json',
    size: 2048,
    durationMs: 142,
    timestamp: Date.now() - 25000,
    requestHeaders: [
      { key: 'Host', value: 'payment.stripe.internal' },
      { key: 'Authorization', value: 'Bearer sk_test_51Mz029481029' },
      { key: 'Content-Type', value: 'application/x-www-form-urlencoded' },
    ],
    requestBody: 'amount=25000&currency=usd&metadata%5Border_id%5D=9482',
    responseHeaders: [
      { key: 'HTTP/1.1', value: '200 OK' },
      { key: 'Content-Type', value: 'application/json' },
      { key: 'Stripe-Version', value: '2024-06-20' },
    ],
    responseBody: JSON.stringify({
      id: 'ch_3M492049281',
      object: 'charge',
      amount: 25000,
      currency: 'usd',
      paid: true,
      status: 'succeeded',
    }, null, 2),
    ip: '34.210.12.9',
  },
  {
    id: 'req-104',
    method: 'DELETE',
    host: 'api.kubernetes.local',
    path: '/api/v1/namespaces/staging/pods/web-frontend-847291',
    url: 'https://api.kubernetes.local/api/v1/namespaces/staging/pods/web-frontend-847291',
    statusCode: 500,
    contentType: 'application/json',
    size: 412,
    durationMs: 290,
    timestamp: Date.now() - 40000,
    requestHeaders: [
      { key: 'Host', value: 'api.kubernetes.local' },
      { key: 'Authorization', value: 'Bearer kube_token_sec_49182' },
    ],
    requestBody: '',
    responseHeaders: [
      { key: 'HTTP/1.1', value: '500 Internal Server Error' },
      { key: 'Content-Type', value: 'application/json' },
    ],
    responseBody: JSON.stringify({
      kind: 'Status',
      apiVersion: 'v1',
      metadata: {},
      status: 'Failure',
      message: 'etcd server leader changed during deletion transaction',
      reason: 'InternalError',
      code: 500,
    }, null, 2),
    ip: '172.16.0.1',
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
  pendingQueue: [
    {
      id: 'flow-pending-1',
      timestamp: Date.now(),
      method: 'POST',
      url: 'https://payment.stripe.internal/v1/refunds',
      direction: 'request',
      headers: [
        { key: 'Host', value: 'payment.stripe.internal' },
        { key: 'Authorization', value: 'Bearer sk_test_99218274198' },
        { key: 'Content-Type', value: 'application/json' },
      ],
      body: JSON.stringify({ charge_id: 'ch_3M492049281', amount: 5000, reason: 'requested_by_customer' }, null, 2),
      originalItem: {
        host: 'payment.stripe.internal',
        path: '/v1/refunds',
      },
    },
  ],

  initStore: async () => {
    if (isTauriAvailable()) {
      try {
        const history = await getHttpHistory();
        set({ traffic: history, selectedTrafficId: history[0]?.id || null });
      } catch (e) {
        console.warn('Failed to fetch initial HTTP history via IPC:', e);
        set({ traffic: [], selectedTrafficId: null });
      }
    }

    // Subscribe to live captured traffic from Rust backend
    listenTrafficCaptured((item) => {
      get().addTrafficItem(item);
    });

    // Subscribe to live intercepted pending flows from Rust backend
    listenInterceptPending((flow) => {
      set((state) => ({
        pendingQueue: [flow, ...state.pendingQueue.filter((f) => f.id !== flow.id)],
      }));
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
    set((state) => ({
      traffic: [item, ...state.traffic].slice(0, 10000), // Virtualized capacity limit
    }));
  },

  selectTrafficItem: (id) => set({ selectedTrafficId: id }),

  clearTraffic: async () => {
    try {
      await clearHttpHistory();
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
    } catch (e) {
      console.warn('Flow resumed via mock/local action');
    }
    set((state) => ({
      pendingQueue: state.pendingQueue.filter((f) => f.id !== id),
    }));
  },

  dropPendingFlow: async (id) => {
    try {
      await resumeFlow(id, { action: 'drop' });
    } catch (e) {
      console.warn('Flow dropped via mock/local action');
    }
    set((state) => ({
      pendingQueue: state.pendingQueue.filter((f) => f.id !== id),
    }));
  },

  forwardAllPending: async () => {
    const flows = get().pendingQueue;
    for (const flow of flows) {
      try {
        await resumeFlow(flow.id, { action: 'forward' });
      } catch (e) {
        // ignore
      }
    }
    set({ pendingQueue: [] });
  },
}));
