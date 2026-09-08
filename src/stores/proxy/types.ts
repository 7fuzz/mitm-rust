import type { TrafficItem, InterceptConfig, InterceptRule, PendingFlow, ProxyStatus } from '../../types';
import type { HistorySettings, HistoryDetail } from '../../services/tauri/bridge';

export interface ProxyServerSlice {
  proxyStatus: ProxyStatus;
  setProxyMode: (mode: 'normal' | 'intercept' | 'off') => Promise<void>;
  updateProxyBindings: (bindings: string[]) => Promise<void>;
  initProxyServer: () => Promise<void>;
}

export interface TrafficSlice {
  traffic: TrafficItem[];
  selectedTrafficId: string | null;
  historySettings: HistorySettings;

  // Pagination & Lazy Loading
  historyPage: number;
  pageSize: number;
  hasMore: boolean;
  isLoadingMore: boolean;
  totalDbCount: number;
  trafficDetails: Record<string, HistoryDetail>;

  // Filtering
  searchQuery: string;
  selectedMethods: string[];
  methodFilters: Record<string, 'include' | 'exclude' | 'neutral'>;
  statusCodeRange: 'all' | '2xx' | '3xx' | '4xx' | '5xx';
  onlyIntercepted: boolean;
  onlyRewritten: boolean;
  onlyFailed: boolean;

  // Traffic actions
  initTraffic: () => Promise<void>;
  loadInitialTraffic: () => Promise<void>;
  loadNextPage: () => Promise<void>;
  fetchTrafficDetail: (id: string) => Promise<HistoryDetail | null>;
  cacheTrafficDetail: (detail: HistoryDetail) => void;
  deloadInactiveTraffic: () => void;
  fetchHistorySettings: () => Promise<void>;
  updateHistorySettings: (limiterEnabled: boolean, maxRows: number) => Promise<void>;
  addTrafficItem: (item: TrafficItem) => void;
  addTrafficBatch: (items: TrafficItem[]) => void;
  selectTrafficItem: (id: string | null) => void;
  clearTraffic: () => Promise<void>;
  deleteTrafficItem: (id: string) => void;
  setSearchQuery: (query: string) => void;
  setSelectedMethods: (methods: string[]) => void;
  setMethodFilter: (method: string, state: 'include' | 'exclude' | 'neutral') => void;
  setStatusCodeRange: (range: 'all' | '2xx' | '3xx' | '4xx' | '5xx') => void;
  setOnlyIntercepted: (val: boolean) => void;
  setOnlyRewritten: (val: boolean) => void;
  setOnlyFailed: (val: boolean) => void;
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
