import { useState, useCallback, useMemo } from 'react';
import { RepeaterGroup, SyncData, RepeaterRequest } from './types';
import { invoke } from '@/lib/utils/tauri';

export function useRepeater(activeEnvId?: string) {
  const [repeaterRequests, setRepeaterRequests] = useState<RepeaterRequest[]>([]);
  const [repeaterGroups, setRepeaterGroups] = useState<RepeaterGroup[]>([]);
  const [activeGroupId, setActiveGroupId] = useState<string>('null');

  // Internal bootstrapper
  const initActiveGroup = useCallback((groupId: string) => setActiveGroupId(groupId), []);

  const fetchGroupRequests = useCallback(async (groupId: string) => {
    try {
      const requests = await invoke<RepeaterRequest[]>('get_repeater_requests', { groupId });
      setRepeaterRequests(requests);
    } catch (error) {
      console.error('Failed to fetch group requests:', error);
    }
  }, []);

  const refreshRepeater = useCallback(async () => {
    try {
      const data = await invoke<SyncData>('sync_data');
      setRepeaterGroups(data.repeaterGroups);
      // Backend already filtered repeaterRequests in sync_data based on saved activeGroupId
      setRepeaterRequests(data.repeaterRequests);
      if (data.activeGroupId) {
        setActiveGroupId(data.activeGroupId);
      }
    } catch (error) { console.error('Failed to refresh repeater data:', error); }
  }, []);

  const switchGroup = useCallback(async (groupId: string) => {
    setActiveGroupId(groupId);
    // Save to backend
    invoke('save_state', { key: 'active_group_id', value: groupId }).catch(() => {});
    // Fetch requests for this group
    await fetchGroupRequests(groupId);
  }, [fetchGroupRequests]);

  const addEmptyRequest = useCallback(async (targetGroup: string | null = null, notify?: any) => {
    try {
      const id = await invoke<string>('create_repeater_item', { 
        item: { name: 'New Request', method: 'GET', url: 'https://example.com/api/', headers: [], body: '', response: null, group_id: targetGroup } 
      });
      if (id) { await fetchGroupRequests(activeGroupId); return id; }
    } catch (error) { 
      console.error('Error creating request:', error);
      notify?.error?.('Error creating request: ' + error);
    }
    return null;
  }, [activeGroupId, fetchGroupRequests]);

  const duplicateRequest = useCallback(async (currentReq: RepeaterRequest, notify?: any) => {
    try {
      const id = await invoke<string>('create_repeater_item', { 
        item: {
          name: `${currentReq.name} (Copy)`,
          method: currentReq.method,
          url: currentReq.url,
          headers: currentReq.headers || [],
          body: currentReq.body || '',
          response: currentReq.response,
          group_id: currentReq.groupId
        } 
      });
      if (id) { await fetchGroupRequests(activeGroupId); return id; }
    } catch (error) { 
      console.error('Error duplicating request:', error);
      notify?.error?.('Error duplicating request: ' + error);
    }
    return null;
  }, [activeGroupId, fetchGroupRequests]);

  const deleteRequest = useCallback(async (id: string) => {
    setRepeaterRequests(prev => prev.filter(r => r.id !== id));
    try {
      await invoke('delete_repeater_request', { id });
    } catch (e) { console.error(e); }
  }, []);

  const updateRequest = useCallback(async (id: string, updates: Partial<RepeaterRequest>) => {
    setRepeaterRequests(prev => {
      const current = prev.find(r => r.id === id);
      if (!current) return prev;
      const full = { ...current, ...updates };
      
      invoke('update_repeater_request', { id, updates: full }).catch(e => console.error(e));
      
      return prev.map(r => r.id === id ? full : r);
    });
  }, []);

  const importPostman = useCallback(async (notify?: any) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const collection = JSON.parse(text);
        const groupName = collection.info?.name || 'Postman Import';

        const response = await fetch('/api/repeater-import', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ collection, group: groupName }),
        });
        const result = await response.json();

        if (result.success) {
          notify?.success?.(`✓ Imported ${result.imported} request(s)`);
          await switchGroup('null');
        } else notify?.error?.(`Error: ${result.error}`);
      } catch (error) { notify?.error?.(`Failed to import: ${error}`); }
    };
    input.click();
  }, [switchGroup]);

  const importProject = useCallback(async (onFileLoaded: (data: Record<string, unknown>) => void, notify?: any) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const data = JSON.parse(text);
        onFileLoaded(data);
      } catch (error) { notify?.error?.(`Failed to load file: ${error}`); }
    };
    input.click();
  }, []);

  const finalizeImport = useCallback(async (data: Record<string, any>, options: Record<string, any>, notify: any) => {
    try {
      // Transform the data for the import command
      const importPayload = {
        name: data.name,
        url: data.url,
        header: data.header,
        placeholders: data.placeholders,
        all_environments: data.all_environments,
        all_variables: data.all_variables,
        test_cases: data.test_cases,
        import_environments: options.selectedEnvIds || [],
        import_groups: options.selectedGroupNames || [],
        link_to_environments: options.targetEnvIds || [],
        smart_link: options.smartSync || false,
      };

      const result = await invoke<any>('import_repeater_data', { data: importPayload });

      if (result.success) {
        const imported = result.imported || {};
        const summary = [];
        if (imported.environments) summary.push(`${imported.environments} environment(s)`);
        if (imported.variables) summary.push(`${imported.variables} variable(s)`);
        if (imported.groups) summary.push(`${imported.groups} collection(s)`);
        if (imported.requests) summary.push(`${imported.requests} request(s)`);
        
        const message = summary.length > 0 
          ? `✓ Imported ${summary.join(', ')}`
          : '✓ Import completed';
        
        notify?.success?.(message);
        await refreshRepeater();
      } else {
        notify?.error?.(`Import failed: ${result.error || 'Unknown error'}`);
      }
    } catch (error) {
      console.error('Import error:', error);
      notify?.error?.(`Import failed: ${error}`);
    }
  }, [refreshRepeater]);

  const createGroup = useCallback(async (name: string) => {
    if (!name.trim()) return null;
    try {
      const id = await invoke<string>('create_repeater_group', { name });
      await refreshRepeater();
      return id;
    } catch (e) { console.error(e); return null; }
  }, [refreshRepeater]);

  const renameGroup = useCallback(async (id: string, name: string) => {
    if (!name.trim()) return;
    setRepeaterGroups(prev => prev.map(g => g.id === id ? { ...g, name } : g));
    try {
      await invoke('rename_repeater_group', { id, name });
    } catch (e) { console.error(e); }
  }, []);

  const deleteGroup = useCallback(async (id: string) => {
    setRepeaterGroups(prev => prev.filter(g => g.id !== id));
    setRepeaterRequests(prev => prev.filter(r => r.groupId !== id));
    try {
      await invoke('delete_repeater_group', { id });
      if (activeGroupId === id) switchGroup('null');
    } catch (e) { console.error(e); }
  }, [activeGroupId, switchGroup]);

  const cloneGroup = useCallback(async (id: string, newName: string) => {
    try {
      const newGroupId = await createGroup(newName);
      if (!newGroupId) return null;

      const data = await invoke<SyncData>('sync_data');
      const originalReqs = data.repeaterRequests.filter(r => r.groupId === id);

      for (const req of originalReqs) {
        await invoke('create_repeater_item', { 
          item: {
            name: req.name,
            method: req.method,
            url: req.url,
            headers: req.headers,
            body: req.body,
            groupId: newGroupId,
            response: null
          } 
        });
      }

      await refreshRepeater();
      return newGroupId;
    } catch (error) {
      console.error("Failed to clone group:", error);
      return null;
    }
  }, [createGroup, refreshRepeater]);

  const reorderRequests = useCallback(async (reorderedIds: string[]) => {
    setRepeaterRequests(prev => [...prev].sort((a, b) => reorderedIds.indexOf(a.id) - reorderedIds.indexOf(b.id)));
    try {
      await invoke('reorder_repeater_requests', { ids: reorderedIds });
    } catch (e) { console.error(e); }
  }, []);

  const reorderGroups = useCallback(async (reorderedIds: string[]) => {
    setRepeaterGroups(prev => [...prev].sort((a, b) => reorderedIds.indexOf(a.id) - reorderedIds.indexOf(b.id)));
    try {
      await invoke('reorder_repeater_groups', { ids: reorderedIds });
    } catch (e) { console.error(e); }
  }, []);

  const bulkSync = useCallback((groups: RepeaterGroup[], requests: RepeaterRequest[]) => {
    setRepeaterGroups(groups);
    setRepeaterRequests(requests);
  }, []);

  const getAllGroups = useCallback(async () => {
    const data = await invoke<SyncData>('sync_data');
    return data.repeaterGroups;
  }, []);

  const manageGroupAssignment = useCallback(async (
    action: 'link' | 'unlink' | 'move' | 'get_assignments' | 'get_groups_for_env' | 'bulk', 
    groupId?: string, 
    envId?: string, 
    targetEnvId?: string,
    bulkPayload?: { links: { groupId: string, envId: string }[], unlinks: { groupId: string, envId: string }[] }
  ) => {
    try {
      const res = await invoke<any>('manage_group_assignment', { 
        action, 
        groupId, 
        envId: envId || activeEnvId, 
        targetEnvId,
        bulkLinks: bulkPayload?.links.map(l => [l.groupId, l.envId]),
        bulkUnlinks: bulkPayload?.unlinks.map(l => [l.groupId, l.envId])
      });
      
      if (action === 'get_assignments' || action === 'get_groups_for_env') return res as string[];
      if (res.success) await refreshRepeater();
      return res.success;
    } catch (e) {
      console.error('Group assignment failed:', e);
      return false;
    }
  }, [activeEnvId, refreshRepeater]);

  return useMemo(() => ({
    repeaterRequests, repeaterGroups, activeGroupId, switchGroup,
    _setRawRepeater: setRepeaterRequests, _setRawGroups: setRepeaterGroups, initActiveGroup,
    refreshRepeater, addEmptyRequest, duplicateRequest, deleteRequest, updateRequest, importPostman,
    importProject, finalizeImport, createGroup, renameGroup, deleteGroup, cloneGroup, reorderRequests, reorderGroups,
    manageGroupAssignment, getAllGroups, bulkSync
  }), [
    repeaterRequests, repeaterGroups, activeGroupId, switchGroup, initActiveGroup,
    refreshRepeater, addEmptyRequest, duplicateRequest, deleteRequest, updateRequest, importPostman,
    importProject, finalizeImport, createGroup, renameGroup, deleteGroup, cloneGroup, reorderRequests, reorderGroups,
    manageGroupAssignment, getAllGroups, bulkSync
  ]);
}
