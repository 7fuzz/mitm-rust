import { create } from 'zustand';
import type { RepeaterRequestItem, RepeaterGroup, RepeaterHistoryItem } from '../types';
import { executeRepeaterRequest, createRepeaterItem, updateRepeaterRequest, deleteRepeaterRequest, createRepeaterGroup, renameRepeaterGroup, deleteRepeaterGroup } from '../services/tauri/ipc';

const SAMPLE_GROUPS: RepeaterGroup[] = [
  { id: 'grp-auth', name: 'Authentication Suite', description: 'OAuth & Token Refresh Flow', environmentIds: ['env-dev', 'env-prod'] },
  { id: 'grp-payment', name: 'Stripe Payment Gateway', description: 'Charges, Refunds, and Webhook Testing', environmentIds: ['env-dev'] },
];

const SAMPLE_REQUESTS: RepeaterRequestItem[] = [
  {
    id: 'req-rep-1',
    name: 'Get User Profile',
    groupId: 'grp-auth',
    method: 'GET',
    url: '{{BASE_URL}}/api/v1/user/me',
    params: [{ id: 'p1', key: 'verbose', value: 'true', enabled: true }],
    headers: [
      { id: 'h1', key: 'Authorization', value: 'Bearer {{AUTH_TOKEN}}', enabled: true },
      { id: 'h2', key: 'Accept', value: 'application/json', enabled: true },
    ],
    bodyType: 'none',
    body: '',
    autoExtractRules: [
      { id: 'ae1', type: 'jsonpath', expression: '$.data.id', targetVariable: 'USER_ID', enabled: true },
    ],
  },
  {
    id: 'req-rep-2',
    name: 'Create Stripe Charge',
    groupId: 'grp-payment',
    method: 'POST',
    url: '{{STRIPE_HOST}}/v1/charges',
    params: [],
    headers: [
      { id: 'h3', key: 'Authorization', value: 'Bearer {{STRIPE_SECRET_KEY}}', enabled: true },
      { id: 'h4', key: 'Content-Type', value: 'application/json', enabled: true },
    ],
    bodyType: 'json',
    body: JSON.stringify({ amount: 5000, currency: 'usd', customer: 'cus_99218274' }, null, 2),
    autoExtractRules: [
      { id: 'ae2', type: 'jsonpath', expression: '$.id', targetVariable: 'LAST_CHARGE_ID', enabled: true },
    ],
  },
];

interface RepeaterState {
  groups: RepeaterGroup[];
  requests: RepeaterRequestItem[];
  openTabIds: string[];
  activeTabId: string | null;
  executionHistory: Record<string, RepeaterHistoryItem[]>; // requestId -> history items
  isHistoryDrawerOpen: boolean;
  isCurlModalOpen: boolean;
  lastExecutionResponse: Record<string, RepeaterHistoryItem | null>;
  isExecuting: Record<string, boolean>;

  setActiveTab: (id: string | null) => void;
  openTab: (request: RepeaterRequestItem) => void;
  closeTab: (id: string) => void;
  createNewRequest: (groupId?: string | null) => Promise<void>;
  updateRequest: (request: RepeaterRequestItem) => Promise<void>;
  deleteRequest: (id: string) => Promise<void>;
  
  createGroup: (name: string, description?: string) => Promise<void>;
  renameGroup: (id: string, name: string) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;

  executeActiveRequest: (requestId: string) => Promise<void>;
  toggleHistoryDrawer: (open?: boolean) => void;
  setCurlModalOpen: (open: boolean) => void;
  importCurlCommand: (curlString: string, name?: string) => Promise<void>;
}

export const useRepeaterStore = create<RepeaterState>((set, get) => ({
  groups: SAMPLE_GROUPS,
  requests: SAMPLE_REQUESTS,
  openTabIds: ['req-rep-1', 'req-rep-2'],
  activeTabId: 'req-rep-1',
  executionHistory: {
    'req-rep-1': [
      {
        id: 'hist-1',
        requestId: 'req-rep-1',
        timestamp: Date.now() - 60000,
        method: 'GET',
        url: 'https://api.internal.dev/api/v1/user/me',
        statusCode: 200,
        durationMs: 42,
        size: 320,
        requestHeaders: [{ key: 'Authorization', value: 'Bearer token_abc123' }],
        requestBody: '',
        responseHeaders: [{ key: 'Content-Type', value: 'application/json' }],
        responseBody: JSON.stringify({ id: 'usr_84920', username: 'alex', role: 'admin' }, null, 2),
      },
    ],
  },
  isHistoryDrawerOpen: false,
  isCurlModalOpen: false,
  lastExecutionResponse: {},
  isExecuting: {},

  setActiveTab: (id) => set({ activeTabId: id }),

  openTab: (request) => {
    const { openTabIds } = get();
    if (!openTabIds.includes(request.id)) {
      set({ openTabIds: [...openTabIds, request.id], activeTabId: request.id });
    } else {
      set({ activeTabId: request.id });
    }
  },

  closeTab: (id) => {
    const { openTabIds, activeTabId } = get();
    const nextTabs = openTabIds.filter((tId) => tId !== id);
    let nextActive = activeTabId;
    if (activeTabId === id) {
      nextActive = nextTabs.length > 0 ? nextTabs[nextTabs.length - 1] : null;
    }
    set({ openTabIds: nextTabs, activeTabId: nextActive });
  },

  createNewRequest: async (groupId = null) => {
    const newReq: Partial<RepeaterRequestItem> = {
      name: 'Untitled Request',
      method: 'GET',
      url: 'https://httpbin.org/get',
      params: [],
      headers: [{ id: 'h1', key: 'User-Agent', value: 'MITM-Developer-Studio', enabled: true }],
      bodyType: 'none',
      body: '',
      groupId,
      autoExtractRules: [],
    };
    try {
      const created = await createRepeaterItem(newReq);
      set((state) => ({
        requests: [...state.requests, created],
        openTabIds: [...state.openTabIds, created.id],
        activeTabId: created.id,
      }));
    } catch (err) {
      console.error('Failed to create repeater request:', err);
    }
  },

  updateRequest: async (updatedReq) => {
    set((state) => ({
      requests: state.requests.map((r) => (r.id === updatedReq.id ? updatedReq : r)),
    }));
    try {
      await updateRepeaterRequest(updatedReq);
    } catch (err) {
      console.error('Failed to update repeater request:', err);
    }
  },

  deleteRequest: async (id) => {
    get().closeTab(id);
    set((state) => ({
      requests: state.requests.filter((r) => r.id !== id),
    }));
    try {
      await deleteRepeaterRequest(id);
    } catch (err) {
      console.error('Failed to delete repeater request:', err);
    }
  },

  createGroup: async (name, description) => {
    try {
      const grp = await createRepeaterGroup(name, description);
      set((state) => ({ groups: [...state.groups, grp] }));
    } catch (err) {
      console.error('Failed to create collection group:', err);
    }
  },

  renameGroup: async (id, name) => {
    set((state) => ({
      groups: state.groups.map((g) => (g.id === id ? { ...g, name } : g)),
    }));
    try {
      await renameRepeaterGroup(id, name);
    } catch (err) {
      console.error('Failed to rename collection group:', err);
    }
  },

  deleteGroup: async (id) => {
    set((state) => ({
      groups: state.groups.filter((g) => g.id !== id),
      requests: state.requests.map((r) => (r.groupId === id ? { ...r, groupId: null } : r)),
    }));
    try {
      await deleteRepeaterGroup(id);
    } catch (err) {
      console.error('Failed to delete group:', err);
    }
  },

  executeActiveRequest: async (requestId) => {
    const request = get().requests.find((r) => r.id === requestId);
    if (!request) return;

    set((state) => ({
      isExecuting: { ...state.isExecuting, [requestId]: true },
    }));

    try {
      const result = await executeRepeaterRequest(request);
      set((state) => {
        const existingHist = state.executionHistory[requestId] || [];
        return {
          isExecuting: { ...state.isExecuting, [requestId]: false },
          lastExecutionResponse: { ...state.lastExecutionResponse, [requestId]: result },
          executionHistory: {
            ...state.executionHistory,
            [requestId]: [result, ...existingHist],
          },
        };
      });
    } catch (err) {
      console.error('Failed to execute request:', err);
      set((state) => ({
        isExecuting: { ...state.isExecuting, [requestId]: false },
      }));
    }
  },

  toggleHistoryDrawer: (open) => {
    set((state) => ({
      isHistoryDrawerOpen: open !== undefined ? open : !state.isHistoryDrawerOpen,
    }));
  },

  setCurlModalOpen: (isCurlModalOpen) => set({ isCurlModalOpen }),

  importCurlCommand: async (curlString, name = 'Imported cURL') => {
    // Parse cURL line
    let method = 'GET';
    if (curlString.includes('-X POST') || curlString.includes('--request POST')) method = 'POST';
    else if (curlString.includes('-X PUT') || curlString.includes('--request PUT')) method = 'PUT';
    else if (curlString.includes('-X DELETE') || curlString.includes('--request DELETE')) method = 'DELETE';

    const urlMatch = curlString.match(/https?:\/\/[^\s"']+/);
    const url = urlMatch ? urlMatch[0] : 'https://api.example.com/v1/test';

    const item: Partial<RepeaterRequestItem> = {
      name,
      method,
      url,
      params: [],
      headers: [{ id: 'h1', key: 'Content-Type', value: 'application/json', enabled: true }],
      bodyType: method === 'GET' ? 'none' : 'json',
      body: '{\n  "imported": true\n}',
    };

    const created = await createRepeaterItem(item);
    set((state) => ({
      requests: [...state.requests, created],
      openTabIds: [...state.openTabIds, created.id],
      activeTabId: created.id,
      isCurlModalOpen: false,
    }));
  },
}));
