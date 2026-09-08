import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface RewriteRule {
  id: string;
  name: string;
  enabled: boolean;
  actionType: "partial_request" | "full_request" | "partial_response" | "full_response";
  matchField: "all" | "url" | "host" | "path" | "method" | "header";
  matchOperator: "contains" | "equals" | "regex" | "starts_with";
  matchValue: string;
  targetPart: "url" | "query" | "header" | "body" | "method" | "status";
  targetHeader?: string;
  matchPattern: string;
  replacementValue: string;
  isRegex: boolean;
  mockStatusCode?: number;
  mockHeadersJson?: string;
  mockBody?: string;
  orderIndex: number;
  createdAtMs: number;
}

export interface RewriteHistoryEntry {
  id: string;
  ruleId?: string;
  ruleName: string;
  actionType: string;
  method: string;
  originalUrl: string;
  rewrittenUrl: string;
  originalHeaders: [string, string][];
  rewrittenHeaders: [string, string][];
  originalBody: string;
  rewrittenBody: string;
  statusCode?: number;
  durationMs?: number;
  createdAt: string;
}

export interface RewriteCapturedEvent {
  entry: RewriteHistoryEntry;
}

export const getRewriteRules = async (): Promise<RewriteRule[]> => {
  return await invoke<RewriteRule[]>("get_rewrite_rules");
};

export const saveRewriteRules = async (rules: RewriteRule[]): Promise<RewriteRule[]> => {
  return await invoke<RewriteRule[]>("save_rewrite_rules", { rules });
};

export const toggleRewriteEnabled = async (enabled: boolean): Promise<boolean> => {
  return await invoke<boolean>("toggle_rewrite_enabled", { enabled });
};

export const getRewriteEnabled = async (): Promise<boolean> => {
  return await invoke<boolean>("get_rewrite_enabled");
};

export const getRewriteHistory = async (limit?: number): Promise<RewriteHistoryEntry[]> => {
  return await invoke<RewriteHistoryEntry[]>("get_rewrite_history", { limit: limit || null });
};

export const clearRewriteHistory = async (): Promise<void> => {
  return await invoke<void>("clear_rewrite_history");
};

export const subscribeRewriteCaptured = async (
  callback: (event: RewriteCapturedEvent) => void
): Promise<UnlistenFn> => {
  return await listen<RewriteCapturedEvent>("rewrite_captured", (e) => callback(e.payload));
};
