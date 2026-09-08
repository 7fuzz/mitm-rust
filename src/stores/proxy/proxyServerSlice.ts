import { StateCreator } from 'zustand';
import type { ProxyServerSlice, ProxyState } from './types';
import { setProxyMode as ipcSetProxyMode, isTauriAvailable } from '../../services/tauri/ipc';
import { getProxyState, updateNetworkSettings } from '../../services/tauri/bridge';

export const createProxyServerSlice: StateCreator<
  ProxyState,
  [],
  [],
  ProxyServerSlice
> = (set) => ({
  proxyStatus: {
    mode: 'normal',
    bindings: ['0.0.0.0:8080'],
    activeCount: 1,
  },

  initProxyServer: async () => {
    if (isTauriAvailable()) {
      try {
        const proxyConfig = await getProxyState();
        if (proxyConfig) {
          set((state) => ({
            proxyStatus: {
              ...state.proxyStatus,
              bindings: [`${proxyConfig.host || '0.0.0.0'}:${proxyConfig.port || 8080}`],
            },
          }));
        }
      } catch (e) {
        console.warn('Failed to fetch proxy state via IPC:', e);
      }
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

  updateProxyBindings: async (bindings: string[]) => {
    if (!bindings || bindings.length === 0) return;
    set((state) => ({
      proxyStatus: { ...state.proxyStatus, bindings },
    }));

    if (isTauriAvailable()) {
      try {
        const updated = await updateNetworkSettings(bindings);
        if (updated) {
          set((state) => ({
            proxyStatus: {
              ...state.proxyStatus,
              bindings: [`${updated.host}:${updated.port}`],
            },
          }));
        }
      } catch (err) {
        console.error('Failed to update network settings via IPC:', err);
        throw err;
      }
    }
  },
});
