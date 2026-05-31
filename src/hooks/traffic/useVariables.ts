import { useState, useRef, useCallback, useMemo } from 'react';
import { GlobalVariable, Environment, SyncData } from './types';
import { invoke } from '@/lib/utils/tauri';

export function useVariables(prefs?: { autoSave: boolean }) {
  const [variables, setVariables] = useState<GlobalVariable[]>([]);
  const [environments, setEnvironments] = useState<Environment[]>([{ id: 'default-env-id', name: 'Default' }]);
  const [activeEnvId, setActiveEnvId] = useState('default-env-id');
  const debounceTimers = useRef<Record<string, any>>({});

  const loadVariables = useCallback((vars: GlobalVariable[], envs: Environment[], activeId: string) => {
    setVariables(vars.sort((a, b) => (a.orderIndex || 0) - (b.orderIndex || 0)));
    setEnvironments(envs);
    setActiveEnvId(activeId);
  }, []);

  const reorderVariables = useCallback(async (reorderedVars: GlobalVariable[]) => {
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
  }, [activeEnvId]);

  const addVariable = useCallback(async (v: GlobalVariable) => {
    setVariables(prev => [...prev, v]);
    try {
      await invoke('create_variable', { variable: v });
    } catch (e) { console.error(e); }
  }, []);

  const saveVariable = useCallback(async (id: string, updates: Partial<GlobalVariable>) => {
    setVariables(prev => {
      const current = prev.find(v => v.id === id);
      if (!current) return prev;
      const full = { ...current, ...updates };
      invoke('update_variable', { id, updates: full }).catch(e => console.error(e));
      return prev.map(v => v.id === id ? full : v);
    });
  }, []);

  const saveAllVariables = useCallback(async () => {
    const envVars = variables.filter(v => v.environmentId === activeEnvId);
    for (const v of envVars) {
      try {
        await invoke('update_variable', { id: v.id, updates: v });
      } catch (e) { console.error(e); }
    }
  }, [activeEnvId, variables]);

  const updateVariable = useCallback((id: string, updates: Partial<GlobalVariable>, immediate = false) => {
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
          const upds = debounceTimers.current[`pending-${id}`];
          const current = next.find(v => v.id === id);
          if (current) {
            invoke('update_variable', { id, updates: { ...current, ...upds } }).catch(e => console.error(e));
          }
          delete debounceTimers.current[`pending-${id}`];
        } else {
          debounceTimers.current[id] = setTimeout(() => {
            const upds = debounceTimers.current[`pending-${id}`];
            const current = next.find(v => v.id === id);
            if (current) {
              invoke('update_variable', { id, updates: { ...current, ...upds } }).catch(e => console.error(e));
            }
            delete debounceTimers.current[id];
            delete debounceTimers.current[`pending-${id}`];
          }, 1000);
        }
      }
      
      return next;
    });
  }, [prefs?.autoSave]);

  const deleteVariable = useCallback(async (id: string) => {
    setVariables(prev => prev.filter(v => v.id !== id));
    try {
      await invoke('delete_variable', { id });
    } catch (e) { console.error(e); }
  }, []);

  const setActiveEnvironment = useCallback(async (id: string, refreshGroups?: () => Promise<void>) => {
    setActiveEnvId(id);
    try {
      await invoke('set_active_environment', { id });
      const data = await invoke<SyncData>('sync_data');
      setVariables(data.variables);
      if (refreshGroups) await refreshGroups();
    } catch (error) {
      console.error("Failed to lazy-load environment variables:", error);
    }
  }, []);

  const createEnvironment = useCallback(async (name: string) => {
    const newId = crypto.randomUUID();
    setEnvironments(prev => [...prev, { id: newId, name, is_active: false }]);
    setActiveEnvId(newId);
    try {
      await invoke('create_environment', { id: newId, name });
      await invoke('set_active_environment', { id: newId });
    } catch (e) { console.error(e); }
  }, []);

  const renameEnvironment = useCallback(async (id: string, name: string) => {
    if (id === 'default-env-id' || !name.trim()) return;
    setEnvironments(prev => prev.map(e => e.id === id ? { ...e, name } : e));
  }, []);

  const deleteEnvironment = useCallback(async (id: string) => {
    if (id === 'default-env-id') return;
    setEnvironments(prev => prev.filter(e => e.id !== id));
    setVariables(prev => prev.filter(v => v.environmentId !== id)); // Local cascade
    try {
      await invoke('delete_environment', { id });
      if (activeEnvId === id) setActiveEnvironment('default-env-id');
    } catch (e) { console.error(e); }
  }, [activeEnvId, setActiveEnvironment]);

  const switchWorkspace = useCallback(async (
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
  }, []);

  const updateVariableAutoValue = useCallback((name: string, value: string) => {
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
  }, [activeEnvId, prefs?.autoSave]);

  return useMemo(() => ({
    variables, environments, activeEnvId,
    loadVariables,
    addVariable, updateVariable, deleteVariable, saveVariable, saveAllVariables,
    setActiveEnvironment, createEnvironment, renameEnvironment, deleteEnvironment,
    updateVariableAutoValue, reorderVariables, switchWorkspace
  }), [
    variables, environments, activeEnvId, loadVariables,
    addVariable, updateVariable, deleteVariable, saveVariable, saveAllVariables,
    setActiveEnvironment, createEnvironment, renameEnvironment, deleteEnvironment,
    updateVariableAutoValue, reorderVariables, switchWorkspace
  ]);
}
