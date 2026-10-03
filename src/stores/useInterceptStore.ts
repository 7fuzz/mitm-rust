import { create } from "zustand";
import {
  InterceptRule,
  PendingFlowPayload,
  getInterceptRules,
  updateInterceptRules,
  toggleInterceptor,
  forwardInterceptedFlow,
  dropInterceptedFlow,
  forwardAllInterceptedFlows,
  dropAllInterceptedFlows,
  getPendingFlows,
  subscribeInterceptTriggered,
  getProxyState,
  setInterceptSourceScope,
  isTauriAvailable,
  focusAppWindow,
  setFocusPreference,
  getFocusPreference,
} from "../services/tauri/bridge";
import { UnlistenFn } from "@tauri-apps/api/event";
import { useSettingsStore } from "./useSettingsStore";
import { ALL_SOURCES, type SourceScope } from "../types";
import { tryPrettifyJson } from '../utils/prettifyJson';

export interface InterceptParam {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export const parseParamsFromUrl = (urlStr: string): InterceptParam[] => {
  if (!urlStr) return [];
  try {
    const urlObj = new URL(urlStr);
    const result: InterceptParam[] = [];
    urlObj.searchParams.forEach((val, key) => {
      result.push({
        id: `param-${Math.random().toString(36).substring(2, 9)}`,
        key,
        value: val,
        enabled: true,
      });
    });
    return result;
  } catch {
    const qIdx = urlStr.indexOf("?");
    if (qIdx === -1) return [];
    const query = urlStr.substring(qIdx + 1);
    const searchParams = new URLSearchParams(query);
    const result: InterceptParam[] = [];
    searchParams.forEach((val, key) => {
      result.push({
        id: `param-${Math.random().toString(36).substring(2, 9)}`,
        key,
        value: val,
        enabled: true,
      });
    });
    return result;
  }
};

export const rebuildUrlWithParams = (baseUrlOrFullUrl: string, params: InterceptParam[]): string => {
  try {
    const urlObj = new URL(baseUrlOrFullUrl);
    const newSearchParams = new URLSearchParams();
    params.forEach((p) => {
      if (p.enabled && p.key.trim()) {
        newSearchParams.append(p.key.trim(), p.value);
      }
    });
    const searchStr = newSearchParams.toString();
    urlObj.search = searchStr ? `?${searchStr}` : "";
    return urlObj.toString();
  } catch {
    const qIdx = baseUrlOrFullUrl.indexOf("?");
    const base = qIdx !== -1 ? baseUrlOrFullUrl.substring(0, qIdx) : baseUrlOrFullUrl;
    const activeParams = params
      .filter((p) => p.enabled && p.key.trim())
      .map((p) => `${encodeURIComponent(p.key.trim())}=${encodeURIComponent(p.value)}`)
      .join("&");
    return activeParams ? `${base}?${activeParams}` : base;
  }
};

interface InterceptState {
  interceptEnabled: boolean;
  interceptMode: "request" | "response" | "both";
  interceptSourceScope: SourceScope;
  focusOnIntercepted: boolean;
  rules: InterceptRule[];
  pendingFlows: PendingFlowPayload[];
  selectedFlowId: string | null;
  editedMethod: string;
  editedUrl: string;
  editedParams: InterceptParam[];
  editedHeaders: [string, string][];
  editedBodyText: string;
  unsubFn: UnlistenFn | null;

  // Actions
  initInterceptStore: () => Promise<void>;
  setInterceptEnabled: (enabled: boolean) => Promise<void>;
  setInterceptMode: (mode: "request" | "response" | "both") => Promise<void>;
  setInterceptSourceScope: (scope: SourceScope) => Promise<void>;
  setFocusOnIntercepted: (val: boolean) => void;
  setRules: (rules: InterceptRule[]) => Promise<void>;
  addRule: (rule: Omit<InterceptRule, "id" | "createdAtMs">) => Promise<void>;
  updateRule: (rule: InterceptRule) => Promise<void>;
  toggleRule: (id: string) => Promise<void>;
  updateRuleAction: (id: string, action: "intercept" | "pass") => Promise<void>;
  deleteRule: (id: string) => Promise<void>;
  selectFlow: (flowId: string) => void;
  setEditedMethod: (method: string) => void;
  setEditedUrl: (url: string) => void;
  setEditedParams: (params: InterceptParam[]) => void;
  setEditedHeaders: (headers: [string, string][]) => void;
  setEditedBodyText: (bodyText: string) => void;
  forwardCurrentFlow: () => Promise<void>;
  dropCurrentFlow: () => Promise<void>;
  forwardAllFlows: () => Promise<void>;
  dropAllFlows: () => Promise<void>;
}

export const useInterceptStore = create<InterceptState>((set, get) => ({
  interceptEnabled: false,
  interceptMode: "request",
  interceptSourceScope: ALL_SOURCES,
  focusOnIntercepted: typeof window !== "undefined" ? localStorage.getItem("mitm_focus_on_intercepted") !== "false" : true,
  rules: [],
  pendingFlows: [],
  selectedFlowId: null,
  editedMethod: "GET",
  editedUrl: "",
  editedParams: [],
  editedHeaders: [],
  editedBodyText: "",
  unsubFn: null,

  initInterceptStore: async () => {
    try {
      const cfg = await getProxyState();
      const rules = await getInterceptRules();
      const pending = await getPendingFlows();

      const formattedPending = pending.map((p) => ({
        ...p,
        bodyText: tryPrettifyJson(p.bodyText),
      }));

      const selectedId = formattedPending.length > 0 ? formattedPending[0].flowId : null;
      const initialMethod = formattedPending.length > 0 ? formattedPending[0].method : "GET";
      const initialUrl = formattedPending.length > 0 ? formattedPending[0].url : "";
      const initialParams = formattedPending.length > 0 ? parseParamsFromUrl(formattedPending[0].url) : [];
      const initialHeaders = formattedPending.length > 0 ? formattedPending[0].headers : [];
      const initialBody = formattedPending.length > 0 ? formattedPending[0].bodyText : "";

      let initialFocus = true;
      if (isTauriAvailable()) {
        try {
          initialFocus = await getFocusPreference();
        } catch {
          initialFocus = typeof window !== "undefined" ? localStorage.getItem("mitm_focus_on_intercepted") !== "false" : true;
        }
      } else {
        initialFocus = typeof window !== "undefined" ? localStorage.getItem("mitm_focus_on_intercepted") !== "false" : true;
      }

      set({
        interceptEnabled: cfg.interceptEnabled,
        interceptMode: cfg.interceptMode,
        interceptSourceScope: cfg.interceptSourceScope ?? ALL_SOURCES,
        focusOnIntercepted: initialFocus,
        rules,
        pendingFlows: formattedPending,
        selectedFlowId: selectedId,
        editedMethod: initialMethod,
        editedUrl: initialUrl,
        editedParams: initialParams,
        editedHeaders: initialHeaders,
        editedBodyText: initialBody,
      });

      const currentUnsub = get().unsubFn;
      if (currentUnsub) {
        currentUnsub();
      }

      const unsub = await subscribeInterceptTriggered(async (rawFlow) => {
        const flow = {
          ...rawFlow,
          bodyText: tryPrettifyJson(rawFlow.bodyText),
        };
        const state = get();
        const exists = state.pendingFlows.some((f) => f.flowId === flow.flowId);
        if (exists) return;

        const updatedFlows = [...state.pendingFlows, flow];
        const focusEnabled = state.focusOnIntercepted;

        if (focusEnabled) {
          // Bring Tauri window to front & set focus via dedicated native command
          if (isTauriAvailable()) {
            try {
              await focusAppWindow();
            } catch (err) {
              console.warn("Failed to focus window on intercepted traffic:", err);
            }
          }

          // Switch active module to 'intercept'
          useSettingsStore.getState().setActiveModule("intercept");

          // Auto-select newly intercepted flow
          set({
            pendingFlows: updatedFlows,
            selectedFlowId: flow.flowId,
            editedMethod: flow.method || "GET",
            editedUrl: flow.url || "",
            editedParams: parseParamsFromUrl(flow.url || ""),
            editedHeaders: flow.headers || [],
            editedBodyText: flow.bodyText || "",
          });
        } else {
          const shouldSelect = !state.selectedFlowId;
          set({
            pendingFlows: updatedFlows,
            selectedFlowId: shouldSelect ? flow.flowId : state.selectedFlowId,
            editedMethod: shouldSelect ? flow.method || "GET" : state.editedMethod,
            editedUrl: shouldSelect ? flow.url || "" : state.editedUrl,
            editedParams: shouldSelect ? parseParamsFromUrl(flow.url || "") : state.editedParams,
            editedHeaders: shouldSelect ? flow.headers : state.editedHeaders,
            editedBodyText: shouldSelect ? flow.bodyText : state.editedBodyText,
          });
        }
      });

      set({ unsubFn: unsub });
    } catch (e) {
      console.error("Failed to initialize intercept store:", e);
    }
  },

  setFocusOnIntercepted: async (val: boolean) => {
    if (typeof window !== "undefined") {
      localStorage.setItem("mitm_focus_on_intercepted", String(val));
    }
    set({ focusOnIntercepted: val });
    if (isTauriAvailable()) {
      try {
        await setFocusPreference(val);
      } catch (err) {
        console.warn("Failed to sync focus preference to backend:", err);
      }
    }
  },

  setInterceptEnabled: async (enabled: boolean) => {
    try {
      const mode = get().interceptMode;
      const cfg = await toggleInterceptor(enabled, mode);
      set({ interceptEnabled: cfg.interceptEnabled });
    } catch (e) {
      console.error("Failed to toggle interceptor:", e);
    }
  },

  setInterceptMode: async (mode: "request" | "response" | "both") => {
    try {
      const enabled = get().interceptEnabled;
      const cfg = await toggleInterceptor(enabled, mode);
      set({ interceptMode: cfg.interceptMode });
    } catch (e) {
      console.error("Failed to set intercept mode:", e);
    }
  },

  setInterceptSourceScope: async (scope: SourceScope) => {
    const previous = get().interceptSourceScope;
    set({ interceptSourceScope: scope });
    try {
      const cfg = await setInterceptSourceScope(scope);
      set({ interceptSourceScope: cfg.interceptSourceScope ?? ALL_SOURCES });
    } catch (e) {
      console.error("Failed to set intercept sources:", e);
      set({ interceptSourceScope: previous });
    }
  },

  setRules: async (rules: InterceptRule[]) => {
    try {
      const updated = await updateInterceptRules(rules);
      set({ rules: updated });
    } catch (e) {
      console.error("Failed to set rules:", e);
    }
  },

  addRule: async (newRuleData) => {
    const rules = get().rules;
    const rule: InterceptRule = {
      action: newRuleData.action || "intercept",
      ...newRuleData,
      id: crypto.randomUUID(),
      createdAtMs: Date.now(),
      orderIndex: rules.length,
    };
    await get().setRules([...rules, rule]);
  },

  updateRule: async (updatedRule: InterceptRule) => {
    const rules = get().rules.map((r) =>
      r.id === updatedRule.id ? updatedRule : r
    );
    await get().setRules(rules);
  },

  toggleRule: async (id: string) => {
    const rules = get().rules.map((r) =>
      r.id === id ? { ...r, isEnabled: !r.isEnabled } : r
    );
    await get().setRules(rules);
  },

  updateRuleAction: async (id: string, action: "intercept" | "pass") => {
    const rules = get().rules.map((r) =>
      r.id === id ? { ...r, action } : r
    );
    await get().setRules(rules);
  },

  deleteRule: async (id: string) => {
    const rules = get().rules.filter((r) => r.id !== id);
    await get().setRules(rules);
  },

  selectFlow: (flowId: string) => {
    const flow = get().pendingFlows.find((f) => f.flowId === flowId);
    if (flow) {
      set({
        selectedFlowId: flowId,
        editedMethod: flow.method || "GET",
        editedUrl: flow.url || "",
        editedParams: parseParamsFromUrl(flow.url || ""),
        editedHeaders: [...flow.headers],
        editedBodyText: flow.bodyText,
      });
    }
  },

  setEditedMethod: (method: string) => {
    set({ editedMethod: method });
  },

  setEditedUrl: (url: string) => {
    set({
      editedUrl: url,
      editedParams: parseParamsFromUrl(url),
    });
  },

  setEditedParams: (params: InterceptParam[]) => {
    const currentUrl = get().editedUrl;
    const newUrl = rebuildUrlWithParams(currentUrl, params);
    set({
      editedParams: params,
      editedUrl: newUrl,
    });
  },

  setEditedHeaders: (headers: [string, string][]) => {
    set({ editedHeaders: headers });
  },

  setEditedBodyText: (bodyText: string) => {
    set({ editedBodyText: bodyText });
  },

  forwardCurrentFlow: async () => {
    const { selectedFlowId, editedUrl, editedMethod, editedHeaders, editedBodyText, pendingFlows } = get();
    if (!selectedFlowId) return;

    try {
      const originalFlow = pendingFlows.find((f) => f.flowId === selectedFlowId);

      const isUrlModified = originalFlow ? editedUrl !== originalFlow.url : false;
      const isMethodModified = originalFlow ? editedMethod !== (originalFlow.method || "GET") : false;
      const isHeadersModified = originalFlow
        ? JSON.stringify(editedHeaders) !== JSON.stringify(originalFlow.headers)
        : false;
      const isBodyModified = originalFlow ? editedBodyText !== (originalFlow.bodyText || "") : false;

      const modifiedUrl = isUrlModified ? editedUrl : undefined;
      const modifiedMethod = isMethodModified ? editedMethod : undefined;
      const headersJson = isHeadersModified ? JSON.stringify(editedHeaders) : undefined;

      let bodyBytes: number[] | undefined = undefined;
      if (isBodyModified) {
        const encoder = new TextEncoder();
        bodyBytes = Array.from(encoder.encode(editedBodyText));
      }

      await forwardInterceptedFlow(selectedFlowId, modifiedUrl, modifiedMethod, headersJson, bodyBytes);

      const remaining = pendingFlows.filter((f) => f.flowId !== selectedFlowId);
      const nextFlow = remaining.length > 0 ? remaining[0] : null;

      set({
        pendingFlows: remaining,
        selectedFlowId: nextFlow ? nextFlow.flowId : null,
        editedMethod: nextFlow ? nextFlow.method || "GET" : "GET",
        editedUrl: nextFlow ? nextFlow.url || "" : "",
        editedParams: nextFlow ? parseParamsFromUrl(nextFlow.url || "") : [],
        editedHeaders: nextFlow ? nextFlow.headers : [],
        editedBodyText: nextFlow ? nextFlow.bodyText : "",
      });
    } catch (e) {
      console.error("Failed to forward flow:", e);
    }
  },

  dropCurrentFlow: async () => {
    const { selectedFlowId, pendingFlows } = get();
    if (!selectedFlowId) return;

    try {
      await dropInterceptedFlow(selectedFlowId);

      const remaining = pendingFlows.filter((f) => f.flowId !== selectedFlowId);
      const nextFlow = remaining.length > 0 ? remaining[0] : null;

      set({
        pendingFlows: remaining,
        selectedFlowId: nextFlow ? nextFlow.flowId : null,
        editedMethod: nextFlow ? nextFlow.method || "GET" : "GET",
        editedUrl: nextFlow ? nextFlow.url || "" : "",
        editedParams: nextFlow ? parseParamsFromUrl(nextFlow.url || "") : [],
        editedHeaders: nextFlow ? nextFlow.headers : [],
        editedBodyText: nextFlow ? nextFlow.bodyText : "",
      });
    } catch (e) {
      console.error("Failed to drop flow:", e);
    }
  },

  forwardAllFlows: async () => {
    try {
      await forwardAllInterceptedFlows();
      set({
        pendingFlows: [],
        selectedFlowId: null,
        editedHeaders: [],
        editedBodyText: "",
      });
    } catch (e) {
      console.error("Failed to forward all flows:", e);
    }
  },

  dropAllFlows: async () => {
    try {
      await dropAllInterceptedFlows();
      set({
        pendingFlows: [],
        selectedFlowId: null,
        editedHeaders: [],
        editedBodyText: "",
      });
    } catch (e) {
      console.error("Failed to drop all flows:", e);
    }
  },
}));
