import { invoke } from "@tauri-apps/api/core";

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
  method: string;
  url: string;
  headers: HeaderItem[];
  params: ParamItem[];
  bodyType: string;
  bodyContent?: string;
  extractRules: ExtractRuleItem[];
  preRequestId?: string | null;
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
