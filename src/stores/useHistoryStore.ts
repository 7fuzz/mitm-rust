import { create } from "zustand";
import {
  HistorySummaryItem,
  HistoryDetailItem,
  ProxyConfig,
  getHistoryLogs,
  getHistoryDetail,
  clearHistoryLogs,
  getHistorySettings,
  updateHistorySettings,
  getProxyState,
  setProxyMode,
  subscribeTrafficCaptured,
} from "../services/tauri/bridge";
import { UnlistenFn } from "@tauri-apps/api/event";

interface HistoryState {
  logs: HistorySummaryItem[];
  detailMap: Record<string, HistoryDetailItem>;
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
  limiterEnabled: boolean;
  maxRows: number;
  settingsModalOpen: boolean;
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
  fetchHistorySettings: () => Promise<void>;
  setHistoryLimiter: (enabled: boolean, maxRows: number) => Promise<void>;
  setSettingsModalOpen: (open: boolean) => void;
  changeProxyMode: (mode: "on" | "off" | "block_client" | "block") => Promise<void>;
  initSubscription: () => Promise<void>;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  logs: [],
  detailMap: {},
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
  limiterEnabled: true,
  maxRows: 500,
  settingsModalOpen: false,
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

    const cachedDetail = get().detailMap[uuid];
    if (cachedDetail) {
      set({ selectedLogDetail: cachedDetail, loadingDetail: false });
      return;
    }

    try {
      const detail = await getHistoryDetail(uuid);
      set((state) => ({
        selectedLogDetail: detail,
        detailMap: { ...state.detailMap, [uuid]: detail },
        loadingDetail: false,
      }));
    } catch (e) {
      console.error("Failed to fetch history detail by uuid:", e);
      const targetSummary = get().logs.find((l) => l.uuid === uuid);
      if (targetSummary) {
        set({
          selectedLogDetail: {
            ...targetSummary,
            requestHeaders: [],
            responseHeaders: targetSummary.contentType ? [["Content-Type", targetSummary.contentType]] : [],
            requestBody: "",
            responseBody: "",
            phase: "response",
          },
          loadingDetail: false,
        });
      } else {
        set({ selectedLogDetail: null, loadingDetail: false });
      }
    }
  },

  clearLogs: async () => {
    try {
      await clearHistoryLogs();
      set({ logs: [], detailMap: {}, selectedLogUuid: null, selectedLogDetail: null });
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

  fetchHistorySettings: async () => {
    try {
      const settings = await getHistorySettings();
      set({
        limiterEnabled: settings.limiterEnabled,
        maxRows: settings.maxRows,
      });
    } catch (e) {
      console.error("Failed to fetch history settings:", e);
    }
  },

  setHistoryLimiter: async (enabled: boolean, maxRows: number) => {
    try {
      const updated = await updateHistorySettings(enabled, maxRows);
      set({
        limiterEnabled: updated.limiterEnabled,
        maxRows: updated.maxRows,
      });
      get().fetchLogs();
    } catch (e) {
      console.error("Failed to update history limiter:", e);
    }
  },

  setSettingsModalOpen: (open: boolean) => {
    set({ settingsModalOpen: open });
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
      set((state) => {
        const updatedLogs = [event.entry, ...state.logs];
        const updatedDetailMap = { ...state.detailMap, [event.entry.uuid]: event.detail };

        if (state.limiterEnabled && updatedLogs.length > state.maxRows) {
          return {
            logs: updatedLogs.slice(0, state.maxRows),
            detailMap: updatedDetailMap,
          };
        }
        return {
          logs: updatedLogs,
          detailMap: updatedDetailMap,
        };
      });
    });
    set({ unsubFn: unsub });
  },
}));
