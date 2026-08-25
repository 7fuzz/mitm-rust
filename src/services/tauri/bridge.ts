import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface HistorySummaryItem {
  id: number;
  uuid: string;
  method: string;
  url: string;
  host: string;
  statusCode: number;
  durationMs?: number;
  createdAt: string;
}

export interface HistoryDetailItem extends HistorySummaryItem {
  requestHeaders: [string, string][];
  responseHeaders: [string, string][];
  requestBody: string;
  responseBody: string;
  requestBodyHex?: string;
  responseBodyHex?: string;
  phase: string;
}

export interface TrafficCapturedEvent {
  entry: HistorySummaryItem;
}

export interface ProxyConfig {
  port: number;
  host: string;
  isRunning: boolean;
}

export const getHistoryLogs = async (
  page: number = 1,
  limit: number = 50,
  searchTerm?: string,
  methodFilter?: string,
  statusFilter?: number
): Promise<HistorySummaryItem[]> => {
  return await invoke<HistorySummaryItem[]>("get_history_logs", {
    page,
    limit,
    searchTerm: searchTerm || null,
    methodFilter: methodFilter || null,
    statusFilter: statusFilter || null,
  });
};

export const getHistoryDetail = async (id: number): Promise<HistoryDetailItem> => {
  return await invoke<HistoryDetailItem>("get_history_detail", { id });
};

export const clearHistoryLogs = async (): Promise<void> => {
  return await invoke<void>("clear_history_logs");
};

export const startProxy = async (port?: number, host?: string): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("start_proxy", { port: port || null, host: host || null });
};

export const stopProxy = async (): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("stop_proxy");
};

export const toggleProxy = async (): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("toggle_proxy");
};

export const getProxyStatus = async (): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("get_proxy_status");
};

export const subscribeTrafficCaptured = async (
  callback: (event: TrafficCapturedEvent) => void
): Promise<UnlistenFn> => {
  return await listen<TrafficCapturedEvent>("traffic_captured", (e) => {
    callback(e.payload);
  });
};
