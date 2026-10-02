import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ProxyConfig } from "./proxyBridge";
import type { SourceScope } from "../../../types";

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

export const toggleInterceptor = async (
  enabled: boolean,
  mode: "request" | "response" | "both"
): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("toggle_interceptor", { enabled, mode });
};

export const setInterceptSourceScope = async (scope: SourceScope): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("set_intercept_source_scope", { scope });
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
  modifiedUrl?: string,
  modifiedMethod?: string,
  modifiedHeadersJson?: string,
  modifiedBody?: number[]
): Promise<void> => {
  return await invoke<void>("forward_intercepted_flow", {
    flowId,
    modifiedUrl: modifiedUrl || null,
    modifiedMethod: modifiedMethod || null,
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

export const subscribeInterceptTriggered = async (
  callback: (flow: PendingFlowPayload) => void
): Promise<UnlistenFn> => {
  return await listen<PendingFlowPayload>("intercept_triggered", (e) => {
    callback(e.payload);
  });
};

export const focusAppWindow = async (): Promise<void> => {
  return await invoke<void>("focus_app_window");
};

export const setFocusPreference = async (enabled: boolean): Promise<void> => {
  return await invoke<void>("set_focus_preference", { enabled });
};

export const getFocusPreference = async (): Promise<boolean> => {
  return await invoke<boolean>("get_focus_preference");
};
