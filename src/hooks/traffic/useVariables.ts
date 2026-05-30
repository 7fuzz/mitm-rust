import { useState, useRef } from 'react';
import { GlobalVariable, Environment } from './types';

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

  const reorderVariables = (reorderedVars: GlobalVariable[]) => {
    // 1. Optimistic Update
    const updated = reorderedVars.map((v, idx) => ({ ...v, orderIndex: idx }));
    setVariables(prev => {
      const otherEnvVars = prev.filter(v => v.environmentId !== activeEnvId);
      return [...otherEnvVars, ...updated].sort((a, b) => (a.orderIndex || 0) - (b.orderIndex || 0));
    });

    // 2. Persist (ONLY send id and orderIndex)
    const payload = updated.map(v => ({ id: v.id, orderIndex: v.orderIndex }));
    fetch('/api/variables/bulk', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    }).catch(console.error);
  };

  const addVariable = (v: GlobalVariable) => {
    setVariables(prev => [...prev, v]);
    fetch('/api/variables', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(v) }).catch(console.error);
  };

  const saveVariable = (id: string, updates: Partial<GlobalVariable>) => {
    fetch(`/api/variables/${id}`, { 
      method: 'PUT', 
      headers: { 'Content-Type': 'application/json' }, 
      body: JSON.stringify(updates) 
    }).catch(console.error);
  };

  const saveAllVariables = async () => {
    const envVars = variables.filter(v => v.environmentId === activeEnvId);
    if (envVars.length === 0) return;
    
    // For manual "Save All", we still send the full objects because they might have changed
    await fetch('/api/variables/bulk', { 
      method: 'PUT', 
      headers: { 'Content-Type': 'application/json' }, 
      body: JSON.stringify(envVars) 
    }).catch(console.error);
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

  const deleteVariable = (id: string) => {
    setVariables(prev => prev.filter(v => v.id !== id));
    fetch(`/api/variables/${id}`, { method: 'DELETE' }).catch(console.error);
  };

  const setActiveEnvironment = async (id: string, refreshGroups?: () => Promise<void>) => {
    setActiveEnvId(id);

    fetch('/api/environments', {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ activeId: id })
    }).catch(console.error);

    try {
      const res = await fetch(`/api/variables?envId=${id}`);
      const data = await res.json();
      if (data.variables) {
        setVariables(data.variables);
      }
      
      // Refresh groups for the new environment
      if (refreshGroups) await refreshGroups();
    } catch (error) {
      console.error("Failed to lazy-load environment variables:", error);
    }
  };

  const createEnvironment = (name: string) => {
    const newId = crypto.randomUUID();
    setEnvironments(prev => [...prev, { id: newId, name }]);
    setActiveEnvId(newId);
    fetch('/api/environments', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ id: newId, name, activeId: newId }) });
  };

  const renameEnvironment = (id: string, name: string) => {
    if (id === 'default-env-id' || !name.trim()) return;
    setEnvironments(prev => prev.map(e => e.id === id ? { ...e, name } : e));
    fetch(`/api/environments/${id}`, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) });
  };

  const deleteEnvironment = (id: string) => {
    if (id === 'default-env-id') return;
    setEnvironments(prev => prev.filter(e => e.id !== id));
    setVariables(prev => prev.filter(v => v.environmentId !== id)); // Local cascade
    if (activeEnvId === id) setActiveEnvironment('default-env-id');
    fetch(`/api/environments/${id}`, { method: 'DELETE' });
  };

  const switchWorkspace = async (
    envId: string, 
    groupId: string, 
    onSuccess: (data: { variables: any[], groups: any[], requests: any[] }) => void
  ) => {
    setActiveEnvId(envId);
    try {
      const res = await fetch('/api/workspace/switch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ envId, groupId })
      });
      const data = await res.json();
      if (data.success) {
        if (data.environments) setEnvironments(data.environments);
        setVariables(data.variables);
        onSuccess(data);
      }
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
            debounceTimers.current[timerId] = setTimeout(() => {
              fetch(`/api/variables/${v.id}`, { 
                method: 'PUT', 
                headers: { 'Content-Type': 'application/json' }, 
                body: JSON.stringify({ values: newValues }) 
              }).catch(console.error);
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

