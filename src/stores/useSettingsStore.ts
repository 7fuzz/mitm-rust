import { create } from 'zustand';
import type { NavModule, AppPreferences } from '../types';
import { PINNED_ITEMS, isPinned, tabItem } from '../components/layout/navModules';
import { getRootCaPem, exportRootCa, regenerateRootCa, updatePrefs } from '../services/tauri/ipc';

/** Which environments the variable switcher edits: the active workspace's, or the Repeater's own. */
export type QuickVarScope = 'workspace' | 'repeater';

export const quickVarScopeFor = (module: NavModule): QuickVarScope | null => {
  if (module === 'repeater') return 'repeater';
  if (module === 'collections' || module === 'workspace') return 'workspace';
  return null;
};

interface SettingsState {
  activeModule: NavModule;
  openTabs: NavModule[];
  theme: 'dark' | 'light';
  isQuickVarModalOpen: boolean;
  quickVarScope: QuickVarScope;
  caPem: string;
  isCaLoading: boolean;

  setActiveModule: (module: NavModule) => void;
  closeTab: (module: NavModule) => void;
  moveTab: (module: NavModule, target: NavModule) => void;
  setTheme: (theme: 'dark' | 'light') => void;
  toggleTheme: () => void;
  setQuickVarModalOpen: (open: boolean) => void;
  openQuickVarModal: (scope: QuickVarScope) => void;
  fetchCaCert: () => Promise<void>;
  exportCaCert: (destinationPath: string) => Promise<void>;
  regenerateCaCert: () => Promise<void>;
  updatePreferences: (newPrefs: Partial<AppPreferences>) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  activeModule: 'http-history',
  openTabs: PINNED_ITEMS.map((item) => item.id),
  theme: 'dark',
  isQuickVarModalOpen: false,
  quickVarScope: 'workspace',
  caPem: '',
  isCaLoading: false,

  setActiveModule: (module) =>
    set(({ openTabs }) => ({
      activeModule: module,
      openTabs: openTabs.includes(module) || !tabItem(module) ? openTabs : [...openTabs, module],
    })),

  closeTab: (module) => {
    if (isPinned(module)) return;
    const { openTabs, activeModule } = get();
    const index = openTabs.indexOf(module);
    const remaining = openTabs.filter((id) => id !== module);
    set({
      openTabs: remaining,
      activeModule: activeModule === module ? remaining[Math.min(index, remaining.length - 1)] : activeModule,
    });
  },

  moveTab: (module, target) => {
    const tabs = get().openTabs.filter((id) => id !== module);
    const targetIndex = tabs.indexOf(target);
    const after = get().openTabs.indexOf(module) < get().openTabs.indexOf(target);
    tabs.splice(targetIndex + (after ? 1 : 0), 0, module);
    set({ openTabs: tabs });
  },
  
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
  openQuickVarModal: (quickVarScope) => set({ isQuickVarModalOpen: true, quickVarScope }),

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
