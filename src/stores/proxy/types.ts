import type { TrafficItem, InterceptConfig, InterceptRule, PendingFlow, ProxyStatus, ListenerConfig } from '../../types';
import type { HistorySettings, HistoryDetail } from '../../services/tauri/bridge';

export interface ProxyServerSlice {
  proxyStatus: ProxyStatus;
  listeners: ListenerConfig[];
  setProxyMode: (mode: 'normal' | 'intercept' | 'off') => Promise<void>;
  initProxyServer: () => Promise<void>;
  fetchListeners: () => Promise<void>;
  applyListeners: (listeners: ListenerConfig[]) => void;
  addListener: (label: string, address: string, replaceConflicts?: boolean) => Promise<void>;
  removeListener: (id: number) => Promise<void>;
  updateListener: (id: number, label?: string, address?: string, replaceConflicts?: boolean) => Promise<void>;
  setListenerEnabled: (id: number, enabled: boolean) => Promise<void>;
}

export interface TrafficSlice {
  traffic: TrafficItem[];
  selectedTrafficId: number | null;
  historySettings: HistorySettings;
  listenerFilter: string;

  // Pagination & Lazy Loading
  historyPage: number;
  pageSize: number;
  hasMore: boolean;
  isLoadingMore: boolean;
  totalDbCount: number;
  trafficDetails: Record<number, HistoryDetail>;

  // Filtering
  searchQuery: string;
  selectedMethods: string[];
  methodFilters: Record<string, 'include' | 'exclude' | 'neutral'>;
  flagFilters: Record<string, 'include' | 'exclude' | 'neutral'>;
  statusFilters: Record<string, 'include' | 'exclude' | 'neutral'>;
  statusCodeRange: 'all' | '2xx' | '3xx' | '4xx' | '5xx';
  onlyIntercepted: boolean;
  onlyRewritten: boolean;
  onlyFailed: boolean;
  /** Clears every filter except search */
  resetFilters: () => void;

  // Traffic actions
  initTraffic: () => Promise<void>;
  loadInitialTraffic: () => Promise<void>;
  loadNextPage: () => Promise<void>;
  fetchTrafficDetail: (id: number) => Promise<HistoryDetail | null>;
  cacheTrafficDetail: (detail: HistoryDetail) => void;
  deloadInactiveTraffic: () => void;
  fetchHistorySettings: () => Promise<void>;
  updateHistorySettings: (limiterEnabled: boolean, maxRows: number) => Promise<void>;
  addTrafficItem: (item: TrafficItem) => void;
  addTrafficBatch: (items: TrafficItem[]) => void;
  selectTrafficItem: (id: number | null) => void;
  clearTraffic: () => Promise<void>;
  deleteTrafficItem: (id: number) => void;
  setSearchQuery: (query: string) => void;
  setSelectedMethods: (methods: string[]) => void;
  setMethodFilter: (method: string, state: 'include' | 'exclude' | 'neutral') => void;
  setFlagFilter: (flag: string, state: 'include' | 'exclude' | 'neutral') => void;
  setStatusFilter: (status: string, state: 'include' | 'exclude' | 'neutral') => void;
  setStatusCodeRange: (range: 'all' | '2xx' | '3xx' | '4xx' | '5xx') => void;
  setOnlyIntercepted: (val: boolean) => void;
  setOnlyRewritten: (val: boolean) => void;
  setOnlyFailed: (val: boolean) => void;
  setListenerFilter: (listener: string) => void;
}

export interface InterceptSlice {
  interceptConfig: InterceptConfig;
  interceptRules: InterceptRule[];
  pendingQueue: PendingFlow[];

  toggleIntercept: (enabled?: boolean) => Promise<void>;
  setInterceptDirection: (direction: 'request' | 'response' | 'both') => Promise<void>;
  addInterceptRule: (rule: Omit<InterceptRule, 'id'>) => Promise<void>;
  toggleInterceptRule: (id: string) => Promise<void>;
  deleteInterceptRule: (id: string) => Promise<void>;
  forwardPendingFlow: (id: string, modifiedBody?: string, modifiedHeaders?: Record<string, string>) => Promise<void>;
  dropPendingFlow: (id: string) => Promise<void>;
  forwardAllPending: () => Promise<void>;
}

export type ProxyState = ProxyServerSlice & TrafficSlice & InterceptSlice & {
  initStore: () => Promise<void>;
};
