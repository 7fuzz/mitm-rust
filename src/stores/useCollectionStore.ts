import { create } from 'zustand';
import {
  getCollections,
  createCollection,
  updateCollection,
  deleteCollection,
  moveCollection,
  duplicateCollection,
  createRequest,
  updateRequest,
  deleteRequest,
  moveRequest,
  duplicateRequest,
  executeCollectionRequest,
  getRequestHistories,
  clearRequestHistories,
  Collection,
  CollectionTreeItem,
  RequestItem,
  ExecutionResult,
  RequestHistoryItem,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';

interface CollectionState {
  collectionsTree: CollectionTreeItem[];
  openRequests: RequestItem[];
  activeRequestId: string | null;
  isExecuting: Record<string, boolean>;
  executionResult: Record<string, ExecutionResult | null>;
  /** Recorded runs per request, newest first */
  runHistory: Record<string, RequestHistoryItem[]>;
  /** Run shown in the response pane per request; absent means the latest */
  selectedRunId: Record<string, number | undefined>;
  isHistoryDrawerOpen: boolean;
  searchQuery: string;

  // Actions
  fetchCollections: (workspaceId: string) => Promise<void>;
  createNewCollection: (workspaceId: string, parentId: string | null, name: string) => Promise<Collection | null>;
  updateCollectionDetails: (collection: Collection) => Promise<void>;
  deleteCollectionById: (id: string, workspaceId: string) => Promise<void>;
  moveCollectionItem: (collectionId: string, targetParentId: string | null, workspaceId: string) => Promise<void>;
  duplicateCollectionItem: (collectionId: string, workspaceId: string) => Promise<void>;
  createNewRequest: (collectionId: string, name: string) => Promise<RequestItem | null>;
  updateRequestDetails: (request: RequestItem) => Promise<void>;
  deleteRequestById: (id: string, workspaceId: string) => Promise<void>;
  moveRequestItem: (requestId: string, targetCollectionId: string, workspaceId: string) => Promise<void>;
  duplicateRequestItem: (requestId: string, workspaceId: string) => Promise<void>;
  openRequestTab: (request: RequestItem) => void;
  closeRequestTab: (id: string) => void;
  setActiveRequestId: (id: string | null) => void;
  executeRequest: (requestId: string) => Promise<void>;
  fetchRunHistory: (requestId: string) => Promise<void>;
  selectRun: (requestId: string, runId: number | undefined) => void;
  clearRunHistory: (requestId: string) => Promise<void>;
  toggleHistoryDrawer: (open?: boolean) => void;
  setSearchQuery: (query: string) => void;
}

export const useCollectionStore = create<CollectionState>((set, get) => ({
  collectionsTree: [],
  openRequests: [],
  activeRequestId: null,
  isExecuting: {},
  executionResult: {},
  runHistory: {},
  selectedRunId: {},
  isHistoryDrawerOpen: false,
  searchQuery: '',

  fetchCollections: async (workspaceId: string) => {
    if (!workspaceId) return;
    if (isTauriAvailable()) {
      try {
        const tree = await getCollections(workspaceId);
        set({ collectionsTree: tree });
      } catch (err) {
        console.error('Failed to fetch collections tree:', err);
      }
    }
  },

  createNewCollection: async (workspaceId, parentId, name) => {
    try {
      if (isTauriAvailable()) {
        const created = await createCollection(workspaceId, parentId, name);
        await get().fetchCollections(workspaceId);
        return created;
      } else {
        const newCol: Collection = {
          id: 'col-' + Date.now(),
          workspaceId,
          parentId: parentId || undefined,
          name,
          orderIndex: 0,
          createdAtMs: Date.now(),
          updatedAtMs: Date.now(),
        };
        return newCol;
      }
    } catch (err) {
      console.error('Failed to create collection:', err);
      return null;
    }
  },

  updateCollectionDetails: async (collection) => {
    if (isTauriAvailable()) {
      try {
        await updateCollection(collection);
        await get().fetchCollections(collection.workspaceId);
      } catch (err) {
        console.error('Failed to update collection:', err);
      }
    }
  },

  deleteCollectionById: async (id, workspaceId) => {
    if (isTauriAvailable()) {
      try {
        await deleteCollection(id);
        if (workspaceId) await get().fetchCollections(workspaceId);
      } catch (err) {
        console.error('Failed to delete collection:', err);
      }
    }
  },

  moveCollectionItem: async (collectionId, targetParentId, workspaceId) => {
    if (isTauriAvailable()) {
      try {
        await moveCollection(collectionId, targetParentId);
        if (workspaceId) await get().fetchCollections(workspaceId);
      } catch (err) {
        console.error('Failed to move collection:', err);
      }
    }
  },

  duplicateCollectionItem: async (collectionId, workspaceId) => {
    if (isTauriAvailable()) {
      try {
        await duplicateCollection(collectionId);
        if (workspaceId) await get().fetchCollections(workspaceId);
      } catch (err) {
        console.error('Failed to duplicate collection:', err);
      }
    }
  },

  createNewRequest: async (collectionId, name) => {
    try {
      if (isTauriAvailable()) {
        const created = await createRequest(collectionId, name);
        get().openRequestTab(created);
        return created;
      } else {
        const newReq: RequestItem = {
          id: 'req-' + Date.now(),
          collectionId,
          name,
          method: 'GET',
          url: 'https://httpbin.org/get',
          headers: [],
          params: [],
          bodyType: 'none',
          extractRules: [],
          orderIndex: 0,
          createdAtMs: Date.now(),
          updatedAtMs: Date.now(),
        };
        get().openRequestTab(newReq);
        return newReq;
      }
    } catch (err) {
      console.error('Failed to create collection request:', err);
      return null;
    }
  },

  updateRequestDetails: async (request) => {
    const patchTree = (nodes: CollectionTreeItem[]): CollectionTreeItem[] =>
      nodes.map((col) => ({
        ...col,
        requests: col.requests.map((r) => (r.id === request.id ? request : r)),
        children: patchTree(col.children),
      }));
    set((state) => ({
      openRequests: state.openRequests.map((r) => (r.id === request.id ? request : r)),
      collectionsTree: patchTree(state.collectionsTree),
    }));
    if (isTauriAvailable()) {
      try {
        await updateRequest(request);
      } catch (err) {
        console.error('Failed to update request:', err);
      }
    }
  },

  deleteRequestById: async (id, workspaceId) => {
    get().closeRequestTab(id);
    if (isTauriAvailable()) {
      try {
        await deleteRequest(id);
        if (workspaceId) await get().fetchCollections(workspaceId);
      } catch (err) {
        console.error('Failed to delete request:', err);
      }
    }
  },

  moveRequestItem: async (requestId, targetCollectionId, workspaceId) => {
    if (isTauriAvailable()) {
      try {
        await moveRequest(requestId, targetCollectionId);
        if (workspaceId) await get().fetchCollections(workspaceId);
      } catch (err) {
        console.error('Failed to move request:', err);
      }
    }
  },

  duplicateRequestItem: async (requestId, workspaceId) => {
    if (isTauriAvailable()) {
      try {
        const dup = await duplicateRequest(requestId);
        get().openRequestTab(dup);
        if (workspaceId) await get().fetchCollections(workspaceId);
      } catch (err) {
        console.error('Failed to duplicate request:', err);
      }
    }
  },

  openRequestTab: (request) => {
    const { openRequests } = get();
    const exists = openRequests.some((r) => r.id === request.id);
    if (!exists) {
      set({ openRequests: [...openRequests, request], activeRequestId: request.id });
    } else {
      set({
        openRequests: openRequests.map((r) => (r.id === request.id ? request : r)),
        activeRequestId: request.id,
      });
    }
  },

  closeRequestTab: (id) => {
    const remaining = get().openRequests.filter((r) => r.id !== id);
    let nextActiveId = get().activeRequestId;
    if (get().activeRequestId === id) {
      nextActiveId = remaining[0]?.id || null;
    }
    set({ openRequests: remaining, activeRequestId: nextActiveId });
  },

  setActiveRequestId: (id) => set({ activeRequestId: id }),

  executeRequest: async (requestId) => {
    if (!requestId) return;
    set((state) => ({
      isExecuting: { ...state.isExecuting, [requestId]: true },
    }));

    try {
      if (isTauriAvailable()) {
        const result = await executeCollectionRequest(requestId);
        set((state) => ({
          executionResult: { ...state.executionResult, [requestId]: result },
          isExecuting: { ...state.isExecuting, [requestId]: false },
          selectedRunId: { ...state.selectedRunId, [requestId]: undefined },
        }));
        await get().fetchRunHistory(requestId);
        try {
          const { activeWorkspaceId, loadEnvironments } = (await import('./useWorkspaceStore')).useWorkspaceStore.getState();
          if (activeWorkspaceId) {
            await loadEnvironments(activeWorkspaceId);
          }
        } catch {}
      } else {
        await new Promise((r) => setTimeout(r, 400));
        const mockResult: ExecutionResult = {
          historyId: Date.now(),
          requestId,
          statusCode: 200,
          statusText: 'OK',
          responseHeaders: [
            { id: '1', key: 'Content-Type', value: 'application/json', enabled: true },
          ],
          responseBody: JSON.stringify({ message: 'Collection request execution successful', requestId }, null, 2),
          durationMs: 38,
          responseSize: 160,
        };
        set((state) => ({
          executionResult: { ...state.executionResult, [requestId]: mockResult },
          isExecuting: { ...state.isExecuting, [requestId]: false },
        }));
      }
    } catch (err) {
      console.error('Failed to execute collection request:', err);
      set((state) => ({
        isExecuting: { ...state.isExecuting, [requestId]: false },
      }));
    }
  },

  fetchRunHistory: async (requestId) => {
    if (!requestId || !isTauriAvailable()) return;
    try {
      const runs = await getRequestHistories(requestId);
      set((state) => ({ runHistory: { ...state.runHistory, [requestId]: runs } }));
    } catch (err) {
      console.error('Failed to load request histories:', err);
    }
  },

  selectRun: (requestId, runId) =>
    set((state) => ({ selectedRunId: { ...state.selectedRunId, [requestId]: runId } })),

  clearRunHistory: async (requestId) => {
    if (isTauriAvailable()) {
      try {
        await clearRequestHistories(requestId);
      } catch (err) {
        console.error('Failed to clear request histories:', err);
        return;
      }
    }
    set((state) => ({
      runHistory: { ...state.runHistory, [requestId]: [] },
      selectedRunId: { ...state.selectedRunId, [requestId]: undefined },
      executionResult: { ...state.executionResult, [requestId]: null },
    }));
  },

  toggleHistoryDrawer: (open) => set((state) => ({ isHistoryDrawerOpen: open ?? !state.isHistoryDrawerOpen })),

  setSearchQuery: (query) => set({ searchQuery: query }),
}));
