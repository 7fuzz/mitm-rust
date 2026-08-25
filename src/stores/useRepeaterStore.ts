import { create } from 'zustand';
import type { TrafficItem } from '../types';
import {
  getRepeaterTabs,
  createRepeaterTab,
  updateRepeaterTab,
  deleteRepeaterTab,
  executeRepeaterRequest,
  getRepeaterHistory,
  insertRepeaterHistory,
  RepeaterTab,
  HeaderItem,
  ParamItem,
  RepeaterExecutionResult,
  RepeaterHistoryItem,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';

interface RepeaterState {
  tabs: RepeaterTab[];
  activeTabId: string | null;
  isExecuting: Record<string, boolean>;
  executionHistory: Record<string, RepeaterHistoryItem[]>;
  lastExecutionResult: Record<string, RepeaterExecutionResult | null>;
  isHistoryDrawerOpen: boolean;
  viewMode: 'sidebar' | 'tabs';

  // Compatibility properties for legacy components
  groups: any[];
  requests: RepeaterTab[];
  openTabIds: string[];
  lastExecutionResponse: Record<string, any>;
  isCurlModalOpen: boolean;

  // Actions
  initStore: () => Promise<void>;
  setActiveTab: (id: string | null) => void;
  createNewRequest: () => Promise<void>;
  updateTab: (tab: RepeaterTab) => Promise<void>;
  deleteTab: (id: string) => Promise<void>;
  sendToRepeater: (item: TrafficItem) => Promise<void>;
  executeActiveRequest: (tabId?: string) => Promise<void>;
  fetchHistory: (tabId: string) => Promise<void>;
  toggleHistoryDrawer: (open?: boolean) => void;
  setHistoryDrawerOpen: (open?: boolean) => void;
  setViewMode: (mode: 'sidebar' | 'tabs') => void;

  // Compatibility methods for legacy components
  closeTab: (id: string) => void;
  openTab: (id: string) => void;
  createGroup: (name: string) => void;
  deleteGroup: (id: string) => void;
  setCurlModalOpen: (open: boolean) => void;
  importCurlCommand: (curl: string) => void;
}

export const useRepeaterStore = create<RepeaterState>((set, get) => ({
  tabs: [],
  activeTabId: null,
  isExecuting: {},
  executionHistory: {},
  lastExecutionResult: {},
  isHistoryDrawerOpen: false,
  viewMode: 'sidebar',

  // Compatibility state defaults
  groups: [],
  requests: [],
  openTabIds: [],
  lastExecutionResponse: {},
  isCurlModalOpen: false,

  initStore: async () => {
    if (isTauriAvailable()) {
      try {
        const fetchedTabs = await getRepeaterTabs();
        set({ tabs: fetchedTabs, requests: fetchedTabs });
        if (fetchedTabs.length > 0 && !get().activeTabId) {
          const firstId = fetchedTabs[0].id;
          set({ activeTabId: firstId });
          get().fetchHistory(firstId);
        }
      } catch (err) {
        console.error('Failed to load repeater tabs:', err);
      }
    }
  },

  setActiveTab: (id) => {
    set({ activeTabId: id });
    if (id) {
      get().fetchHistory(id);
    }
  },

  createNewRequest: async () => {
    try {
      if (isTauriAvailable()) {
        const created = await createRepeaterTab();
        set((state) => ({
          tabs: [created, ...state.tabs.filter((t) => t.id !== created.id)],
          requests: [created, ...state.tabs.filter((t) => t.id !== created.id)],
          activeTabId: created.id,
        }));
        get().fetchHistory(created.id);
      } else {
        const newTab: RepeaterTab = {
          id: 'tab-' + Date.now(),
          method: 'GET',
          url: 'https://httpbin.org/get',
          headers: [{ id: 'h-' + Date.now(), key: 'User-Agent', value: 'MITM-Developer-Studio', enabled: true }],
          params: [],
          bodyType: 'none',
          bodyContent: '',
          extractRules: [],
          orderIndex: get().tabs.length,
          createdAtMs: Date.now(),
          updatedAtMs: Date.now(),
          executionCount: 0,
        };
        set((state) => ({
          tabs: [newTab, ...state.tabs],
          requests: [newTab, ...state.tabs],
          activeTabId: newTab.id,
        }));
      }
    } catch (err) {
      console.error('Failed to create repeater tab:', err);
    }
  },

  sendToRepeater: async (item: TrafficItem) => {
    // 1. Extract query params from URL
    const params: ParamItem[] = [];
    try {
      const rawUrl = item.url || item.path || '';
      const urlObj = new URL(rawUrl.startsWith('http') ? rawUrl : `http://localhost${rawUrl.startsWith('/') ? '' : '/'}${rawUrl}`);
      urlObj.searchParams.forEach((value, key) => {
        params.push({
          id: `p-${Date.now()}-${Math.random().toString(36).substr(2, 4)}`,
          key,
          value,
          enabled: true,
        });
      });
    } catch (e) {}

    // 2. Map request headers
    const headers: HeaderItem[] = (item.requestHeaders || []).map((h, idx) => ({
      id: `h-${Date.now()}-${idx}`,
      key: h.key,
      value: h.value,
      enabled: true,
    }));

    // 3. Determine bodyType & bodyContent
    let bodyType = 'none';
    let bodyContent = item.requestBody || '';
    if (bodyContent.trim()) {
      bodyType = (bodyContent.trim().startsWith('{') || bodyContent.trim().startsWith('[')) ? 'json' : 'raw';
    }

    const nowMs = Date.now();

    // 4. Create the new tab
    let createdTab: RepeaterTab;
    if (isTauriAvailable()) {
      createdTab = await createRepeaterTab();
    } else {
      createdTab = {
        id: 'tab-' + nowMs,
        method: item.method,
        url: item.url,
        headers: [],
        params: [],
        bodyType: 'none',
        bodyContent: '',
        extractRules: [],
        orderIndex: get().tabs.length,
        createdAtMs: nowMs,
        updatedAtMs: nowMs,
        executionCount: 1,
      };
    }

    // Populate full request details on tab
    const populatedTab: RepeaterTab = {
      ...createdTab,
      method: item.method || 'GET',
      url: item.url || 'https://httpbin.org/get',
      headers,
      params,
      bodyType,
      bodyContent,
      updatedAtMs: nowMs,
      executionCount: 1,
      lastStatusCode: item.statusCode,
      lastDurationMs: item.durationMs,
    };

    // Store tab in backend DB & state
    if (isTauriAvailable()) {
      await updateRepeaterTab(populatedTab);
    }

    // Prepend new tab to top of tabs list
    set((state) => ({
      tabs: [populatedTab, ...state.tabs.filter((t) => t.id !== populatedTab.id)],
      requests: [populatedTab, ...state.tabs.filter((t) => t.id !== populatedTab.id)],
      activeTabId: populatedTab.id,
    }));

    // 5. Construct initial execution result from traffic item history
    const initialResult: RepeaterExecutionResult = {
      historyId: 0,
      repeaterId: populatedTab.id,
      statusCode: item.statusCode || 200,
      statusText: 'OK',
      responseHeaders: (item.responseHeaders || []).map((h, i) => ({
        id: `resh-${i}`,
        key: h.key,
        value: h.value,
        enabled: true,
      })),
      responseBody: item.responseBody || '',
      durationMs: item.durationMs || 0,
      responseSize: item.size || (item.responseBody ? item.responseBody.length : 0),
    };

    set((state) => ({
      lastExecutionResult: {
        ...state.lastExecutionResult,
        [populatedTab.id]: initialResult,
      },
    }));

    // Insert history record in backend database
    if (isTauriAvailable()) {
      try {
        await insertRepeaterHistory({
          id: 0,
          repeaterId: populatedTab.id,
          method: populatedTab.method,
          url: populatedTab.url,
          requestHeaders: headers,
          requestBody: bodyContent,
          statusCode: item.statusCode || 200,
          responseHeaders: initialResult.responseHeaders,
          responseBody: item.responseBody || '',
          durationMs: item.durationMs || 0,
          executedAtMs: nowMs,
        });
        get().fetchHistory(populatedTab.id);
      } catch (e) {
        console.warn('Failed to insert initial history log via IPC:', e);
      }
    }
  },

  updateTab: async (updatedTab) => {
    const nextTab = { ...updatedTab, updatedAtMs: Date.now() };
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === nextTab.id ? nextTab : t)),
      requests: state.tabs.map((t) => (t.id === nextTab.id ? nextTab : t)),
    }));

    if (isTauriAvailable()) {
      try {
        await updateRepeaterTab(nextTab);
      } catch (err) {
        console.error('Failed to save repeater tab via IPC:', err);
      }
    }
  },

  deleteTab: async (id) => {
    const remaining = get().tabs.filter((t) => t.id !== id);
    let nextActiveId = get().activeTabId;

    if (get().activeTabId === id) {
      nextActiveId = remaining[0]?.id || null;
    }

    set({
      tabs: remaining,
      requests: remaining,
      activeTabId: nextActiveId,
    });

    if (isTauriAvailable()) {
      try {
        await deleteRepeaterTab(id);
      } catch (err) {
        console.error('Failed to delete repeater tab via IPC:', err);
      }
    }

    if (nextActiveId) {
      get().fetchHistory(nextActiveId);
    }
  },

  executeActiveRequest: async (tabId) => {
    const targetId = tabId || get().activeTabId;
    if (!targetId) return;

    set((state) => ({
      isExecuting: { ...state.isExecuting, [targetId]: true },
    }));

    try {
      if (isTauriAvailable()) {
        const result = await executeRepeaterRequest(targetId);
        set((state) => ({
          lastExecutionResult: { ...state.lastExecutionResult, [targetId]: result },
          isExecuting: { ...state.isExecuting, [targetId]: false },
        }));
        await get().fetchHistory(targetId);
        // Refresh tabs to update last status code & latency stats
        const refreshedTabs = await getRepeaterTabs();
        set({ tabs: refreshedTabs, requests: refreshedTabs });
      } else {
        // Fallback mock execution
        await new Promise((r) => setTimeout(r, 450));
        const mockResult: RepeaterExecutionResult = {
          historyId: Date.now(),
          repeaterId: targetId,
          statusCode: 200,
          statusText: 'OK',
          responseHeaders: [
            { id: '1', key: 'Content-Type', value: 'application/json', enabled: true },
            { id: '2', key: 'Server', value: 'mock-repeater-server/1.0', enabled: true },
          ],
          responseBody: JSON.stringify({ message: 'Repeater execution successful', tabId: targetId }, null, 2),
          durationMs: 45,
          responseSize: 184,
        };
        set((state) => ({
          lastExecutionResult: { ...state.lastExecutionResult, [targetId]: mockResult },
          isExecuting: { ...state.isExecuting, [targetId]: false },
        }));
      }
    } catch (err: any) {
      console.error('Failed to execute repeater request:', err);
      set((state) => ({
        isExecuting: { ...state.isExecuting, [targetId]: false },
      }));
    }
  },

  fetchHistory: async (tabId: string) => {
    if (!tabId || !isTauriAvailable()) return;
    try {
      const historyLogs = await getRepeaterHistory(tabId, 1, 100);
      set((state) => ({
        executionHistory: {
          ...state.executionHistory,
          [tabId]: historyLogs,
        },
      }));
    } catch (err) {
      console.error('Failed to fetch repeater execution history:', err);
    }
  },

  toggleHistoryDrawer: (open) => {
    const nextState = open !== undefined ? open : !get().isHistoryDrawerOpen;
    set({ isHistoryDrawerOpen: nextState });
    if (nextState && get().activeTabId) {
      get().fetchHistory(get().activeTabId!);
    }
  },

  setHistoryDrawerOpen: (open) => {
    get().toggleHistoryDrawer(open);
  },

  setViewMode: (viewMode) => set({ viewMode }),

  // Legacy compatibility methods
  closeTab: (id) => get().deleteTab(id),
  openTab: (id) => get().setActiveTab(id),
  createGroup: () => {},
  deleteGroup: () => {},
  setCurlModalOpen: (open) => set({ isCurlModalOpen: open }),
  importCurlCommand: () => {},
}));
