import { create } from 'zustand';
import type { EnvironmentItem, VariableItem, ReplacementRule, CollectionLink } from '../types';
import { useCollectionStore } from './useCollectionStore';
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
  importTargetWorkspaceId: string | null;

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
  importProjectJson: (jsonContent: string, targetWorkspaceId?: string, customWorkspaceName?: string) => Promise<ImportSummary | null>;
  loadEnvironments: (workspaceId: string) => Promise<void>;
  saveEnvironmentVariables: (env: Environment) => Promise<void>;
  setImportModalOpen: (open: boolean) => void;
  openImportModalForWorkspace: (workspaceId?: string) => void;

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
  importTargetWorkspaceId: null,

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
          const savedActiveWsId = localStorage.getItem('mitm_active_workspace_id');
          const activeWs = fetchedWorkspaces.find((w) => w.id === savedActiveWsId) || fetchedWorkspaces[0];
          set({ activeWorkspaceId: activeWs.id });
          await get().loadEnvironments(activeWs.id);
        } else {
          // Create default workspace if none exists
          const created = await createWorkspace('Default Workspace', 'Main development workspace');
          set({ workspaces: [created], activeWorkspaceId: created.id });
          localStorage.setItem('mitm_active_workspace_id', created.id);
          await get().loadEnvironments(created.id);
        }
      } catch (err) {
        console.error('Failed to init workspace store:', err);
      }
    }
  },

  selectWorkspace: async (id) => {
    set({ activeWorkspaceId: id });
    localStorage.setItem('mitm_active_workspace_id', id);
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
        localStorage.setItem('mitm_active_workspace_id', created.id);
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
        localStorage.setItem('mitm_active_workspace_id', newWs.id);
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
    if (nextActiveId) {
      localStorage.setItem('mitm_active_workspace_id', nextActiveId);
    } else {
      localStorage.removeItem('mitm_active_workspace_id');
    }

    if (isTauriAvailable()) {
      try {
        await deleteWorkspace(id);
        const freshWorkspaces = await getWorkspaces();
        set({ workspaces: freshWorkspaces });
        if (nextActiveId) {
          await get().selectWorkspace(nextActiveId);
        }
      } catch (err) {
        console.error('Failed to delete workspace:', err);
      }
    }
  },

  importProjectJson: async (jsonContent, targetWorkspaceId, customWorkspaceName) => {
    if (isTauriAvailable()) {
      try {
        const summary = await importWorkspaceJson(jsonContent, targetWorkspaceId, customWorkspaceName);
        await get().initStore();
        if (summary.workspaceId) {
          await get().selectWorkspace(summary.workspaceId);
          // Clear stale open tabs & fetch fresh collection tree with extract rules
          useCollectionStore.setState({ openRequests: [], activeRequestId: null });
          await useCollectionStore.getState().fetchCollections(summary.workspaceId);
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
      environmentsList: state.environmentsList.map((e) =>
        e.id === env.id
          ? { ...env }
          : env.isActive
          ? { ...e, isActive: false }
          : e
      ),
      activeEnvironmentId: env.isActive ? env.id : state.activeEnvironmentId,
    }));
    if (isTauriAvailable()) {
      try {
        await saveWorkspaceEnvironment(env);
      } catch (err) {
        console.error('Failed to save environment variables:', err);
      }
    }
  },

  setImportModalOpen: (open) => set({ isImportModalOpen: open, importTargetWorkspaceId: open ? get().importTargetWorkspaceId : null }),

  openImportModalForWorkspace: (workspaceId) =>
    set({ isImportModalOpen: true, importTargetWorkspaceId: workspaceId || get().activeWorkspaceId }),

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
