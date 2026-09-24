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
import { parseUrlQueryParams } from '../utils/urlParams';

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
  setExecutionResult: (tabId: string, result: RepeaterExecutionResult | RepeaterHistoryItem | null) => void;
  restoreHistoryToTab: (tabId: string, hist: RepeaterHistoryItem) => Promise<void>;
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

export const getDefaultRepeaterHeaders = (): HeaderItem[] => [
  {
    id: `h-ua-${Date.now()}-1`,
    key: 'User-Agent',
    value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36',
    enabled: true,
  },
  {
    id: `h-acc-${Date.now()}-2`,
    key: 'Accept',
    value: '*/*',
    enabled: true,
  },
  {
    id: `h-lang-${Date.now()}-3`,
    key: 'Accept-Language',
    value: 'en-US,en;q=0.9',
    enabled: true,
  },
  {
    id: `h-enc-${Date.now()}-4`,
    key: 'Accept-Encoding',
    value: 'gzip, deflate, br, zstd',
    enabled: true,
  },
  {
    id: `h-conn-${Date.now()}-5`,
    key: 'Connection',
    value: 'keep-alive',
    enabled: true,
  },
  {
    id: `h-fetch-dest-${Date.now()}-6`,
    key: 'Sec-Fetch-Dest',
    value: 'empty',
    enabled: true,
  },
  {
    id: `h-fetch-mode-${Date.now()}-7`,
    key: 'Sec-Fetch-Mode',
    value: 'cors',
    enabled: true,
  },
  {
    id: `h-fetch-site-${Date.now()}-8`,
    key: 'Sec-Fetch-Site',
    value: 'same-site',
    enabled: true,
  },
  {
    id: `h-ct-${Date.now()}-9`,
    key: 'Content-Type',
    value: 'application/json',
    enabled: false,
  },
];

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
        if (fetchedTabs && fetchedTabs.length > 0) {
          set({ tabs: fetchedTabs, requests: fetchedTabs });
          if (!get().activeTabId) {
            const firstId = fetchedTabs[0].id;
            set({ activeTabId: firstId });
            get().fetchHistory(firstId);
          }
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
    let created: RepeaterTab | null = null;
    if (isTauriAvailable()) {
      try {
        created = await createRepeaterTab();
      } catch (err) {
        console.warn('[RepeaterStore] createRepeaterTab IPC error, creating in-memory fallback tab:', err);
      }
    }

    const nowMs = Date.now();
    if (!created) {
      created = {
        id: 'tab-' + nowMs,
        method: 'GET',
        url: 'https://httpbin.org/get',
        headers: getDefaultRepeaterHeaders(),
        params: [],
        bodyType: 'none',
        bodyContent: '',
        extractRules: [],
        orderIndex: get().tabs.length,
        createdAtMs: nowMs,
        updatedAtMs: nowMs,
        executionCount: 0,
      };
    }

    const newTab = created;
    set((state) => ({
      tabs: [newTab, ...state.tabs.filter((t) => t.id !== newTab.id)],
      requests: [newTab, ...state.tabs.filter((t) => t.id !== newTab.id)],
      activeTabId: newTab.id,
    }));

    if (isTauriAvailable()) {
      try {
        await get().fetchHistory(newTab.id);
      } catch (e) {}
    }
  },

  sendToRepeater: async (item: TrafficItem) => {
    // 1. Extract query params from URL
    const rawUrl = item.url || item.path || '';
    const params: ParamItem[] = parseUrlQueryParams(rawUrl, []);

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
    let createdTab: RepeaterTab | null = null;
    if (isTauriAvailable()) {
      try {
        createdTab = await createRepeaterTab();
      } catch (e) {
        console.warn('[RepeaterStore] createRepeaterTab IPC error in sendToRepeater:', e);
      }
    }
    if (!createdTab) {
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
      lastDurationMs: item.durationMs ?? undefined,
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

    // Flush any unsaved active tab changes to backend DB before execution
    const currentTab = get().tabs.find((t) => t.id === targetId);
    if (currentTab && isTauriAvailable()) {
      try {
        await updateRepeaterTab(currentTab);
      } catch (e) {}
    }

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

  setExecutionResult: (tabId, result) => {
    set((state) => ({
      lastExecutionResult: {
        ...state.lastExecutionResult,
        [tabId]: result as any,
      },
    }));
  },

  restoreHistoryToTab: async (tabId, hist) => {
    const tab = get().tabs.find((t) => t.id === tabId);
    if (!tab) return;
    const params = parseUrlQueryParams(hist.url, tab.params || []);
    const updatedTab: RepeaterTab = {
      ...tab,
      method: hist.method,
      url: hist.url,
      params,
      headers: hist.requestHeaders,
      bodyContent: hist.requestBody || '',
      updatedAtMs: Date.now(),
    };
    await get().updateTab(updatedTab);
    set((state) => ({
      lastExecutionResult: {
        ...state.lastExecutionResult,
        [tabId]: hist as any,
      },
    }));
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
  importCurlCommand: async (curl: string) => {
    try {
      const clean = curl.replace(/\\\n/g, ' ').replace(/\\\r\n/g, ' ');
      let method = 'GET';
      let url = 'https://httpbin.org/get';
      const headers: HeaderItem[] = [];
      const params: ParamItem[] = [];
      let bodyType = 'none';
      let bodyContent = '';

      const methodMatch = clean.match(/(?:-X|--request)\s+([A-Za-z]+)/i);
      if (methodMatch) {
        method = methodMatch[1].toUpperCase();
      }

      const headerRegex = /(?:-H|--header)\s+(?:"([^"]+)"|'([^']+)'|(\S+))/g;
      let match;
      let hIdx = 0;
      while ((match = headerRegex.exec(clean)) !== null) {
        const hVal = match[1] || match[2] || match[3] || '';
        const colonIdx = hVal.indexOf(':');
        if (colonIdx > 0) {
          const key = hVal.substring(0, colonIdx).trim();
          const value = hVal.substring(colonIdx + 1).trim();
          headers.push({ id: `h-import-${Date.now()}-${hIdx++}`, key, value, enabled: true });
        }
      }

      const dataMatch = clean.match(/(?:-d|--data|--data-raw|--data-binary)\s+(?:"([\s\S]*?)"|'([\s\S]*?)'|(\S+))/);
      if (dataMatch) {
        bodyContent = dataMatch[1] ?? dataMatch[2] ?? dataMatch[3] ?? '';
        if (!methodMatch) method = 'POST';
        if (bodyContent.trim().startsWith('{') || bodyContent.trim().startsWith('[')) {
          bodyType = 'json';
        } else {
          bodyType = 'raw';
        }
      }

      const urlMatch = clean.match(/https?:\/\/[^\s"']+/i);
      if (urlMatch) {
        url = urlMatch[0];
      } else {
        const tokens = clean.split(/\s+/);
        for (let i = 1; i < tokens.length; i++) {
          const t = tokens[i].replace(/^["']|["']$/g, '');
          if (!t.startsWith('-') && (t.includes('.') || t.includes('localhost'))) {
            url = t.startsWith('http') ? t : `https://${t}`;
            break;
          }
        }
      }

      try {
        const urlObj = new URL(url);
        let pIdx = 0;
        urlObj.searchParams.forEach((val, k) => {
          params.push({ id: `p-import-${Date.now()}-${pIdx++}`, key: k, value: val, enabled: true });
        });
      } catch (e) {}

      const nowMs = Date.now();
      let createdTab: RepeaterTab | null = null;
      if (isTauriAvailable()) {
        try {
          createdTab = await createRepeaterTab();
        } catch (e) {
          console.warn('[RepeaterStore] createRepeaterTab IPC error in importCurlCommand:', e);
        }
      }
      if (!createdTab) {
        createdTab = {
          id: 'tab-' + nowMs,
          method,
          url,
          headers,
          params,
          bodyType,
          bodyContent,
          extractRules: [],
          orderIndex: get().tabs.length,
          createdAtMs: nowMs,
          updatedAtMs: nowMs,
          executionCount: 0,
        };
      }

      const populatedTab: RepeaterTab = {
        ...createdTab,
        method,
        url,
        headers,
        params,
        bodyType,
        bodyContent,
        updatedAtMs: nowMs,
      };

      if (isTauriAvailable()) {
        await updateRepeaterTab(populatedTab);
      }

      set((state) => ({
        tabs: [populatedTab, ...state.tabs.filter((t) => t.id !== populatedTab.id)],
        requests: [populatedTab, ...state.tabs.filter((t) => t.id !== populatedTab.id)],
        activeTabId: populatedTab.id,
        isCurlModalOpen: false,
      }));
    } catch (err) {
      console.error('Failed to import cURL command:', err);
    }
  },
}));
