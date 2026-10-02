import { create } from 'zustand';
import type { NavModule, AppPreferences } from '../types';
import { getRootCaPem, exportRootCa, regenerateRootCa, updatePrefs } from '../services/tauri/ipc';

interface SettingsState {
  activeModule: NavModule;
  theme: 'dark' | 'light';
  isQuickVarModalOpen: boolean;
  caPem: string;
  isCaLoading: boolean;

  setActiveModule: (module: NavModule) => void;
  setTheme: (theme: 'dark' | 'light') => void;
  toggleTheme: () => void;
  setQuickVarModalOpen: (open: boolean) => void;
  fetchCaCert: () => Promise<void>;
  exportCaCert: (destinationPath: string) => Promise<void>;
  regenerateCaCert: () => Promise<void>;
  updatePreferences: (newPrefs: Partial<AppPreferences>) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  activeModule: 'http-history',
  theme: 'dark',
  isQuickVarModalOpen: false,
  caPem: '',
  isCaLoading: false,

  setActiveModule: (module) => set({ activeModule: module }),
  
  setTheme: (theme) => {
    set({ theme });
    if (typeof document !== 'undefined') {
      if (theme === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    }
  },

  toggleTheme: () => {
    const nextTheme = get().theme === 'dark' ? 'light' : 'dark';
    get().setTheme(nextTheme);
  },

  setQuickVarModalOpen: (isQuickVarModalOpen) => set({ isQuickVarModalOpen }),

  fetchCaCert: async () => {
    set({ isCaLoading: true });
    try {
      const pem = await getRootCaPem();
      set({ caPem: pem as string, isCaLoading: false });
    } catch (err) {
      console.error('Failed to fetch Root CA:', err);
      set({ isCaLoading: false });
    }
  },

  exportCaCert: async (destinationPath: string) => {
    try {
      await exportRootCa(destinationPath);
    } catch (err) {
      console.error('Failed to export Root CA:', err);
      throw err;
    }
  },

  regenerateCaCert: async () => {
    set({ isCaLoading: true });
    try {
      const pem = await regenerateRootCa();
      set({ caPem: pem, isCaLoading: false });
    } catch (err) {
      console.error('Failed to regenerate Root CA:', err);
      set({ isCaLoading: false });
    }
  },

  updatePreferences: async (newPrefs) => {
    try {
      await updatePrefs(newPrefs);
      if (newPrefs.theme) get().setTheme(newPrefs.theme);
    } catch (err) {
      console.error('Failed to update preferences:', err);
    }
  },
}));
