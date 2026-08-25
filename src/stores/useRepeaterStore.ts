import { create } from 'zustand';
import type { TrafficItem } from '../types';
import type {
  RepeaterTab,
  RepeaterHistoryItem,
  RepeaterExecutionResult,
  HeaderItem,
  ParamItem,
} from '../services/tauri/bridge';
import {
  getRepeaterTabs,
  createRepeaterTab,
  updateRepeaterTab,
  deleteRepeaterTab,
  executeRepeaterRequest,
  getRepeaterHistory,
  insertRepeaterHistory,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';

interface RepeaterState {
  tabs: RepeaterTab[];
  activeTabId: string | null;
  viewMode: 'sidebar' | 'tabs';
  executionHistory: Record<string, RepeaterHistoryItem[]>;
  lastExecutionResult: Record<string, RepeaterExecutionResult | null>;
  isExecuting: Record<string, boolean>;
  isHistoryDrawerOpen: boolean;
  isCurlModalOpen: boolean;

  // Compatibility aliases for legacy components
  requests: RepeaterTab[];
  groups: any[];
  openTabIds: string[];
  lastExecutionResponse: Record<string, RepeaterExecutionResult | null>;

  // Actions
  initStore: () => Promise<void>;
  setViewMode: (mode: 'sidebar' | 'tabs') => void;
  setActiveTab: (id: string | null) => void;
  createNewRequest: (name?: string) => Promise<void>;
  sendToRepeater: (item: TrafficItem) => Promise<void>;
  updateTab: (tab: RepeaterTab) => Promise<void>;
  deleteTab: (id: string) => Promise<void>;
  openTab: (tab: RepeaterTab) => void;
  closeTab: (id: string) => void;

  // Execution
  executeActiveRequest: (id: string) => Promise<void>;
  fetchHistory: (id?: string) => Promise<void>;
  toggleHistoryDrawer: () => void;

  // Modals & Tools
  setHistoryDrawerOpen: (open?: boolean) => void;
  setCurlModalOpen: (open?: boolean) => void;
  importCurlCommand: (curlStr: string) => void;

  // Legacy Action Aliases
  createGroup: (name?: string) => Promise<void>;
  deleteGroup: (id?: string) => Promise<void>;
  updateRequest: (tab: RepeaterTab) => Promise<void>;
  deleteRequest: (id: string) => Promise<void>;
  getTabExecutionResult: (id: string) => RepeaterExecutionResult | null;
}

export const useRepeaterStore = create<RepeaterState>((set, get) => ({
  tabs: [],
  activeTabId: null,
  viewMode: 'sidebar',
  executionHistory: {},
  lastExecutionResult: {},
  isExecuting: {},
  isHistoryDrawerOpen: false,
  isCurlModalOpen: false,

  // Legacy compatibility getters
  get requests() {
    return get().tabs;
  },
  groups: [],
  get openTabIds() {
    return get().tabs.map((t) => t.id);
  },
  get lastExecutionResponse() {
    return get().lastExecutionResult;
  },

  getTabExecutionResult: (id: string) => {
    return get().lastExecutionResult[id] || null;
  },

  setViewMode: (mode) => set({ viewMode: mode }),

  initStore: async () => {
    if (isTauriAvailable()) {
      try {
        const tabs = await getRepeaterTabs();
        if (tabs && tabs.length > 0) {
          set({
            tabs,
            activeTabId: get().activeTabId || tabs[0].id,
          });
          // Auto-fetch histories for all loaded tabs
          for (const tab of tabs) {
            get().fetchHistory(tab.id);
          }
        } else {
          set({
            tabs: [],
            activeTabId: null,
          });
        }
      } catch (err) {
        console.warn('Failed to load repeater tabs from backend IPC:', err);
      }
    }
  },

  setActiveTab: (id) => {
    set({ activeTabId: id });
    if (id) {
      get().fetchHistory(id);
    }
  },

  createNewRequest: async (name) => {
    const tabName = name || 'Untitled Request';
    try {
      if (isTauriAvailable()) {
        const created = await createRepeaterTab(tabName);
        set((state) => ({
          tabs: [created, ...state.tabs.filter((t) => t.id !== created.id)],
          activeTabId: created.id,
        }));
        get().fetchHistory(created.id);
      } else {
        const newTab: RepeaterTab = {
          id: 'tab-' + Date.now(),
          name: tabName,
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

    const tabName = `${item.method} ${item.path || '/'}`;
    const nowMs = Date.now();

    // 4. Create the new tab
    let createdTab: RepeaterTab;
    if (isTauriAvailable()) {
      createdTab = await createRepeaterTab(tabName);
    } else {
      createdTab = {
        id: 'tab-' + nowMs,
        name: tabName,
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
      name: tabName,
      method: item.method,
      url: item.url,
      headers,
      params,
      bodyType,
      bodyContent,
      executionCount: 1,
      lastStatusCode: item.statusCode,
      lastDurationMs: item.durationMs,
    };

    await get().updateTab(populatedTab);

    // Always place newly sent request at the TOP of the tabs list
    set((state) => ({
      tabs: [populatedTab, ...state.tabs.filter((t) => t.id !== populatedTab.id)],
      activeTabId: populatedTab.id,
    }));

    // 5. Construct initial execution result & history item from traffic capture
    const responseHeaders: HeaderItem[] = (item.responseHeaders || []).map((h, i) => ({
      id: `rh-${nowMs}-${i}`,
      key: h.key,
      value: h.value,
      enabled: true,
    }));

    const executionResult: RepeaterExecutionResult = {
      historyId: nowMs,
      repeaterId: populatedTab.id,
      statusCode: item.statusCode || 200,
      statusText: item.statusCode === 200 ? 'OK' : `HTTP ${item.statusCode}`,
      responseHeaders,
      responseBody: item.responseBody || '',
      durationMs: item.durationMs || 0,
      responseSize: item.size || (item.responseBody?.length || 0),
    };

    set((state) => ({
      lastExecutionResult: { ...state.lastExecutionResult, [populatedTab.id]: executionResult },
    }));

    if (isTauriAvailable()) {
      try {
        const historyItem: RepeaterHistoryItem = {
          id: 0,
          repeaterId: populatedTab.id,
          method: item.method,
          url: item.url,
          requestHeaders: headers,
          requestBody: bodyContent ? bodyContent : undefined,
          statusCode: item.statusCode || 200,
          responseHeaders,
          responseBody: item.responseBody ? item.responseBody : undefined,
          durationMs: item.durationMs || 0,
          executedAtMs: item.timestamp || nowMs,
        };
        await insertRepeaterHistory(historyItem);
        await get().fetchHistory(populatedTab.id);
      } catch (err) {
        console.error('Failed to insert repeater history from traffic item:', err);
      }
    } else {
      const mockHistory: RepeaterHistoryItem = {
        id: nowMs,
        repeaterId: populatedTab.id,
        method: item.method,
        url: item.url,
        requestHeaders: headers,
        requestBody: bodyContent,
        statusCode: item.statusCode || 200,
        responseHeaders,
        responseBody: item.responseBody,
        durationMs: item.durationMs || 0,
        executedAtMs: item.timestamp || nowMs,
      };
      set((state) => ({
        executionHistory: {
          ...state.executionHistory,
          [populatedTab.id]: [mockHistory],
        },
      }));
    }
  },

  updateTab: async (updatedTab) => {
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === updatedTab.id ? updatedTab : t)),
    }));
    if (isTauriAvailable()) {
      try {
        await updateRepeaterTab(updatedTab);
      } catch (err) {
        console.error('Failed to update repeater tab:', err);
      }
    }
  },

  updateRequest: async (updatedTab) => {
    return get().updateTab(updatedTab);
  },

  deleteTab: async (id) => {
    const { tabs, activeTabId } = get();
    const nextTabs = tabs.filter((t) => t.id !== id);
    let nextActive = activeTabId;
    if (activeTabId === id) {
      nextActive = nextTabs.length > 0 ? nextTabs[0].id : null;
    }
    set({ tabs: nextTabs, activeTabId: nextActive });

    if (isTauriAvailable()) {
      try {
        await deleteRepeaterTab(id);
      } catch (err) {
        console.error('Failed to delete repeater tab:', err);
      }
    }
  },

  deleteRequest: async (id) => {
    return get().deleteTab(id);
  },

  openTab: (tab) => {
    const { tabs } = get();
    const filtered = tabs.filter((t) => t.id !== tab.id);
    set({ tabs: [tab, ...filtered], activeTabId: tab.id });
    get().fetchHistory(tab.id);
  },

  closeTab: (id) => {
    get().deleteTab(id);
  },

  createGroup: async () => {},
  deleteGroup: async () => {},

  importCurlCommand: (curlStr) => {
    if (!curlStr.trim()) return;
    get().createNewRequest('cURL Import');
  },

  executeActiveRequest: async (id) => {
    const activeTab = get().tabs.find((t) => t.id === id);
    if (activeTab && isTauriAvailable()) {
      try {
        await updateRepeaterTab(activeTab);
      } catch (err) {
        console.warn('Failed to sync tab before execution:', err);
      }
    }

    set((state) => ({
      isExecuting: { ...state.isExecuting, [id]: true },
    }));

    try {
      if (isTauriAvailable()) {
        const result = await executeRepeaterRequest(id);
        set((state) => ({
          isExecuting: { ...state.isExecuting, [id]: false },
          lastExecutionResult: { ...state.lastExecutionResult, [id]: result },
        }));
        await get().fetchHistory(id);
        // Refresh tab metadata (executionCount, lastStatusCode, lastDurationMs)
        const updatedTabs = await getRepeaterTabs();
        if (updatedTabs) {
          set({ tabs: updatedTabs });
        }
      } else {
        const tab = get().tabs.find((t) => t.id === id);
        const mockResult: RepeaterExecutionResult = {
          historyId: Date.now(),
          repeaterId: id,
          statusCode: 200,
          statusText: 'OK',
          responseHeaders: [
            { id: 'rh1', key: 'Content-Type', value: 'application/json', enabled: true },
            { id: 'rh2', key: 'Server', value: 'gunicorn/19.9.0', enabled: true },
          ],
          responseBody: JSON.stringify({
            args: {},
            headers: { 'User-Agent': 'MITM-Developer-Studio/2.0' },
            origin: '127.0.0.1',
            url: tab?.url || 'https://httpbin.org/get',
          }, null, 2),
          durationMs: 64,
          responseSize: 312,
        };
        set((state) => ({
          isExecuting: { ...state.isExecuting, [id]: false },
          lastExecutionResult: { ...state.lastExecutionResult, [id]: mockResult },
          tabs: state.tabs.map((t) =>
            t.id === id
              ? {
                  ...t,
                  executionCount: (t.executionCount || 0) + 1,
                  lastStatusCode: 200,
                  lastDurationMs: 64,
                }
              : t
          ),
        }));
      }
    } catch (err) {
      console.error('Failed to execute repeater request:', err);
      const errMessage = typeof err === 'string' ? err : (err as any)?.message || JSON.stringify(err);
      const errorResult: RepeaterExecutionResult = {
        historyId: 0,
        repeaterId: id,
        statusCode: 0,
        statusText: 'ERR_FAILED',
        responseHeaders: [],
        responseBody: `Error executing request: ${errMessage}`,
        durationMs: 0,
        responseSize: 0,
      };
      set((state) => ({
        isExecuting: { ...state.isExecuting, [id]: false },
        lastExecutionResult: { ...state.lastExecutionResult, [id]: errorResult },
      }));
    }
  },

  fetchHistory: async (id) => {
    const targetId = id || get().activeTabId;
    if (!targetId) return;

    if (isTauriAvailable()) {
      try {
        const history = await getRepeaterHistory(targetId, 1, 50);
        set((state) => ({
          executionHistory: { ...state.executionHistory, [targetId]: history },
        }));
      } catch (err) {
        console.error('Failed to fetch repeater history:', err);
      }
    }
  },

  toggleHistoryDrawer: () => {
    set((state) => ({ isHistoryDrawerOpen: !state.isHistoryDrawerOpen }));
  },

  setHistoryDrawerOpen: (open) => {
    set((state) => ({
      isHistoryDrawerOpen: typeof open === 'boolean' ? open : !state.isHistoryDrawerOpen,
    }));
  },

  setCurlModalOpen: (open) => {
    set((state) => ({
      isCurlModalOpen: typeof open === 'boolean' ? open : !state.isCurlModalOpen,
    }));
  },
}));
