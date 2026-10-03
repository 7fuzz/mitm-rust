import { create } from 'zustand';
import {
  getWorkspaceEnvironments,
  saveWorkspaceEnvironment,
  REPEATER_WORKSPACE_ID,
  type Environment,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';
import { applyEnvironmentSave } from '../utils/envVariables';

interface RepeaterEnvState {
  environmentsList: Environment[];
  activeEnvironmentId: string;
  loadEnvironments: () => Promise<void>;
  saveEnvironmentVariables: (env: Environment) => Promise<void>;
}

const defaultEnvironment = (): Environment => {
  const now = Date.now();
  return {
    id: `env-${crypto.randomUUID()}`,
    workspaceId: REPEATER_WORKSPACE_ID,
    name: 'Default',
    isActive: true,
    variables: [],
    createdAtMs: now,
    updatedAtMs: now,
  };
};

/** Environments of the reserved Repeater workspace, separate from the active workspace's. */
export const useRepeaterEnvStore = create<RepeaterEnvState>((set) => ({
  environmentsList: [],
  activeEnvironmentId: '',

  loadEnvironments: async () => {
    if (!isTauriAvailable()) return;
    try {
      let envs = await getWorkspaceEnvironments(REPEATER_WORKSPACE_ID);
      // The variable switcher can only add variables to an existing environment
      if (envs.length === 0) {
        const env = defaultEnvironment();
        await saveWorkspaceEnvironment(env);
        envs = [env];
      }
      const active = envs.find((e) => e.isActive) || envs[0];
      set({ environmentsList: envs, activeEnvironmentId: active.id });
    } catch (err) {
      console.error('Failed to load Repeater environments:', err);
    }
  },

  saveEnvironmentVariables: async (env) => {
    set((state) => ({
      environmentsList: applyEnvironmentSave(state.environmentsList, env),
      activeEnvironmentId: env.isActive ? env.id : state.activeEnvironmentId,
    }));
    if (isTauriAvailable()) {
      try {
        await saveWorkspaceEnvironment(env);
      } catch (err) {
        console.error('Failed to save Repeater environment:', err);
      }
    }
  },
}));
