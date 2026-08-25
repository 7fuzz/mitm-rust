import { create } from 'zustand';
import type { EnvironmentItem, VariableItem, ReplacementRule, CollectionLink } from '../types';
import {
  getWorkspaces,
  createWorkspace,
  updateWorkspace,
  deleteWorkspace,
  setActiveWorkspace,
  importWorkspaceJson,
  getWorkspaceEnvironments,
  saveWorkspaceEnvironment,
  Workspace,
  Environment,
  ImportSummary,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';

interface WorkspaceState {
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  environmentsList: Environment[];
  activeEnvironmentId: string;
  isImportModalOpen: boolean;

  // Legacy compatibility fields
  environments: EnvironmentItem[];
  variables: VariableItem[];
  replacements: ReplacementRule[];
  collectionLinks: CollectionLink[];

  // Actions
  initStore: () => Promise<void>;
  selectWorkspace: (id: string) => Promise<void>;
  createNewWorkspace: (name: string, description?: string) => Promise<Workspace | null>;
  updateWorkspaceDetails: (workspace: Workspace) => Promise<void>;
  deleteWorkspaceById: (id: string) => Promise<void>;
  importProjectJson: (jsonContent: string) => Promise<ImportSummary | null>;
  loadEnvironments: (workspaceId: string) => Promise<void>;
  saveEnvironmentVariables: (env: Environment) => Promise<void>;
  setImportModalOpen: (open: boolean) => void;

  // Legacy compatibility methods
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

const SAMPLE_WORKSPACES: Workspace[] = [
  {
    id: 'ws-default',
    name: 'Default Workspace',
    description: 'Main development and testing workspace',
    activeEnvironmentId: 'env-dev',
    createdAtMs: Date.now(),
    updatedAtMs: Date.now(),
  },
];

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  workspaces: SAMPLE_WORKSPACES,
  activeWorkspaceId: 'ws-default',
  environmentsList: [],
  activeEnvironmentId: 'env-dev',
  isImportModalOpen: false,

  environments: [
    { id: 'env-global', name: 'Global', isDefault: true, color: '#94a3b8' },
    { id: 'env-dev', name: 'Development', color: '#38bdf8' },
    { id: 'env-staging', name: 'Staging', color: '#f59e0b' },
    { id: 'env-prod', name: 'Production', color: '#ef4444' },
  ],
  variables: [],
  replacements: [],
  collectionLinks: [],

  initStore: async () => {
    if (isTauriAvailable()) {
      try {
        const fetchedWorkspaces = await getWorkspaces();
        if (fetchedWorkspaces.length > 0) {
          set({ workspaces: fetchedWorkspaces });
          const activeId = get().activeWorkspaceId || fetchedWorkspaces[0].id;
          set({ activeWorkspaceId: activeId });
          await get().loadEnvironments(activeId);
        } else {
          // Create default workspace if none exists
          const created = await createWorkspace('Default Workspace', 'Main development workspace');
          set({ workspaces: [created], activeWorkspaceId: created.id });
          await get().loadEnvironments(created.id);
        }
      } catch (err) {
        console.error('Failed to init workspace store:', err);
      }
    }
  },

  selectWorkspace: async (id) => {
    set({ activeWorkspaceId: id });
    if (isTauriAvailable()) {
      try {
        await setActiveWorkspace(id);
        await get().loadEnvironments(id);
      } catch (err) {
        console.error('Failed to set active workspace:', err);
      }
    }
  },

  createNewWorkspace: async (name, description) => {
    try {
      if (isTauriAvailable()) {
        const created = await createWorkspace(name, description);
        set((state) => ({
          workspaces: [created, ...state.workspaces],
          activeWorkspaceId: created.id,
        }));
        await get().loadEnvironments(created.id);
        return created;
      } else {
        const newWs: Workspace = {
          id: 'ws-' + Date.now(),
          name,
          description,
          createdAtMs: Date.now(),
          updatedAtMs: Date.now(),
        };
        set((state) => ({
          workspaces: [newWs, ...state.workspaces],
          activeWorkspaceId: newWs.id,
        }));
        return newWs;
      }
    } catch (err) {
      console.error('Failed to create workspace:', err);
      return null;
    }
  },

  updateWorkspaceDetails: async (workspace) => {
    set((state) => ({
      workspaces: state.workspaces.map((w) => (w.id === workspace.id ? workspace : w)),
    }));
    if (isTauriAvailable()) {
      try {
        await updateWorkspace(workspace);
      } catch (err) {
        console.error('Failed to update workspace:', err);
      }
    }
  },

  deleteWorkspaceById: async (id) => {
    const remaining = get().workspaces.filter((w) => w.id !== id);
    const nextActiveId = remaining[0]?.id || null;
    set({ workspaces: remaining, activeWorkspaceId: nextActiveId });

    if (isTauriAvailable()) {
      try {
        await deleteWorkspace(id);
        if (nextActiveId) {
          await get().loadEnvironments(nextActiveId);
        }
      } catch (err) {
        console.error('Failed to delete workspace:', err);
      }
    }
  },

  importProjectJson: async (jsonContent) => {
    if (isTauriAvailable()) {
      try {
        const summary = await importWorkspaceJson(jsonContent);
        await get().initStore();
        if (summary.workspaceId) {
          await get().selectWorkspace(summary.workspaceId);
        }
        return summary;
      } catch (err) {
        console.error('Failed to import workspace JSON:', err);
        throw err;
      }
    }
    return null;
  },

  loadEnvironments: async (workspaceId) => {
    if (isTauriAvailable() && workspaceId) {
      try {
        const envs = await getWorkspaceEnvironments(workspaceId);
        set({ environmentsList: envs });
        const activeEnv = envs.find((e) => e.isActive) || envs[0];
        if (activeEnv) {
          set({ activeEnvironmentId: activeEnv.id });
        }
      } catch (err) {
        console.error('Failed to load workspace environments:', err);
      }
    }
  },

  saveEnvironmentVariables: async (env) => {
    set((state) => ({
      environmentsList: state.environmentsList.map((e) => (e.id === env.id ? env : e)),
    }));
    if (isTauriAvailable()) {
      try {
        await saveWorkspaceEnvironment(env);
        if (get().activeWorkspaceId) {
          await get().loadEnvironments(get().activeWorkspaceId!);
        }
      } catch (err) {
        console.error('Failed to save environment variables:', err);
      }
    }
  },

  setImportModalOpen: (open) => set({ isImportModalOpen: open }),

  // Legacy compatibility methods
  setActiveEnv: async (id) => set({ activeEnvironmentId: id }),
  createEnv: async () => {},
  deleteEnv: async () => {},
  addVar: async () => {},
  updateVar: async () => {},
  deleteVar: async () => {},
  addReplacement: async () => {},
  toggleReplacement: async () => {},
  deleteReplacementRule: async () => {},
  toggleCollectionLink: () => {},
}));
