import { StateCreator } from 'zustand';
import type { ProxyServerSlice, ProxyState } from './types';
import { setProxyMode as ipcSetProxyMode, isTauriAvailable } from '../../services/tauri/ipc';
import {
  getListenerConfigs,
  addListener as ipcAddListener,
  removeListener as ipcRemoveListener,
  updateListener as ipcUpdateListener,
  setListenerEnabled as ipcSetListenerEnabled,
} from '../../services/tauri/bridge';

export const createProxyServerSlice: StateCreator<
  ProxyState,
  [],
  [],
  ProxyServerSlice
> = (set, get) => ({
  proxyStatus: {
    mode: 'normal',
    activeCount: 1,
  },
  listeners: [],

  initProxyServer: async () => {
    if (isTauriAvailable()) await get().fetchListeners();
  },

  fetchListeners: async () => {
    if (isTauriAvailable()) {
      try {
        const configs = await getListenerConfigs();
        get().applyListeners(configs || []);
      } catch (e) {
        console.warn('Failed to fetch listener configs:', e);
      }
    }
  },

  applyListeners: (listeners) => set({ listeners }),

  addListener: async (label, address, replaceConflicts = false) => {
    if (isTauriAvailable()) {
      get().applyListeners(await ipcAddListener(label, address, replaceConflicts));
    } else {
      const { listeners } = get();
      const nextId = (listeners.length ? Math.max(...listeners.map((l) => l.id)) : 0) + 1;
      get().applyListeners([...listeners, { id: nextId, label, address, enabled: true }]);
    }
  },

  removeListener: async (id) => {
    if (isTauriAvailable()) {
      get().applyListeners(await ipcRemoveListener(id));
    } else {
      get().applyListeners(get().listeners.filter((l) => l.id !== id));
    }
  },

  updateListener: async (id, label, address, replaceConflicts = false) => {
    if (isTauriAvailable()) {
      get().applyListeners(await ipcUpdateListener(id, label, address, replaceConflicts));
    } else {
      get().applyListeners(
        get().listeners.map((l) =>
          l.id === id ? { ...l, label: label ?? l.label, address: address ?? l.address } : l
        )
      );
    }
  },

  setListenerEnabled: async (id, enabled) => {
    if (isTauriAvailable()) {
      get().applyListeners(await ipcSetListenerEnabled(id, enabled));
    } else {
      get().applyListeners(get().listeners.map((l) => (l.id === id ? { ...l, enabled } : l)));
    }
  },

  setProxyMode: async (mode) => {
    try {
      await ipcSetProxyMode(mode);
      set((state) => ({
        proxyStatus: { ...state.proxyStatus, mode },
      }));
    } catch (err) {
      console.error('Failed to set proxy mode:', err);
    }
  },
});

