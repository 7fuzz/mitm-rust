import { StateCreator } from 'zustand';
import type { InterceptSlice, ProxyState } from './types';
import type { InterceptConfig, InterceptRule } from '../../types';
import {
  updateInterceptConfig,
  updateFilterConfig,
  resumeFlow,
} from '../../services/tauri/ipc';

export const createInterceptSlice: StateCreator<
  ProxyState,
  [],
  [],
  InterceptSlice
> = (set, get) => ({
  interceptConfig: {
    enabled: false,
    mode: 'all',
    direction: 'request',
    autoTimeoutSeconds: 30,
  },
  interceptRules: [
    { id: 'rule-1', target: 'domain', pattern: '*.stripe.internal', action: 'intercept', enabled: true },
    { id: 'rule-2', target: 'path', pattern: '/api/v1/auth/*', action: 'intercept', enabled: true },
  ],
  pendingQueue: [],

  toggleIntercept: async (enabled) => {
    const nextEnabled = enabled !== undefined ? enabled : !get().interceptConfig.enabled;
    const nextConfig: InterceptConfig = { ...get().interceptConfig, enabled: nextEnabled };
    try {
      await updateInterceptConfig(nextConfig);
      set({ interceptConfig: nextConfig });
    } catch (err) {
      console.error('Failed to toggle intercept config:', err);
    }
  },

  setInterceptDirection: async (direction) => {
    const nextConfig: InterceptConfig = { ...get().interceptConfig, direction };
    try {
      await updateInterceptConfig(nextConfig);
      set({ interceptConfig: nextConfig });
    } catch (err) {
      console.error('Failed to update intercept direction:', err);
    }
  },

  addInterceptRule: async (rule) => {
    const newRule: InterceptRule = { ...rule, id: 'rule-' + Date.now() };
    const nextRules = [...get().interceptRules, newRule];
    set({ interceptRules: nextRules });
    await updateFilterConfig({ rules: nextRules });
  },

  toggleInterceptRule: async (id) => {
    const nextRules = get().interceptRules.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r));
    set({ interceptRules: nextRules });
    await updateFilterConfig({ rules: nextRules });
  },

  deleteInterceptRule: async (id) => {
    const nextRules = get().interceptRules.filter((r) => r.id !== id);
    set({ interceptRules: nextRules });
    await updateFilterConfig({ rules: nextRules });
  },

  forwardPendingFlow: async (id, modifiedBody, modifiedHeaders) => {
    try {
      await resumeFlow(id, { action: 'forward', modifiedBody, modifiedHeaders });
      set((state) => ({
        pendingQueue: state.pendingQueue.filter((p) => p.id !== id),
      }));
    } catch (err) {
      console.error('Failed to forward pending flow:', err);
    }
  },

  dropPendingFlow: async (id) => {
    try {
      await resumeFlow(id, { action: 'drop' });
      set((state) => ({
        pendingQueue: state.pendingQueue.filter((p) => p.id !== id),
      }));
    } catch (err) {
      console.error('Failed to drop pending flow:', err);
    }
  },

  forwardAllPending: async () => {
    const pending = get().pendingQueue;
    for (const flow of pending) {
      try {
        await resumeFlow(flow.id, { action: 'forward' });
      } catch (err) {
        console.error('Failed to forward pending flow:', flow.id, err);
      }
    }
    set({ pendingQueue: [] });
  },
});
