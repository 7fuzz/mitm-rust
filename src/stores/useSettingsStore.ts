import { create } from 'zustand';
import type { NavModule, AppPreferences } from '../types';
import { getRootCaPem, regenerateRootCa, updatePrefs } from '../services/tauri/ipc';

interface SettingsState {
  activeModule: NavModule;
  theme: 'dark' | 'light';
  layoutMode: 'horizontal' | 'vertical';
  fontSize: 'sm' | 'md' | 'lg';
  isQuickVarModalOpen: boolean;
  dbSizeMb: string;
  isSqliteConnected: boolean;
  caPem: string;
  isCaLoading: boolean;

  setActiveModule: (module: NavModule) => void;
  setTheme: (theme: 'dark' | 'light') => void;
  toggleTheme: () => void;
  setLayoutMode: (mode: 'horizontal' | 'vertical') => void;
  setFontSize: (size: 'sm' | 'md' | 'lg') => void;
  setQuickVarModalOpen: (open: boolean) => void;
  fetchCaCert: () => Promise<void>;
  regenerateCaCert: () => Promise<void>;
  updatePreferences: (newPrefs: Partial<AppPreferences>) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  activeModule: 'http-history',
  theme: 'dark',
  layoutMode: 'vertical',
  fontSize: 'sm',
  isQuickVarModalOpen: false,
  dbSizeMb: '4.2 MB',
  isSqliteConnected: true,
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

  setLayoutMode: (layoutMode) => set({ layoutMode }),
  setFontSize: (fontSize) => set({ fontSize }),
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
      if (newPrefs.layoutMode) set({ layoutMode: newPrefs.layoutMode });
      if (newPrefs.fontSize) set({ fontSize: newPrefs.fontSize });
    } catch (err) {
      console.error('Failed to update preferences:', err);
    }
  },
}));
