import { useState, useCallback, useMemo } from 'react';
import { RepeaterGroup, SyncData, RepeaterRequest } from './types';
import { invoke } from '@/lib/utils/tauri';

export function useRepeater(activeEnvId?: string) {
  const [repeaterRequests, setRepeaterRequests] = useState<RepeaterRequest[]>([]);
  const [repeaterGroups, setRepeaterGroups] = useState<RepeaterGroup[]>([]);
  const [activeGroupId, setActiveGroupId] = useState<string>('null');

  // Internal bootstrapper
  const initActiveGroup = useCallback((groupId: string) => setActiveGroupId(groupId), []);

  const mapBackendResponse = useCallback((requests: any[]): RepeaterRequest[] => {
    return requests.map(r => ({
      ...r,
      response: r.response ? {
        status: r.response.status_code ?? r.response.status ?? 0,
        headers: r.response.response_headers ?? r.response.headers ?? [],
        body: r.response.response_body ?? r.response.body ?? '',
        time: r.response.duration_ms ?? r.response.time,
      } : undefined,
    }));
  }, []);

  const fetchGroupRequests = useCallback(async (groupId: string) => {
    try {
      const requests = await invoke<any[]>('get_repeater_requests', { groupId });
      setRepeaterRequests(mapBackendResponse(requests));
    } catch (error) {
      console.error('Failed to fetch group requests:', error);
    }
  }, [mapBackendResponse]);

  const refreshRepeater = useCallback(async () => {
    try {
      const data = await invoke<SyncData>('sync_data');
      setRepeaterGroups(data.repeaterGroups);
      setRepeaterRequests(mapBackendResponse(data.repeaterRequests as any[]));
      if (data.activeGroupId) {
        setActiveGroupId(data.activeGroupId);
      }
    } catch (error) { console.error('Failed to refresh repeater data:', error); }
  }, [mapBackendResponse]);

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

  const importPostman = useCallback(async (onFileLoaded: (data: Record<string, any>) => void, notify?: any) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const collection = JSON.parse(text);
        
        if (!collection.info && !collection.item) {
          throw new Error("Invalid format. Make sure you are selecting a valid Postman collection export.");
        }
        
        const converted = convertPostmanToMitmFormat(collection);
        onFileLoaded(converted);
      } catch (error) { 
        console.error('Failed to import Postman collection:', error);
        notify?.error?.(`Failed to import Postman collection: ${error instanceof Error ? error.message : error}`);
      }
    };
    input.click();
  }, []);

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

  const updateGroupExtractions = useCallback(async (id: string, extract: Record<string, string>) => {
    setRepeaterGroups(prev => prev.map(g => g.id === id ? { ...g, extract } : g));
    try {
      await invoke('update_repeater_group_extractions', { id, extract });
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

  const bulkDeleteGroups = useCallback(async (ids: string[]) => {
    if (!ids || ids.length === 0) return;
    setRepeaterGroups(prev => prev.filter(g => !ids.includes(g.id)));
    setRepeaterRequests(prev => prev.filter(r => !r.groupId || !ids.includes(r.groupId)));
    try {
      await invoke('bulk_delete_repeater_groups', { ids });
      if (activeGroupId && ids.includes(activeGroupId)) switchGroup('null');
      await refreshRepeater();
    } catch (e) { console.error(e); }
  }, [activeGroupId, switchGroup, refreshRepeater]);

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
    importProject, finalizeImport, createGroup, renameGroup, deleteGroup, bulkDeleteGroups, cloneGroup, reorderRequests, reorderGroups,
    manageGroupAssignment, getAllGroups, bulkSync, updateGroupExtractions
  }), [
    repeaterRequests, repeaterGroups, activeGroupId, switchGroup, initActiveGroup,
    refreshRepeater, addEmptyRequest, duplicateRequest, deleteRequest, updateRequest, importPostman,
    importProject, finalizeImport, createGroup, renameGroup, deleteGroup, bulkDeleteGroups, cloneGroup, reorderRequests, reorderGroups,
    manageGroupAssignment, getAllGroups, bulkSync, updateGroupExtractions
  ]);
}

function convertPostmanToMitmFormat(collection: any): any {
  const projectName = collection.info?.name || "Postman Import";
  const test_cases: any[] = [];
  const placeholders: Record<string, string> = {};
  const all_environments: any[] = [];
  const all_variables: any[] = [];

  // Parse collection variables
  if (Array.isArray(collection.variable) && collection.variable.length > 0) {
    const envId = "postman-env";
    all_environments.push({ id: envId, name: `${projectName} (Collection Variables)` });

    collection.variable.forEach((v: any) => {
      if (v.key) {
        placeholders[v.key] = v.value || "";
        all_variables.push({
          environmentId: envId,
          name: v.key,
          activeIndex: 0,
          values: [
            {
              name: "(auto)",
              value: v.value || ""
            }
          ]
        });
      }
    });
  }

  function parseRequest(item: any, inheritedAuth: any = null) {
    const req = item.request || {};
    const method = req.method || "GET";
    let urlStr = "";

    if (typeof req.url === "string") {
      urlStr = req.url;
    } else if (req.url && typeof req.url === "object") {
      urlStr = req.url.raw || "";
    }

    const headers: Record<string, string> = {};
    if (Array.isArray(req.header)) {
      req.header.forEach((h: any) => {
        if (h.key && !h.disabled) {
          headers[h.key] = h.value || "";
        }
      });
    }

    // --- AUTO-GENERATE BEARER AUTH HEADER ---
    const auth = req.auth || inheritedAuth;
    if (auth && auth.type === "bearer") {
      let tokenVal = "";
      if (Array.isArray(auth.bearer)) {
        const tokenObj = auth.bearer.find((b: any) => b.key === "token");
        tokenVal = tokenObj ? tokenObj.value : "";
      } else if (auth.bearer && typeof auth.bearer === "object") {
        tokenVal = auth.bearer.token || auth.bearer.value || "";
      }
      if (tokenVal && !headers["Authorization"] && !headers["authorization"]) {
        headers["Authorization"] = `Bearer ${tokenVal}`;
      }
    }

    // --- AUTO-GENERATE CONTENT-TYPE/ACCEPT FOR JSON RAW BODY ---
    if (req.body && req.body.mode === "raw" && req.body.options?.raw?.language === "json") {
      const hasContentType = Object.keys(headers).some(k => k.toLowerCase() === "content-type");
      if (!hasContentType) {
        headers["Content-Type"] = "application/json";
      }
      const hasAccept = Object.keys(headers).some(k => k.toLowerCase() === "accept");
      if (!hasAccept) {
        headers["Accept"] = "application/json";
      }
    }

    let body = "";
    if (req.body) {
      if (req.body.mode === "raw") {
        body = req.body.raw || "";
      } else if (req.body.mode === "urlencoded" && Array.isArray(req.body.urlencoded)) {
        const parts = req.body.urlencoded
          .filter((f: any) => !f.disabled && f.key)
          .map((f: any) => `${encodeURIComponent(f.key)}=${encodeURIComponent(f.value || "")}`);
        body = parts.join("&");
      } else if (req.body.mode === "formdata" && Array.isArray(req.body.formdata)) {
        const parts = req.body.formdata
          .filter((f: any) => !f.disabled && f.key)
          .map((f: any) => `${f.key}=${f.value || ""}`);
        body = parts.join("\n");
      }
    }

    return {
      name: item.name || "Untitled Request",
      method,
      endpoint: urlStr,
      header: headers,
      body,
      params: {},
      extract: {}
    };
  }

  function traverse(item: any, parentNamePath: string[] = [], parentAuth: any = null) {
    const currentPath = [...parentNamePath, item.name || ""];
    const currentAuth = item.auth || parentAuth;
    if (Array.isArray(item.item) && !item.request) {
      // It's a folder
      const directRequests = item.item.filter((sub: any) => sub.request);
      if (directRequests.length > 0) {
        const groupName = currentPath.filter(Boolean).join(" / ");
        test_cases.push({
          name: groupName,
          url: "",
          target: directRequests.map((sub: any) => parseRequest(sub, currentAuth))
        });
      }
      
      const subFolders = item.item.filter((sub: any) => !sub.request);
      subFolders.forEach((sub: any) => traverse(sub, currentPath, currentAuth));
    } else if (item.request) {
      // Root-level request
      let rootGroup = test_cases.find(tc => tc.name === projectName);
      if (!rootGroup) {
        rootGroup = {
          name: projectName,
          url: "",
          target: []
        };
        test_cases.push(rootGroup);
      }
      rootGroup.target.push(parseRequest(item, currentAuth));
    }
  }

  if (Array.isArray(collection.item)) {
    collection.item.forEach((item: any) => traverse(item, [], collection.auth));
  }

  return {
    name: projectName,
    url: "",
    header: {},
    placeholders,
    all_environments,
    all_variables,
    test_cases
  };
}
