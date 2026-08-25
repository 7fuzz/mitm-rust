import { create } from 'zustand';
import type { EnvironmentItem, VariableItem, ReplacementRule, CollectionLink } from '../types';
import { createEnvironment, deleteEnvironment, setActiveEnvironment, createVariable, updateVariable, deleteVariable, saveReplacementsBulk, deleteReplacement } from '../services/tauri/ipc';

const SAMPLE_ENVIRONMENTS: EnvironmentItem[] = [
  { id: 'env-global', name: 'Global', isDefault: true, color: '#94a3b8' },
  { id: 'env-dev', name: 'Development', color: '#38bdf8' },
  { id: 'env-staging', name: 'Staging', color: '#f59e0b' },
  { id: 'env-prod', name: 'Production', color: '#ef4444' },
];

const SAMPLE_VARIABLES: VariableItem[] = [
  { id: 'var-1', key: 'BASE_URL', value: 'https://api-dev.internal.net', environmentId: 'env-dev', isSecret: false, type: 'string', description: 'Dev server base endpoint' },
  { id: 'var-2', key: 'AUTH_TOKEN', value: 'bearer_token_super_secret_992182', environmentId: 'env-dev', isSecret: true, type: 'string', description: 'JWT authentication bearer' },
  { id: 'var-3', key: 'BASE_URL', value: 'https://api.production.com', environmentId: 'env-prod', isSecret: false, type: 'string', description: 'Production server base endpoint' },
  { id: 'var-4', key: 'STRIPE_SECRET_KEY', value: 'sk_live_51M000000000000000', environmentId: 'env-prod', isSecret: true, type: 'string', description: 'Stripe live key' },
  { id: 'var-5', key: 'GLOBAL_TIMEOUT', value: '5000', environmentId: 'env-global', isSecret: false, type: 'number', description: 'Default API timeout ms' },
];

const SAMPLE_REPLACEMENTS: ReplacementRule[] = [
  { id: 'rep-1', domain: '*.internal.dev', target: 'header', isRegex: false, pattern: 'User-Agent', replacement: 'MITM-Custom-Security-Agent/1.0', enabled: true },
  { id: 'rep-2', domain: 'api.github.com', target: 'body', isRegex: true, pattern: '\"is_admin\":\\s*false', replacement: '"is_admin": true', enabled: false },
];

const SAMPLE_COLLECTION_LINKS: CollectionLink[] = [
  { groupId: 'grp-auth', environmentId: 'env-dev' },
  { groupId: 'grp-auth', environmentId: 'env-staging' },
  { groupId: 'grp-payment', environmentId: 'env-dev' },
];

interface WorkspaceState {
  environments: EnvironmentItem[];
  activeEnvironmentId: string;
  variables: VariableItem[];
  replacements: ReplacementRule[];
  collectionLinks: CollectionLink[];

  setActiveEnv: (id: string) => Promise<void>;
  createEnv: (name: string, color?: string) => Promise<void>;
  deleteEnv: (id: string) => Promise<void>;

  addVar: (v: Partial<VariableItem>) => Promise<void>;
  updateVar: (v: VariableItem) => Promise<void>;
  deleteVar: (id: string) => Promise<void>;

  addReplacement: (rule: Omit<ReplacementRule, 'id'>) => Promise<void>;
  toggleReplacement: (id: string) => Promise<void>;
  deleteReplacementRule: (id: string) => Promise<void>;

  toggleCollectionLink: (groupId: string, environmentId: string) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  environments: SAMPLE_ENVIRONMENTS,
  activeEnvironmentId: 'env-dev',
  variables: SAMPLE_VARIABLES,
  replacements: SAMPLE_REPLACEMENTS,
  collectionLinks: SAMPLE_COLLECTION_LINKS,

  setActiveEnv: async (id) => {
    set({ activeEnvironmentId: id });
    try {
      await setActiveEnvironment(id);
    } catch (err) {
      console.error('Failed to set active environment:', err);
    }
  },

  createEnv: async (name, color) => {
    try {
      const newEnv = await createEnvironment(name, color);
      set((state) => ({ environments: [...state.environments, newEnv] }));
    } catch (err) {
      console.error('Failed to create environment:', err);
    }
  },

  deleteEnv: async (id) => {
    if (id === 'env-global') return; // Cannot delete global
    set((state) => ({
      environments: state.environments.filter((e) => e.id !== id),
      variables: state.variables.filter((v) => v.environmentId !== id),
      activeEnvironmentId: state.activeEnvironmentId === id ? 'env-global' : state.activeEnvironmentId,
    }));
    try {
      await deleteEnvironment(id);
    } catch (err) {
      console.error('Failed to delete environment:', err);
    }
  },

  addVar: async (v) => {
    try {
      const created = await createVariable(v);
      set((state) => ({ variables: [...state.variables, created] }));
    } catch (err) {
      console.error('Failed to add variable:', err);
    }
  },

  updateVar: async (v) => {
    set((state) => ({
      variables: state.variables.map((item) => (item.id === v.id ? v : item)),
    }));
    try {
      await updateVariable(v);
    } catch (err) {
      console.error('Failed to update variable:', err);
    }
  },

  deleteVar: async (id) => {
    set((state) => ({
      variables: state.variables.filter((item) => item.id !== id),
    }));
    try {
      await deleteVariable(id);
    } catch (err) {
      console.error('Failed to delete variable:', err);
    }
  },

  addReplacement: async (rule) => {
    const newRule: ReplacementRule = { ...rule, id: 'rep-' + Date.now() };
    const nextRules = [...get().replacements, newRule];
    set({ replacements: nextRules });
    try {
      await saveReplacementsBulk(nextRules);
    } catch (err) {
      console.error('Failed to save replacements:', err);
    }
  },

  toggleReplacement: async (id) => {
    const nextRules = get().replacements.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r));
    set({ replacements: nextRules });
    try {
      await saveReplacementsBulk(nextRules);
    } catch (err) {
      console.error('Failed to save replacements:', err);
    }
  },

  deleteReplacementRule: async (id) => {
    const nextRules = get().replacements.filter((r) => r.id !== id);
    set({ replacements: nextRules });
    try {
      await deleteReplacement(id);
    } catch (err) {
      console.error('Failed to delete replacement:', err);
    }
  },

  toggleCollectionLink: (groupId, environmentId) => {
    const { collectionLinks } = get();
    const exists = collectionLinks.some((l) => l.groupId === groupId && l.environmentId === environmentId);
    if (exists) {
      set({
        collectionLinks: collectionLinks.filter((l) => !(l.groupId === groupId && l.environmentId === environmentId)),
      });
    } else {
      set({ collectionLinks: [...collectionLinks, { groupId, environmentId }] });
    }
  },
}));
