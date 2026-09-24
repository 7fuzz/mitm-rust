import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface HistoryEntry {
  id: string;
  method: string;
  url: string;
  host: string;
  path: string;
  contentType: string;
  responseSize: number;
  statusCode: number;
  requestHeaders: [string, string][];
  responseHeaders: [string, string][];
  requestBody: string;
  responseBody: string;
  phase: string;
  durationMs?: number;
  createdAt: string;
  isIntercepted: boolean;
  isRewritten: boolean;
  isFailed: boolean;
}

export interface HistoryDetail {
  id: string;
  requestHeaders: [string, string][];
  responseHeaders: [string, string][];
  requestBody: string;
  responseBody: string;
}

export interface TrafficCapturedEvent {
  entry: HistoryEntry;
}

export interface HistorySettings {
  limiterEnabled: boolean;
  maxRows: number;
}

export interface HistoryFilterOptions {
  searchTerm?: string;
  methodFilter?: string;
  statusFilter?: number;
  statusRange?: string;
  onlyIntercepted?: boolean;
  onlyRewritten?: boolean;
  onlyFailed?: boolean;
  onlyWaiting?: boolean;
  includeBodies?: boolean;
}

export const getHistoryLogs = async (
  page: number = 1,
  limit: number = 50,
  options?: HistoryFilterOptions | string,
  methodFilter?: string,
  statusFilter?: number
): Promise<HistoryEntry[]> => {
  let opts: HistoryFilterOptions = {};
  if (typeof options === 'string' || methodFilter !== undefined || statusFilter !== undefined) {
    opts = {
      searchTerm: typeof options === 'string' ? options : undefined,
      methodFilter,
      statusFilter,
    };
  } else if (options) {
    opts = options;
  }

  return await invoke<HistoryEntry[]>("get_history_logs", {
    page,
    limit,
    searchTerm: opts.searchTerm || null,
    search_term: opts.searchTerm || null,
    methodFilter: opts.methodFilter || null,
    method_filter: opts.methodFilter || null,
    statusFilter: opts.statusFilter ?? null,
    status_filter: opts.statusFilter ?? null,
    statusRange: opts.statusRange || null,
    status_range: opts.statusRange || null,
    onlyIntercepted: opts.onlyIntercepted ?? null,
    only_intercepted: opts.onlyIntercepted ?? null,
    onlyRewritten: opts.onlyRewritten ?? null,
    only_rewritten: opts.onlyRewritten ?? null,
    onlyFailed: opts.onlyFailed ?? null,
    only_failed: opts.onlyFailed ?? null,
    onlyWaiting: opts.onlyWaiting ?? null,
    only_waiting: opts.onlyWaiting ?? null,
    includeBodies: opts.includeBodies ?? false,
    include_bodies: opts.includeBodies ?? false,
  });
};

export const getHistoryDetail = async (id: string): Promise<HistoryDetail | null> => {
  return await invoke<HistoryDetail | null>("get_history_detail", { id });
};

export const getHistoryCount = async (options?: HistoryFilterOptions): Promise<number> => {
  const opts = options || {};
  return await invoke<number>("get_history_count", {
    searchTerm: opts.searchTerm || null,
    search_term: opts.searchTerm || null,
    methodFilter: opts.methodFilter || null,
    method_filter: opts.methodFilter || null,
    statusFilter: opts.statusFilter ?? null,
    status_filter: opts.statusFilter ?? null,
    statusRange: opts.statusRange || null,
    status_range: opts.statusRange || null,
    onlyIntercepted: opts.onlyIntercepted ?? null,
    only_intercepted: opts.onlyIntercepted ?? null,
    onlyRewritten: opts.onlyRewritten ?? null,
    only_rewritten: opts.onlyRewritten ?? null,
    onlyFailed: opts.onlyFailed ?? null,
    only_failed: opts.onlyFailed ?? null,
    onlyWaiting: opts.onlyWaiting ?? null,
    only_waiting: opts.onlyWaiting ?? null,
  });
};

export const clearHistoryLogs = async (): Promise<void> => {
  return await invoke<void>("clear_history_logs");
};

export const getHistorySettings = async (): Promise<HistorySettings> => {
  return await invoke<HistorySettings>("get_history_settings");
};

export const updateHistorySettings = async (
  limiterEnabled: boolean,
  maxRows: number
): Promise<HistorySettings> => {
  return await invoke<HistorySettings>("update_history_settings", {
    limiterEnabled,
    limiter_enabled: limiterEnabled,
    enabled: limiterEnabled,
    maxRows,
    max_rows: maxRows,
  });
};

export const subscribeTrafficCaptured = async (
  callback: (event: TrafficCapturedEvent) => void
): Promise<UnlistenFn> => {
  return await listen<TrafficCapturedEvent>("traffic_captured", (e) => {
    callback(e.payload);
  });
};
