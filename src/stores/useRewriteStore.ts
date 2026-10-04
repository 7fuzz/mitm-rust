import { create } from "zustand";
import {
  RewriteRule,
  RewriteHistoryEntry,
  getRewriteRules,
  saveRewriteRules,
  toggleRewriteEnabled,
  getRewriteEnabled,
  getRewriteSourceScope,
  setRewriteSourceScope,
  getRewriteHistory,
  clearRewriteHistory,
  subscribeRewriteCaptured,
} from "../services/tauri/bridge";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { ALL_SOURCES, type SourceScope } from "../types";

interface RewriteState {
  rules: RewriteRule[];
  logs: RewriteHistoryEntry[];
  isRewriteEnabled: boolean;
  sourceScope: SourceScope;
  selectedLogId: number | null;
  searchQuery: string;
  isRuleModalOpen: boolean;
  editingRule: RewriteRule | null;
  unsubFn: UnlistenFn | null;

  // Actions
  initStore: () => Promise<void>;
  fetchStatus: () => Promise<void>;
  toggleEnabled: () => Promise<void>;
  setSourceScope: (scope: SourceScope) => Promise<void>;
  fetchRules: () => Promise<void>;
  addOrUpdateRule: (rule: Omit<RewriteRule, "id" | "createdAtMs" | "orderIndex"> & { id?: string }) => Promise<void>;
  toggleRule: (id: string) => Promise<void>;
  deleteRule: (id: string) => Promise<void>;
  duplicateRule: (id: string) => Promise<void>;
  moveRule: (index: number, direction: "up" | "down") => Promise<void>;
  fetchLogs: () => Promise<void>;
  clearLogs: () => Promise<void>;
  selectLog: (id: number | null) => void;
  setSearchQuery: (query: string) => void;
  openRuleModal: (rule?: RewriteRule) => void;
  closeRuleModal: () => void;
}

export const useRewriteStore = create<RewriteState>((set, get) => ({
  rules: [],
  logs: [],
  isRewriteEnabled: true,
  sourceScope: ALL_SOURCES,
  selectedLogId: null,
  searchQuery: "",
  isRuleModalOpen: false,
  editingRule: null,
  unsubFn: null,

  fetchStatus: async () => {
    try {
      const [isRewriteEnabled, rules] = await Promise.all([getRewriteEnabled(), getRewriteRules()]);
      set({ isRewriteEnabled, rules });
    } catch (e) {
      console.error("Failed to fetch rewrite status:", e);
    }
  },

  initStore: async () => {
    try {
      const enabled = await getRewriteEnabled();
      const sourceScope = await getRewriteSourceScope();
      const rules = await getRewriteRules();
      const logs = await getRewriteHistory(200);

      set({
        isRewriteEnabled: enabled,
        sourceScope,
        rules,
        logs,
        selectedLogId: logs.length > 0 ? logs[0].id : null,
      });

      const currentUnsub = get().unsubFn;
      if (currentUnsub) {
        currentUnsub();
      }

      const unsub = await subscribeRewriteCaptured((event) => {
        set((state) => {
          const updatedLogs = [event.entry, ...state.logs.slice(0, 300)];
          return {
            logs: updatedLogs,
            selectedLogId: state.selectedLogId || event.entry.id,
          };
        });
      });

      set({ unsubFn: unsub });
    } catch (e) {
      console.error("Failed to initialize rewrite store:", e);
    }
  },

  toggleEnabled: async () => {
    const next = !get().isRewriteEnabled;
    try {
      await toggleRewriteEnabled(next);
      set({ isRewriteEnabled: next });
    } catch (e) {
      console.error("Failed to toggle rewrite enabled:", e);
    }
  },

  setSourceScope: async (scope: SourceScope) => {
    const previous = get().sourceScope;
    set({ sourceScope: scope });
    try {
      set({ sourceScope: await setRewriteSourceScope(scope) });
    } catch (e) {
      console.error("Failed to set rewrite sources:", e);
      set({ sourceScope: previous });
    }
  },

  fetchRules: async () => {
    try {
      const rules = await getRewriteRules();
      set({ rules });
    } catch (e) {
      console.error("Failed to fetch rewrite rules:", e);
    }
  },

  addOrUpdateRule: async (ruleData) => {
    const currentRules = [...get().rules];
    if (ruleData.id) {
      // Update existing
      const updated = currentRules.map((r) =>
        r.id === ruleData.id ? ({ ...r, ...ruleData } as RewriteRule) : r
      );
      const saved = await saveRewriteRules(updated);
      set({ rules: saved, isRuleModalOpen: false, editingRule: null });
    } else {
      // Create new
      const newRule: RewriteRule = {
        id: `rw-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        name: ruleData.name || "Untitled Rule",
        enabled: ruleData.enabled ?? true,
        actionType: ruleData.actionType,
        matchField: ruleData.matchField || "all",
        matchOperator: ruleData.matchOperator || "contains",
        matchValue: ruleData.matchValue || "",
        targetPart: ruleData.targetPart || "url",
        targetHeader: ruleData.targetHeader,
        matchPattern: ruleData.matchPattern || "",
        replacementValue: ruleData.replacementValue || "",
        isRegex: ruleData.isRegex || false,
        mockStatusCode: ruleData.mockStatusCode,
        mockHeadersJson: ruleData.mockHeadersJson,
        mockBody: ruleData.mockBody,
        orderIndex: currentRules.length,
        createdAtMs: Date.now(),
      };
      const updated = [...currentRules, newRule];
      const saved = await saveRewriteRules(updated);
      set({ rules: saved, isRuleModalOpen: false, editingRule: null });
    }
  },

  toggleRule: async (id: string) => {
    const currentRules = get().rules.map((r) =>
      r.id === id ? { ...r, enabled: !r.enabled } : r
    );
    try {
      const saved = await saveRewriteRules(currentRules);
      set({ rules: saved });
    } catch (e) {
      console.error("Failed to toggle rule:", e);
    }
  },

  deleteRule: async (id: string) => {
    const currentRules = get().rules.filter((r) => r.id !== id);
    try {
      const saved = await saveRewriteRules(currentRules);
      set({ rules: saved });
    } catch (e) {
      console.error("Failed to delete rule:", e);
    }
  },

  duplicateRule: async (id: string) => {
    const target = get().rules.find((r) => r.id === id);
    if (!target) return;
    const duplicated: RewriteRule = {
      ...target,
      id: `rw-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      name: `${target.name} (Copy)`,
      orderIndex: get().rules.length,
      createdAtMs: Date.now(),
    };
    const updated = [...get().rules, duplicated];
    try {
      const saved = await saveRewriteRules(updated);
      set({ rules: saved });
    } catch (e) {
      console.error("Failed to duplicate rule:", e);
    }
  },

  moveRule: async (index: number, direction: "up" | "down") => {
    const rules = [...get().rules];
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= rules.length) return;

    const [removed] = rules.splice(index, 1);
    rules.splice(targetIndex, 0, removed);

    // Update orderIndex
    const reordered = rules.map((r, idx) => ({ ...r, orderIndex: idx }));
    try {
      const saved = await saveRewriteRules(reordered);
      set({ rules: saved });
    } catch (e) {
      console.error("Failed to reorder rules:", e);
    }
  },

  fetchLogs: async () => {
    try {
      const logs = await getRewriteHistory(200);
      set({ logs });
    } catch (e) {
      console.error("Failed to fetch rewrite logs:", e);
    }
  },

  clearLogs: async () => {
    try {
      await clearRewriteHistory();
      set({ logs: [], selectedLogId: null });
    } catch (e) {
      console.error("Failed to clear rewrite logs:", e);
    }
  },

  selectLog: (id: number | null) => {
    set({ selectedLogId: id });
  },

  setSearchQuery: (query: string) => {
    set({ searchQuery: query });
  },

  openRuleModal: (rule?: RewriteRule) => {
    set({ isRuleModalOpen: true, editingRule: rule || null });
  },

  closeRuleModal: () => {
    set({ isRuleModalOpen: false, editingRule: null });
  },
}));
