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
}

export interface TrafficCapturedEvent {
  entry: HistoryEntry;
}

export interface HistorySettings {
  limiterEnabled: boolean;
  maxRows: number;
}

export interface ProxyConfig {
  proxyEnabled: boolean;
  interceptEnabled: boolean;
  interceptMode: "request" | "response" | "both";
  proxyMode: "on" | "off" | "block_client" | "block";
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
  action?: "intercept" | "pass";
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

export interface HeaderItem {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export interface ParamItem {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export interface ExtractRuleItem {
  id: string;
  type: string;
  expression: string;
  targetVariable: string;
  enabled: boolean;
}

export interface RepeaterTab {
  id: string;
  name: string;
  method: string;
  url: string;
  headers: HeaderItem[];
  params: ParamItem[];
  bodyType: string;
  bodyContent?: string;
  extractRules: ExtractRuleItem[];
  orderIndex: number;
  createdAtMs: number;
  updatedAtMs: number;
  executionCount?: number;
  lastStatusCode?: number;
  lastDurationMs?: number;
}

export interface RepeaterHistoryItem {
  id: number;
  repeaterId: string;
  method: string;
  url: string;
  requestHeaders: HeaderItem[];
  requestBody?: string;
  statusCode: number;
  responseHeaders: HeaderItem[];
  responseBody?: string;
  durationMs: number;
  executedAtMs: number;
}

export interface RepeaterExecutionResult {
  historyId: number;
  repeaterId: string;
  statusCode: number;
  statusText: string;
  responseHeaders: HeaderItem[];
  responseBody: string;
  durationMs: number;
  responseSize: number;
}

export interface MigrationProgressPayload {
  step: number;
  total: number;
  name: string;
  status: string;
  isComplete: boolean;
  hasError: boolean;
  errorMessage?: string;
}

export const subscribeDbMigrationProgress = async (
  callback: (event: MigrationProgressPayload) => void
): Promise<UnlistenFn> => {
  return await listen<MigrationProgressPayload>("db-migration-progress", (e) => {
    callback(e.payload);
  });
};

export const runDatabaseMigrations = async (): Promise<void> => {
  return await invoke<void>("run_database_migrations");
};

export const backupAndResetDatabase = async (): Promise<string> => {
  return await invoke<string>("backup_and_reset_database");
};

export const exportDatabaseFile = async (destinationPath: string): Promise<void> => {
  return await invoke<void>("export_database_file", { destinationPath });
};

export const quitApplication = async (): Promise<void> => {
  return await invoke<void>("quit_application");
};

export const getHistoryLogs = async (
  page: number = 1,
  limit: number = 50,
  searchTerm?: string,
  methodFilter?: string,
  statusFilter?: number
): Promise<HistoryEntry[]> => {
  return await invoke<HistoryEntry[]>("get_history_logs", {
    page,
    limit,
    searchTerm: searchTerm || null,
    methodFilter: methodFilter || null,
    statusFilter: statusFilter || null,
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
    maxRows,
  });
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

export const getRepeaterTabs = async (): Promise<RepeaterTab[]> => {
  return await invoke<RepeaterTab[]>("get_repeater_tabs");
};

export const createRepeaterTab = async (
  name?: string,
  fromHistoryId?: string
): Promise<RepeaterTab> => {
  return await invoke<RepeaterTab>("create_repeater_tab", {
    name: name || null,
    fromHistoryId: fromHistoryId || null,
  });
};

export const updateRepeaterTab = async (
  tab: RepeaterTab
): Promise<void> => {
  return await invoke<void>("update_repeater_tab", { tab });
};

export const deleteRepeaterTab = async (id: string): Promise<void> => {
  return await invoke<void>("delete_repeater_tab", { id });
};

export const executeRepeaterRequest = async (
  id: string
): Promise<RepeaterExecutionResult> => {
  return await invoke<RepeaterExecutionResult>("execute_repeater_request", { id });
};

export const getRepeaterHistory = async (
  repeaterId: string,
  page: number = 1,
  limit: number = 50
): Promise<RepeaterHistoryItem[]> => {
  return await invoke<RepeaterHistoryItem[]>("get_repeater_history", {
    repeaterId,
    page,
    limit,
  });
};

export const insertRepeaterHistory = async (
  history: RepeaterHistoryItem
): Promise<number> => {
  return await invoke<number>("insert_repeater_history", { history });
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
