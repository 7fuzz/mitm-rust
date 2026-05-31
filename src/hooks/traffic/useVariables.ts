import { useState, useRef } from 'react';
import { GlobalVariable, Environment, SyncData } from './types';
import { invoke } from '@/lib/utils/tauri';

export function useVariables(prefs?: { autoSave: boolean }) {
  const [variables, setVariables] = useState<GlobalVariable[]>([]);
  const [environments, setEnvironments] = useState<Environment[]>([{ id: 'default-env-id', name: 'Default' }]);
  const [activeEnvId, setActiveEnvId] = useState('default-env-id');
  const debounceTimers = useRef<Record<string, any>>({});

  const loadVariables = (vars: GlobalVariable[], envs: Environment[], activeId: string) => {
    setVariables(vars.sort((a, b) => (a.orderIndex || 0) - (b.orderIndex || 0)));
    setEnvironments(envs);
    setActiveEnvId(activeId);
  };

  const reorderVariables = async (reorderedVars: GlobalVariable[]) => {
    // 1. Optimistic Update
    const updated = reorderedVars.map((v, idx) => ({ ...v, orderIndex: idx }));
    setVariables(prev => {
      const otherEnvVars = prev.filter(v => v.environmentId !== activeEnvId);
      return [...otherEnvVars, ...updated].sort((a, b) => (a.orderIndex || 0) - (b.orderIndex || 0));
    });

    // 2. Persist
    for (const v of updated) {
      try {
        await invoke('update_variable', { id: v.id, updates: v });
      } catch (e) { console.error(e); }
    }
  };

  const addVariable = async (v: GlobalVariable) => {
    setVariables(prev => [...prev, v]);
    try {
      await invoke('create_variable', { variable: v });
    } catch (e) { console.error(e); }
  };

  const saveVariable = async (id: string, updates: Partial<GlobalVariable>) => {
    const current = variables.find(v => v.id === id);
    if (!current) return;
    const full = { ...current, ...updates };
    try {
      await invoke('update_variable', { id, updates: full });
    } catch (e) { console.error(e); }
  };

  const saveAllVariables = async () => {
    const envVars = variables.filter(v => v.environmentId === activeEnvId);
    for (const v of envVars) {
      try {
        await invoke('update_variable', { id: v.id, updates: v });
      } catch (e) { console.error(e); }
    }
  };

  const updateVariable = (id: string, updates: Partial<GlobalVariable>, immediate = false) => {
    setVariables(prev => {
      const next = prev.map(v => v.id === id ? { ...v, ...updates } : v);
      
      if (prefs?.autoSave || immediate) {
        if (debounceTimers.current[id]) {
          // Merge updates for the debounced save
          debounceTimers.current[`pending-${id}`] = { 
            ...(debounceTimers.current[`pending-${id}`] || {}), 
            ...updates 
          };
          clearTimeout(debounceTimers.current[id]);
        } else {
          debounceTimers.current[`pending-${id}`] = updates;
        }
        
        if (immediate) {
          saveVariable(id, debounceTimers.current[`pending-${id}`]);
          delete debounceTimers.current[`pending-${id}`];
        } else {
          debounceTimers.current[id] = setTimeout(() => {
            saveVariable(id, debounceTimers.current[`pending-${id}`]);
            delete debounceTimers.current[id];
            delete debounceTimers.current[`pending-${id}`];
          }, 1000);
        }
      }
      
      return next;
    });
  };

  const deleteVariable = async (id: string) => {
    setVariables(prev => prev.filter(v => v.id !== id));
    try {
      await invoke('delete_variable', { id });
    } catch (e) { console.error(e); }
  };

  const setActiveEnvironment = async (id: string, refreshGroups?: () => Promise<void>) => {
    setActiveEnvId(id);
    try {
      await invoke('set_active_environment', { id });
      const data = await invoke<SyncData>('sync_data');
      setVariables(data.variables);
      if (refreshGroups) await refreshGroups();
    } catch (error) {
      console.error("Failed to lazy-load environment variables:", error);
    }
  };

  const createEnvironment = async (name: string) => {
    const newId = crypto.randomUUID();
    setEnvironments(prev => [...prev, { id: newId, name, is_active: false }]);
    setActiveEnvId(newId);
    try {
      await invoke('create_environment', { id: newId, name });
      await invoke('set_active_environment', { id: newId });
    } catch (e) { console.error(e); }
  };

  const renameEnvironment = async (id: string, name: string) => {
    if (id === 'default-env-id' || !name.trim()) return;
    setEnvironments(prev => prev.map(e => e.id === id ? { ...e, name } : e));
    // TODO: implement rename_environment in rust if needed
  };

  const deleteEnvironment = async (id: string) => {
    if (id === 'default-env-id') return;
    setEnvironments(prev => prev.filter(e => e.id !== id));
    setVariables(prev => prev.filter(v => v.environmentId !== id)); // Local cascade
    try {
      await invoke('delete_environment', { id });
      if (activeEnvId === id) setActiveEnvironment('default-env-id');
    } catch (e) { console.error(e); }
  };

  const switchWorkspace = async (
    envId: string, 
    _groupId: string, 
    onSuccess: (data: { variables: any[], groups: any[], requests: any[] }) => void
  ) => {
    setActiveEnvId(envId);
    try {
      await invoke('set_active_environment', { id: envId });
      const data = await invoke<SyncData>('sync_data');
      setVariables(data.variables);
      onSuccess({ variables: data.variables, groups: data.repeaterGroups, requests: data.repeaterRequests });
    } catch (error) {
      console.error("Failed to switch workspace context:", error);
    }
  };

  const updateVariableAutoValue = (name: string, value: string) => {
    setVariables(prev => prev.map(v => {
      if (v.name === name && v.environmentId === activeEnvId) {
        const autoVal = v.values.find(val => val.name === '(auto)');
        if (autoVal) {
          const newValues = v.values.map(val => val.name === '(auto)' ? { ...val, value } : val);
          
          if (prefs?.autoSave) {
            const timerId = `auto-${v.id}`;
            if (debounceTimers.current[timerId]) clearTimeout(debounceTimers.current[timerId]);
            debounceTimers.current[timerId] = setTimeout(async () => {
              try {
                await invoke('update_variable', { id: v.id, updates: { ...v, values: newValues } });
              } catch (e) { console.error(e); }
              delete debounceTimers.current[timerId];
            }, 2000);
          }
          
          return { ...v, values: newValues };
        }
      }
      return v;
    }));
  };

  return {
    variables, environments, activeEnvId,
    loadVariables,
    addVariable, updateVariable, deleteVariable, saveVariable, saveAllVariables,
    setActiveEnvironment, createEnvironment, renameEnvironment, deleteEnvironment,
    updateVariableAutoValue, reorderVariables, switchWorkspace
  };
}

