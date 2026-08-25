import { create } from "zustand";
import {
  HistorySummaryItem,
  HistoryDetailItem,
  ProxyConfig,
  getHistoryLogs,
  getHistoryDetail,
  clearHistoryLogs,
  getProxyState,
  setProxyMode,
  subscribeTrafficCaptured,
} from "../services/tauri/bridge";
import { UnlistenFn } from "@tauri-apps/api/event";

interface HistoryState {
  logs: HistorySummaryItem[];
  page: number;
  limit: number;
  searchTerm: string;
  methodFilter: string;
  statusFilter: number | null;
  selectedLogUuid: string | null;
  selectedLogDetail: HistoryDetailItem | null;
  loadingDetail: boolean;
  autoScroll: boolean;
  proxyConfig: ProxyConfig;
  isLoading: boolean;
  unsubFn: UnlistenFn | null;

  // Actions
  fetchLogs: () => Promise<void>;
  setSearchTerm: (term: string) => void;
  setMethodFilter: (method: string) => void;
  setStatusFilter: (status: number | null) => void;
  selectLog: (uuid: string) => Promise<void>;
  clearLogs: () => Promise<void>;
  toggleAutoScroll: () => void;
  fetchProxyStatus: () => Promise<void>;
  changeProxyMode: (mode: "on" | "off" | "block_client" | "block") => Promise<void>;
  initSubscription: () => Promise<void>;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  logs: [],
  page: 1,
  limit: 100,
  searchTerm: "",
  methodFilter: "ALL",
  statusFilter: null,
  selectedLogUuid: null,
  selectedLogDetail: null,
  loadingDetail: false,
  autoScroll: true,
  proxyConfig: {
    proxyEnabled: true,
    interceptEnabled: false,
    interceptMode: "both",
    proxyMode: "on",
    port: 8080,
    host: "127.0.0.1",
  },
  isLoading: false,
  unsubFn: null,

  fetchLogs: async () => {
    const { page, limit, searchTerm, methodFilter, statusFilter } = get();
    set({ isLoading: true });
    try {
      const logs = await getHistoryLogs(
        page,
        limit,
        searchTerm,
        methodFilter,
        statusFilter ?? undefined
      );
      set({ logs, isLoading: false });
    } catch (e) {
      console.error("Failed to fetch history logs:", e);
      set({ isLoading: false });
    }
  },

  setSearchTerm: (searchTerm: string) => {
    set({ searchTerm, page: 1 });
    get().fetchLogs();
  },

  setMethodFilter: (methodFilter: string) => {
    set({ methodFilter, page: 1 });
    get().fetchLogs();
  },

  setStatusFilter: (statusFilter: number | null) => {
    set({ statusFilter, page: 1 });
    get().fetchLogs();
  },

  selectLog: async (uuid: string) => {
    set({ selectedLogUuid: uuid, loadingDetail: true });
    const target = get().logs.find((l) => l.uuid === uuid);
    if (!target) {
      set({ selectedLogDetail: null, loadingDetail: false });
      return;
    }

    if (target.id > 0) {
      try {
        const detail = await getHistoryDetail(target.id);
        set({ selectedLogDetail: detail, loadingDetail: false });
      } catch (e) {
        console.error("Failed to fetch history detail by id:", e);
        set({
          selectedLogDetail: {
            ...target,
            requestHeaders: [],
            responseHeaders: target.contentType ? [["Content-Type", target.contentType]] : [],
            requestBody: "",
            responseBody: "",
            phase: "response",
          },
          loadingDetail: false,
        });
      }
    } else {
      set({
        selectedLogDetail: {
          ...target,
          requestHeaders: [],
          responseHeaders: target.contentType ? [["Content-Type", target.contentType]] : [],
          requestBody: "",
          responseBody: "",
          phase: "response",
        },
        loadingDetail: false,
      });
    }
  },

  clearLogs: async () => {
    try {
      await clearHistoryLogs();
      set({ logs: [], selectedLogUuid: null, selectedLogDetail: null });
    } catch (e) {
      console.error("Failed to clear logs:", e);
    }
  },

  toggleAutoScroll: () => {
    set((state) => ({ autoScroll: !state.autoScroll }));
  },

  fetchProxyStatus: async () => {
    try {
      const config = await getProxyState();
      set({ proxyConfig: config });
    } catch (e) {
      console.error("Failed to fetch proxy status:", e);
    }
  },

  changeProxyMode: async (mode: "on" | "off" | "block_client" | "block") => {
    try {
      const config = await setProxyMode(mode);
      set({ proxyConfig: config });
    } catch (e) {
      console.error("Failed to set proxy mode:", e);
    }
  },

  initSubscription: async () => {
    const currentUnsub = get().unsubFn;
    if (currentUnsub) {
      currentUnsub();
    }
    const unsub = await subscribeTrafficCaptured((event) => {
      set((state) => ({
        logs: [event.entry, ...state.logs],
      }));
    });
    set({ unsubFn: unsub });
  },
}));
