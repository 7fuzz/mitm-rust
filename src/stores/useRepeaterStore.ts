import { create } from 'zustand';
import type { RepeaterTab, RepeaterHistoryItem, RepeaterExecutionResult } from '../services/tauri/bridge';
import {
  getRepeaterTabs,
  createRepeaterTab,
  updateRepeaterTab,
  deleteRepeaterTab,
  executeRepeaterRequest,
  getRepeaterHistory,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';

const SAMPLE_TABS: RepeaterTab[] = [
  {
    id: 'tab-1',
    name: 'Get User Profile',
    method: 'GET',
    url: 'https://httpbin.org/get',
    headers: [
      { id: 'h1', key: 'User-Agent', value: 'MITM-Developer-Studio/2.0', enabled: true },
      { id: 'h2', key: 'Accept', value: 'application/json', enabled: true },
    ],
    params: [{ id: 'p1', key: 'verbose', value: 'true', enabled: true }],
    bodyType: 'none',
    bodyContent: '',
    extractRules: [],
    orderIndex: 0,
    createdAtMs: Date.now() - 60000,
    updatedAtMs: Date.now() - 60000,
  },
  {
    id: 'tab-2',
    name: 'Post Echo JSON',
    method: 'POST',
    url: 'https://httpbin.org/post',
    headers: [
      { id: 'h3', key: 'Content-Type', value: 'application/json', enabled: true },
    ],
    params: [],
    bodyType: 'json',
    bodyContent: JSON.stringify({ message: 'Hello from MITM Repeater!', timestamp: Date.now() }, null, 2),
    extractRules: [],
    orderIndex: 1,
    createdAtMs: Date.now() - 30000,
    updatedAtMs: Date.now() - 30000,
  },
];

interface RepeaterState {
  tabs: RepeaterTab[];
  activeTabId: string | null;
  executionHistory: Record<string, RepeaterHistoryItem[]>;
  lastExecutionResult: Record<string, RepeaterExecutionResult | null>;
  isExecuting: Record<string, boolean>;
  isHistoryDrawerOpen: boolean;
  isCurlModalOpen: boolean;

  // Compatibility aliases for legacy components
  requests: RepeaterTab[];
  groups: any[];
  openTabIds: string[];
  lastExecutionResponse: Record<string, any>;

  initStore: () => Promise<void>;
  setActiveTab: (id: string | null) => void;
  createNewRequest: (name?: string | null) => Promise<void>;
  updateTab: (tab: RepeaterTab) => Promise<void>;
  updateRequest: (tab: RepeaterTab) => Promise<void>;
  deleteTab: (id: string) => Promise<void>;
  deleteRequest: (id: string) => Promise<void>;
  openTab: (tab: RepeaterTab) => void;
  closeTab: (id: string) => void;
  createGroup: (name: string, description?: string) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;
  executeActiveRequest: (id: string) => Promise<void>;
  fetchHistory: (repeaterId: string) => Promise<void>;
  toggleHistoryDrawer: (open?: boolean) => void;
  setCurlModalOpen: (open: boolean) => void;
  importCurlCommand: (curlString: string, name?: string) => Promise<void>;
}

export const useRepeaterStore = create<RepeaterState>((set, get) => ({
  tabs: SAMPLE_TABS,
  activeTabId: 'tab-1',
  executionHistory: {},
  lastExecutionResult: {},
  isExecuting: {},
  isHistoryDrawerOpen: false,
  isCurlModalOpen: false,

  // Compatibility getters/state aliases
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

  initStore: async () => {
    if (isTauriAvailable()) {
      try {
        const tabs = await getRepeaterTabs();
        if (tabs && tabs.length > 0) {
          set({
            tabs,
            activeTabId: tabs[0].id,
          });
        }
      } catch (err) {
        console.warn('Failed to load repeater tabs from backend IPC:', err);
      }
    }
  },

  setActiveTab: (id) => set({ activeTabId: id }),

  createNewRequest: async (name) => {
    const tabName = name || 'Untitled Request';
    try {
      if (isTauriAvailable()) {
        const created = await createRepeaterTab(tabName);
        set((state) => ({
          tabs: [...state.tabs, created],
          activeTabId: created.id,
        }));
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
        };
        set((state) => ({
          tabs: [...state.tabs, newTab],
          activeTabId: newTab.id,
        }));
      }
    } catch (err) {
      console.error('Failed to create repeater tab:', err);
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
      nextActive = nextTabs.length > 0 ? nextTabs[nextTabs.length - 1].id : null;
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
    if (!tabs.some((t) => t.id === tab.id)) {
      set({ tabs: [...tabs, tab], activeTabId: tab.id });
    } else {
      set({ activeTabId: tab.id });
    }
  },

  closeTab: (id) => {
    get().deleteTab(id);
  },

  createGroup: async () => {},
  deleteGroup: async () => {},

  executeActiveRequest: async (id) => {
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
        get().fetchHistory(id);
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
        }));
      }
    } catch (err) {
      console.error('Failed to execute repeater request:', err);
      set((state) => ({
        isExecuting: { ...state.isExecuting, [id]: false },
      }));
    }
  },

  fetchHistory: async (repeaterId) => {
    if (isTauriAvailable()) {
      try {
        const historyItems = await getRepeaterHistory(repeaterId, 1, 50);
        set((state) => ({
          executionHistory: {
            ...state.executionHistory,
            [repeaterId]: historyItems,
          },
        }));
      } catch (err) {
        console.error('Failed to fetch repeater history:', err);
      }
    }
  },

  toggleHistoryDrawer: (open) => {
    set((state) => ({
      isHistoryDrawerOpen: open !== undefined ? open : !state.isHistoryDrawerOpen,
    }));
  },

  setCurlModalOpen: (isCurlModalOpen) => set({ isCurlModalOpen }),

  importCurlCommand: async (curlString, name = 'Imported cURL') => {
    let method = 'GET';
    if (curlString.includes('-X POST') || curlString.includes('--request POST')) method = 'POST';
    else if (curlString.includes('-X PUT') || curlString.includes('--request PUT')) method = 'PUT';
    else if (curlString.includes('-X DELETE') || curlString.includes('--request DELETE')) method = 'DELETE';

    const urlMatch = curlString.match(/https?:\/\/[^\s"']+/);
    const url = urlMatch ? urlMatch[0] : 'https://httpbin.org/get';

    const newTab: RepeaterTab = {
      id: 'tab-' + Date.now(),
      name,
      method,
      url,
      headers: [{ id: 'h1', key: 'User-Agent', value: 'MITM-Developer-Studio', enabled: true }],
      params: [],
      bodyType: method === 'GET' ? 'none' : 'json',
      bodyContent: '{\n  "imported": true\n}',
      extractRules: [],
      orderIndex: get().tabs.length,
      createdAtMs: Date.now(),
      updatedAtMs: Date.now(),
    };

    if (isTauriAvailable()) {
      const created = await createRepeaterTab(name);
      created.method = method;
      created.url = url;
      created.bodyType = method === 'GET' ? 'none' : 'json';
      created.bodyContent = '{\n  "imported": true\n}';
      await updateRepeaterTab(created);
      set((state) => ({
        tabs: [...state.tabs, created],
        activeTabId: created.id,
        isCurlModalOpen: false,
      }));
    } else {
      set((state) => ({
        tabs: [...state.tabs, newTab],
        activeTabId: newTab.id,
        isCurlModalOpen: false,
      }));
    }
  },
}));
