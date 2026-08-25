import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface HistorySummaryItem {
  id: number;
  uuid: string;
  method: string;
  url: string;
  host: string;
  path: string;
  contentType: string;
  responseSize: number;
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
  proxyEnabled: boolean;
  interceptEnabled: boolean;
  interceptMode: "request" | "response" | "both";
  proxyMode: "on" | "off" | "block_client" | "block";
  port: number;
  host: string;
}

export interface InterceptRule {
  id: string;
  isEnabled: boolean;
  targetPhase: "request" | "response" | "both";
  matchField: "url" | "host" | "path" | "method" | "header";
  operator: "contains" | "equals" | "regex";
  matchValue: string;
  orderIndex: number;
  createdAtMs: number;
}

export interface PendingFlowPayload {
  flowId: string;
  phase: "request" | "response";
  method: string;
  url: string;
  headers: [string, string][];
  body: number[];
  bodyText: string;
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

export const getProxyState = async (): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("get_proxy_state");
};

export const setProxyMode = async (
  mode: "on" | "off" | "block_client" | "block"
): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("set_proxy_mode", { mode });
};

export const toggleProxy = async (enabled: boolean): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("toggle_proxy", { enabled });
};

export const toggleInterceptor = async (
  enabled: boolean,
  mode: "request" | "response" | "both"
): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("toggle_interceptor", { enabled, mode });
};

export const getInterceptRules = async (): Promise<InterceptRule[]> => {
  return await invoke<InterceptRule[]>("get_intercept_rules");
};

export const updateInterceptRules = async (
  rules: InterceptRule[]
): Promise<InterceptRule[]> => {
  return await invoke<InterceptRule[]>("update_intercept_rules", { rules });
};

export const forwardInterceptedFlow = async (
  flowId: string,
  modifiedHeadersJson?: string,
  modifiedBody?: number[]
): Promise<void> => {
  return await invoke<void>("forward_intercepted_flow", {
    flowId,
    modifiedHeadersJson: modifiedHeadersJson || null,
    modifiedBody: modifiedBody || null,
  });
};

export const dropInterceptedFlow = async (flowId: string): Promise<void> => {
  return await invoke<void>("drop_intercepted_flow", { flowId });
};

export const forwardAllInterceptedFlows = async (): Promise<void> => {
  return await invoke<void>("forward_all_intercepted_flows");
};

export const dropAllInterceptedFlows = async (): Promise<void> => {
  return await invoke<void>("drop_all_intercepted_flows");
};

export const getPendingFlows = async (): Promise<PendingFlowPayload[]> => {
  return await invoke<PendingFlowPayload[]>("get_pending_flows");
};

export const subscribeTrafficCaptured = async (
  callback: (event: TrafficCapturedEvent) => void
): Promise<UnlistenFn> => {
  return await listen<TrafficCapturedEvent>("traffic_captured", (e) => {
    callback(e.payload);
  });
};

export const subscribeInterceptTriggered = async (
  callback: (flow: PendingFlowPayload) => void
): Promise<UnlistenFn> => {
  return await listen<PendingFlowPayload>("intercept_triggered", (e) => {
    callback(e.payload);
  });
};
