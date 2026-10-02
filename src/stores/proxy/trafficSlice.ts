import { StateCreator } from 'zustand';
import type { TrafficSlice, ProxyState } from './types';
import type { TrafficItem } from '../../types';
import {
  subscribeTrafficCaptured,
  getHistoryLogs,
  getHistoryDetail,
  getHistoryCount,
  clearHistoryLogs,
  getHistorySettings,
  updateHistorySettings,
  isTauriAvailable,
  HistoryDetail,
  HistoryFilterOptions,
} from '../../services/tauri/bridge';
import { getUiPref, useUiPrefsStore } from '../useUiPrefsStore';

export const mapHeaders = (headers: any): { key: string; value: string }[] => {
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

const parseTimestamp = (value: unknown): number | null => {
  if (typeof value !== 'string' || !value) return null;
  const ms = Date.parse(value);
  return Number.isNaN(ms) ? null : ms;
};

export const mapHistoryEntryToTrafficItem = (item: any): TrafficItem => {
  const reqHeaders = mapHeaders(item.requestHeaders || item.request_headers);
  const resHeaders = mapHeaders(item.responseHeaders || item.response_headers);
  const status = item.statusCode ?? item.status_code ?? 0;
  const resBody = item.responseBody ?? item.response_body ?? '';
  const phase = item.phase || 'response';
  const isFailed = Boolean(
    item.isFailed ||
    item.is_failed ||
    (phase === 'response' && status === 0) ||
    (status === 502 && resBody.includes('[MITM]'))
  );

  return {
    id: String(item.id),
    method: item.method || 'GET',
    host: item.host || '',
    path: item.path || item.url || '/',
    url: item.url || '',
    statusCode: status,
    contentType: item.contentType || item.content_type || 'text/plain',
    size: item.responseSize ?? item.size ?? 0,
    durationMs: item.durationMs ?? item.duration_ms ?? null,
    timestamp: item.createdAt ? Date.parse(item.createdAt) : item.timestamp || Date.now(),
    requestAt: parseTimestamp(item.requestAt ?? item.request_at),
    responseAt: parseTimestamp(item.responseAt ?? item.response_at),
    requestHeaders: reqHeaders,
    responseHeaders: resHeaders,
    requestBody: item.requestBody ?? item.request_body ?? '',
    responseBody: resBody,
    phase,
    listenerLabel: item.listenerLabel || item.listener_label || '',
    isIntercepted: Boolean(item.isIntercepted || item.is_intercepted),
    isRewritten: Boolean(item.isRewritten || item.is_rewritten),
    isFailed,
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

declare global {
  interface Window {
    __MITM_TRAFFIC_UNLISTEN__?: (() => void) | null;
  }
}

// Clean up any existing listener during Vite HMR
if (typeof window !== 'undefined' && window.__MITM_TRAFFIC_UNLISTEN__) {
  try {
    window.__MITM_TRAFFIC_UNLISTEN__();
  } catch (e) {
    console.warn('Failed to unlisten previous traffic listener:', e);
  }
  window.__MITM_TRAFFIC_UNLISTEN__ = null;
}

let incomingBatchMap = new Map<string, TrafficItem>();
let batchTimer: ReturnType<typeof setTimeout> | null = null;
let isTrafficSubscribing = false;

export const createTrafficSlice: StateCreator<
  ProxyState,
  [],
  [],
  TrafficSlice
> = (set, get) => {
  const flushIncomingBatch = () => {
    if (incomingBatchMap.size === 0) {
      batchTimer = null;
      return;
    }
    const batch = Array.from(incomingBatchMap.values());
    incomingBatchMap.clear();
    batchTimer = null;

    get().addTrafficBatch(batch);
  };

  const queueIncomingItem = (item: TrafficItem) => {
    const existing = incomingBatchMap.get(item.id);
    if (existing) {
      // If previous event was response, do not revert to earlier phase
      if (existing.phase === 'response' && item.phase !== 'response') {
        incomingBatchMap.set(item.id, { ...item, ...existing });
      } else {
        incomingBatchMap.set(item.id, { ...existing, ...item });
      }
    } else {
      incomingBatchMap.set(item.id, item);
    }
    if (!batchTimer) {
      batchTimer = setTimeout(flushIncomingBatch, 40);
    }
  };

  // Filter bar state (except search) survives restarts as the "history.filters" UI preference
  const persistFilters = () => {
    const { methodFilters, statusFilters, flagFilters, listenerFilter } = get();
    const activeOnly = (m: Record<string, string>) =>
      Object.fromEntries(Object.entries(m).filter(([, st]) => st !== 'neutral')) as Record<string, 'include' | 'exclude'>;
    useUiPrefsStore.getState().setPref('history.filters', {
      methods: activeOnly(methodFilters),
      statuses: activeOnly(statusFilters),
      flags: activeOnly(flagFilters),
      listener: listenerFilter,
    });
  };

  let filtersRestored = false;
  const restoreFilters = async () => {
    if (filtersRestored) return;
    filtersRestored = true;
    await useUiPrefsStore.getState().load();
    const f = getUiPref('history.filters');
    const includedStatuses = Object.entries(f.statuses).filter(([, st]) => st === 'include');
    set({
      methodFilters: f.methods,
      statusFilters: f.statuses,
      flagFilters: f.flags,
      listenerFilter: f.listener,
      statusCodeRange: includedStatuses.length === 1 ? (includedStatuses[0][0] as '2xx' | '3xx' | '4xx' | '5xx') : 'all',
      onlyIntercepted: f.flags['intercepted'] === 'include',
      onlyRewritten: f.flags['rewritten'] === 'include',
      onlyFailed: f.flags['failed'] === 'include',
    });
  };

  return {
    traffic: SAMPLE_TRAFFIC,
    selectedTrafficId: 'req-101',
    historySettings: {
      limiterEnabled: true,
      maxRows: 500,
    },

    historyPage: 1,
    pageSize: 100,
    hasMore: true,
    isLoadingMore: false,
    totalDbCount: 0,
    trafficDetails: {},

    searchQuery: '',
    selectedMethods: [],
    methodFilters: {},
    flagFilters: {},
    statusFilters: {},
    statusCodeRange: 'all',
    onlyIntercepted: false,
    onlyRewritten: false,
    onlyFailed: false,
    listenerFilter: '',

    initTraffic: async () => {
      await restoreFilters();
      if (isTauriAvailable()) {
        try {
          await get().fetchHistorySettings();
          await get().loadInitialTraffic();
        } catch (e) {
          console.warn('Failed to fetch initial HTTP history via IPC:', e);
        }
      }

      if (!isTrafficSubscribing && !(typeof window !== 'undefined' && window.__MITM_TRAFFIC_UNLISTEN__)) {
        isTrafficSubscribing = true;
        try {
          const unlisten = await subscribeTrafficCaptured((event: any) => {
            const raw = event?.entry || event;
            if (raw && raw.id) {
              const mappedItem = mapHistoryEntryToTrafficItem(raw);
              if (raw.requestBody || raw.responseBody || raw.request_body || raw.response_body) {
                get().cacheTrafficDetail({
                  id: String(raw.id),
                  requestHeaders: mappedItem.requestHeaders.map((h) => [h.key, h.value]),
                  responseHeaders: mappedItem.responseHeaders.map((h) => [h.key, h.value]),
                  requestBody: mappedItem.requestBody,
                  responseBody: mappedItem.responseBody,
                });
              }
              queueIncomingItem(mappedItem);
            }
          });
          if (typeof window !== 'undefined') {
            window.__MITM_TRAFFIC_UNLISTEN__ = unlisten;
          }
        } catch (e) {
          console.warn('Failed to subscribe to traffic captured:', e);
        } finally {
          isTrafficSubscribing = false;
        }
      }
    },

    loadInitialTraffic: async () => {
      if (!isTauriAvailable()) return;
      const { searchQuery, methodFilters, statusFilters, statusCodeRange, flagFilters, onlyIntercepted, onlyRewritten, onlyFailed, listenerFilter, pageSize } = get();

      const includeMethods = Object.entries(methodFilters)
        .filter(([_, st]) => st === 'include')
        .map(([m]) => m);
      const methodFilter = includeMethods.length === 1 ? includeMethods[0] : undefined;

      const includeStatuses = Object.entries(statusFilters)
        .filter(([_, st]) => st === 'include')
        .map(([s]) => s);
      const statusRange = includeStatuses.length === 1 ? includeStatuses[0] : (statusCodeRange !== 'all' ? statusCodeRange : undefined);

      const opts: HistoryFilterOptions = {
        searchTerm: searchQuery || undefined,
        methodFilter,
        statusRange,
        onlyIntercepted: (flagFilters['intercepted'] === 'include' || onlyIntercepted) || undefined,
        onlyRewritten: (flagFilters['rewritten'] === 'include' || onlyRewritten) || undefined,
        onlyFailed: (flagFilters['failed'] === 'include' || onlyFailed) || undefined,
        onlyWaiting: flagFilters['waiting'] === 'include' ? true : undefined,
        listenerFilter: listenerFilter || undefined,
        includeBodies: false,
      };

      set({ isLoadingMore: true, historyPage: 1 });
      try {
        const [logs, count] = await Promise.all([
          getHistoryLogs(1, pageSize, opts),
          getHistoryCount(opts),
        ]);

        if (logs) {
          const mapped = logs.map(mapHistoryEntryToTrafficItem);
          const nextSelectedId = get().selectedTrafficId || mapped[0]?.id || null;
          set({
            traffic: mapped,
            totalDbCount: count,
            hasMore: mapped.length >= pageSize,
            isLoadingMore: false,
            historyPage: 1,
            selectedTrafficId: nextSelectedId,
          });

          if (nextSelectedId) {
            get().fetchTrafficDetail(nextSelectedId);
          }
        }
      } catch (err) {
        console.warn('Failed to load initial traffic from SQLite:', err);
        set({ isLoadingMore: false });
      }
    },

    loadNextPage: async () => {
      if (!isTauriAvailable()) return;
      const { historyPage, pageSize, hasMore, isLoadingMore, searchQuery, methodFilters, statusFilters, statusCodeRange, flagFilters, onlyIntercepted, onlyRewritten, onlyFailed, listenerFilter } = get();
      if (!hasMore || isLoadingMore) return;

      set({ isLoadingMore: true });
      const nextPage = historyPage + 1;

      const includeMethods = Object.entries(methodFilters)
        .filter(([_, st]) => st === 'include')
        .map(([m]) => m);
      const methodFilter = includeMethods.length === 1 ? includeMethods[0] : undefined;

      const includeStatuses = Object.entries(statusFilters)
        .filter(([_, st]) => st === 'include')
        .map(([s]) => s);
      const statusRange = includeStatuses.length === 1 ? includeStatuses[0] : (statusCodeRange !== 'all' ? statusCodeRange : undefined);

      const opts: HistoryFilterOptions = {
        searchTerm: searchQuery || undefined,
        methodFilter,
        statusRange,
        onlyIntercepted: (flagFilters['intercepted'] === 'include' || onlyIntercepted) || undefined,
        onlyRewritten: (flagFilters['rewritten'] === 'include' || onlyRewritten) || undefined,
        onlyFailed: (flagFilters['failed'] === 'include' || onlyFailed) || undefined,
        onlyWaiting: flagFilters['waiting'] === 'include' ? true : undefined,
        listenerFilter: listenerFilter || undefined,
        includeBodies: false,
      };

      try {
        const logs = await getHistoryLogs(nextPage, pageSize, opts);
        if (logs && logs.length > 0) {
          const mapped = logs.map(mapHistoryEntryToTrafficItem);
          const existingIds = new Set(get().traffic.map((t) => t.id));
          const newUnique = mapped.filter((t) => !existingIds.has(t.id));

          set((state) => ({
            traffic: [...state.traffic, ...newUnique],
            historyPage: nextPage,
            hasMore: mapped.length >= pageSize,
            isLoadingMore: false,
          }));
        } else {
          set({ hasMore: false, isLoadingMore: false });
        }
      } catch (err) {
        console.warn('Failed to load next traffic page:', err);
        set({ isLoadingMore: false });
      }
    },

    fetchTrafficDetail: async (id: string) => {
      if (!id) return null;
      const cached = get().trafficDetails[id];
      if (cached) return cached;

      if (!isTauriAvailable()) return null;

      try {
        const detail = await getHistoryDetail(id);
        if (detail) {
          get().cacheTrafficDetail(detail);

          set((state) => {
            const idx = state.traffic.findIndex((t) => t.id === id);
            if (idx >= 0) {
              const updated = [...state.traffic];
              updated[idx] = {
                ...updated[idx],
                requestHeaders: mapHeaders(detail.requestHeaders),
                responseHeaders: mapHeaders(detail.responseHeaders),
                requestBody: detail.requestBody,
                responseBody: detail.responseBody,
              };
              return { traffic: updated };
            }
            return {};
          });

          return detail;
        }
      } catch (err) {
        console.warn(`Failed to fetch history detail for ${id}:`, err);
      }
      return null;
    },

    cacheTrafficDetail: (detail: HistoryDetail) => {
      set((state) => {
        const next = { ...state.trafficDetails, [detail.id]: detail };
        const keys = Object.keys(next);
        if (keys.length > 50) {
          delete next[keys[0]];
        }
        return { trafficDetails: next };
      });
    },

    deloadInactiveTraffic: () => {
      set((state) => {
        if (state.traffic.length <= 150) return {};
        const pruned = state.traffic.slice(0, 100);
        const retainedDetails: Record<string, HistoryDetail> = {};
        if (state.selectedTrafficId && state.trafficDetails[state.selectedTrafficId]) {
          retainedDetails[state.selectedTrafficId] = state.trafficDetails[state.selectedTrafficId];
        }
        return {
          traffic: pruned,
          trafficDetails: retainedDetails,
          historyPage: 1,
          hasMore: true,
        };
      });
    },

    fetchHistorySettings: async () => {
      if (isTauriAvailable()) {
        try {
          const settings = await getHistorySettings();
          set({ historySettings: settings });
        } catch (e) {
          console.warn('Failed to fetch history settings:', e);
        }
      }
    },

    updateHistorySettings: async (limiterEnabled, maxRows) => {
      set((state) => ({
        historySettings: { limiterEnabled, maxRows },
        traffic: limiterEnabled ? state.traffic.slice(0, maxRows) : state.traffic,
      }));

      if (isTauriAvailable()) {
        try {
          const updated = await updateHistorySettings(limiterEnabled, maxRows);
          set({ historySettings: updated });
          await get().loadInitialTraffic();
        } catch (e) {
          console.error('Failed to update history settings:', e);
        }
      }
    },

    addTrafficItem: (item) => {
      queueIncomingItem(item);
    },

    addTrafficBatch: (items: TrafficItem[]) => {
      if (!items || items.length === 0) return;

      set((state) => {
        const settings = state.historySettings;
        const maxLimit = settings.limiterEnabled ? settings.maxRows : 100000;
        const nextTraffic = [...state.traffic];
        const itemsToPrepend: TrafficItem[] = [];

        // Fast index map of existing items in traffic
        const existingMap = new Map<string, number>();
        for (let i = 0; i < nextTraffic.length; i++) {
          existingMap.set(nextTraffic[i].id, i);
        }

        // Intra-batch deduplication: resolve any duplicate items within the batch itself
        const incomingMap = new Map<string, TrafficItem>();
        for (const item of items) {
          const prev = incomingMap.get(item.id);
          if (prev) {
            if (prev.phase === 'response' && item.phase !== 'response') {
              incomingMap.set(item.id, { ...item, ...prev });
            } else {
              incomingMap.set(item.id, { ...prev, ...item });
            }
          } else {
            incomingMap.set(item.id, item);
          }
        }

        // Apply items: update existing in-place, or collect for prepending
        for (const item of incomingMap.values()) {
          const idx = existingMap.get(item.id);
          if (idx !== undefined && idx >= 0) {
            const current = nextTraffic[idx];
            if (current.phase === 'response' && item.phase !== 'response') {
              nextTraffic[idx] = { ...item, ...current };
            } else {
              nextTraffic[idx] = { ...current, ...item };
            }
          } else {
            itemsToPrepend.push(item);
          }
        }

        // Sort itemsToPrepend descending (highest numeric ID / newest timestamp at the top)
        itemsToPrepend.sort((a, b) => {
          const numA = Number(a.id);
          const numB = Number(b.id);
          if (!Number.isNaN(numA) && !Number.isNaN(numB)) {
            return numB - numA;
          }
          return (b.timestamp || 0) - (a.timestamp || 0);
        });

        // Combine and guarantee absolute ID uniqueness
        const combined = [...itemsToPrepend, ...nextTraffic];
        const seen = new Set<string>();
        const uniqueCombined: TrafficItem[] = [];
        for (const it of combined) {
          if (!seen.has(it.id)) {
            seen.add(it.id);
            uniqueCombined.push(it);
          }
        }

        const limited = uniqueCombined.slice(0, maxLimit);
        return {
          traffic: limited,
          totalDbCount: state.totalDbCount + itemsToPrepend.length,
        };
      });
    },

    selectTrafficItem: (id) => {
      set({ selectedTrafficId: id });
      if (id) {
        get().fetchTrafficDetail(id);
      }
    },

    clearTraffic: async () => {
      incomingBatchMap.clear();
      if (batchTimer) {
        clearTimeout(batchTimer);
        batchTimer = null;
      }
      try {
        await clearHistoryLogs();
      } catch (e) {
        console.warn('Failed to clear backend history via IPC');
      }
      set({
        traffic: [],
        selectedTrafficId: null,
        totalDbCount: 0,
        trafficDetails: {},
        historyPage: 1,
        hasMore: false,
      });
    },

    deleteTrafficItem: (id) => {
      set((state) => ({
        traffic: state.traffic.filter((t) => t.id !== id),
        selectedTrafficId: state.selectedTrafficId === id ? null : state.selectedTrafficId,
        totalDbCount: Math.max(0, state.totalDbCount - 1),
      }));
    },

    setSearchQuery: (searchQuery) => {
      set({ searchQuery });
      get().loadInitialTraffic();
    },
    setSelectedMethods: (selectedMethods) => {
      set({ selectedMethods });
      get().loadInitialTraffic();
    },
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
      persistFilters();
      get().loadInitialTraffic();
    },
    setFlagFilter: (flag, filterState) => {
      set((state) => {
        const nextFilters = { ...state.flagFilters };
        if (filterState === 'neutral') {
          delete nextFilters[flag];
        } else {
          nextFilters[flag] = filterState;
        }
        return {
          flagFilters: nextFilters,
          onlyIntercepted: nextFilters['intercepted'] === 'include',
          onlyRewritten: nextFilters['rewritten'] === 'include',
          onlyFailed: nextFilters['failed'] === 'include',
        };
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    setStatusFilter: (status, filterState) => {
      set((state) => {
        const nextFilters = { ...state.statusFilters };
        if (filterState === 'neutral') {
          delete nextFilters[status];
        } else {
          nextFilters[status] = filterState;
        }
        return { statusFilters: nextFilters };
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    setStatusCodeRange: (statusCodeRange) => {
      set((state) => {
        const nextFilters = { ...state.statusFilters };
        ['2xx', '3xx', '4xx', '5xx'].forEach((r) => {
          if (statusCodeRange === r) nextFilters[r] = 'include';
          else delete nextFilters[r];
        });
        return { statusCodeRange, statusFilters: nextFilters };
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    setOnlyIntercepted: (val) => {
      set((state) => {
        const nextFilters = { ...state.flagFilters };
        if (val) nextFilters['intercepted'] = 'include';
        else delete nextFilters['intercepted'];
        return { onlyIntercepted: val, flagFilters: nextFilters };
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    setOnlyRewritten: (val) => {
      set((state) => {
        const nextFilters = { ...state.flagFilters };
        if (val) nextFilters['rewritten'] = 'include';
        else delete nextFilters['rewritten'];
        return { onlyRewritten: val, flagFilters: nextFilters };
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    setOnlyFailed: (val) => {
      set((state) => {
        const nextFilters = { ...state.flagFilters };
        if (val) nextFilters['failed'] = 'include';
        else delete nextFilters['failed'];
        return { onlyFailed: val, flagFilters: nextFilters };
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    resetFilters: () => {
      set({
        methodFilters: {},
        statusFilters: {},
        flagFilters: {},
        listenerFilter: '',
        selectedMethods: [],
        statusCodeRange: 'all',
        onlyIntercepted: false,
        onlyRewritten: false,
        onlyFailed: false,
      });
      persistFilters();
      get().loadInitialTraffic();
    },
    setListenerFilter: (listenerFilter) => {
      set({ listenerFilter });
      persistFilters();
      get().loadInitialTraffic();
    },
  };
};
