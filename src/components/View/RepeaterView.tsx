import { useState, useCallback, useRef, useEffect } from 'react';
import { HeaderEditor } from '../Editor/HeaderEditor';
import { BodyEditor } from '../Editor/BodyEditor';
import { UrlEditor } from '../Editor/UrlEditor';
import { TrafficList } from '../Sidebar/TrafficList';
import { Traffic } from '@/types/traffic';
import HttpResponseViewer from '../ui/HttpResponseViewer';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { useTraffic, RepeaterRequest } from '@/hooks/traffic';
import { useNotification } from '../ui/NotificationProvider';
import { ExtractionModal, RepeaterHistoryModal, CollectionDocModal } from '../Modals';
import { Button, Select, useDialog, Textarea } from '../ui';
import { MarkdownViewer } from '../ui/MarkdownViewer';
import { invoke } from '@/lib/utils/tauri';

export function RepeaterView() {
  const { notify } = useNotification();
  const { confirm, prompt } = useDialog();
  const {
    repeaterRequests, repeaterGroups, activeGroupId, switchGroup,
    addEmptyRequest, duplicateRequest, updateRequest, deleteRequest,
    createGroup, renameGroup, deleteGroup, reorderRequests,
    variables, activeEnvId, updateVariableAutoValue,
    uiLayout, updateUILayout,
    repeaterSelectedId: selectedId, setRepeaterSelectedId: setSelectedId,
    _setRawRepeater,
    simpleMode,
    updateGroupExtractions,
    refreshRepeater
  } = useTraffic();

  const [isLoading, setIsLoading] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [editName, setEditName] = useState('');
  const [editGroupId, setEditGroupId] = useState<string | null>(null);
  const [editMethod, setEditMethod] = useState('GET');
  const [editUrl, setEditUrl] = useState('');
  const [editHeaders, setEditHeaders] = useState<[string, string][]>([]);
  const [editBody, setEditBody] = useState('');
  const [editBodyMode, setEditBodyMode] = useState<'raw' | 'json' | 'urlencoded' | 'multipart'>('raw');
  const [editBodyJson, setEditBodyJson] = useState('');
  const [editBodyUrlencoded, setEditBodyUrlencoded] = useState('');
  const [editBodyMultipart, setEditBodyMultipart] = useState('');
  const [editUrlParams, setEditUrlParams] = useState('');
  const [editExtract, setEditExtract] = useState<Record<string, string>>({});
  const [editDescription, setEditDescription] = useState('');
  const [docViewMode, setDocViewMode] = useState<'preview' | 'edit'>('preview');

  const [extractionModalOpen, setExtractionModalOpen] = useState(false);
  const [groupExtractionModalOpen, setGroupExtractionModalOpen] = useState(false);
  const [historyModalOpen, setHistoryModalOpen] = useState(false);
  const [groupDocModalOpen, setGroupDocModalOpen] = useState(false);

  // Debounce for name updates
  const nameDebounceRef = useRef<NodeJS.Timeout | null>(null);
  const debouncedUpdateName = useCallback((id: string, name: string) => {
    if (nameDebounceRef.current) clearTimeout(nameDebounceRef.current);
    nameDebounceRef.current = setTimeout(() => {
      updateRequest(id, { name });
    }, 300);
  }, [updateRequest]);

  // Debounce for description updates
  const descDebounceRef = useRef<NodeJS.Timeout | null>(null);
  const debouncedUpdateDescription = useCallback((id: string, description: string) => {
    if (descDebounceRef.current) clearTimeout(descDebounceRef.current);
    descDebounceRef.current = setTimeout(() => {
      updateRequest(id, { description });
    }, 300);
  }, [updateRequest]);

  const currentReq = repeaterRequests.find(r => r.id === selectedId) || repeaterRequests[0] || null;
  const activeGroupObj = repeaterGroups.find(g => g.id === (currentReq?.groupId || activeGroupId));
  const activeGroup = activeGroupObj;

  useEffect(() => {
    if (!repeaterRequests.length) {
      if (selectedId && setSelectedId) setSelectedId(null);
      return;
    }

    if (!selectedId || !repeaterRequests.some(r => r.id === selectedId)) {
      setSelectedId(repeaterRequests[0].id);
    }
  }, [repeaterRequests, selectedId, setSelectedId]);

  // Auto-update selectedId if we defaulted to repeaterRequests[0]
  useEffect(() => {
    if (currentReq && currentReq.id !== selectedId && setSelectedId) {
      setSelectedId(currentReq.id);
    }
  }, [currentReq, selectedId, setSelectedId]);

  // Sync builder states when selected request changes
  useEffect(() => {
    if (currentReq) {
      setEditName(currentReq.name);
      setEditGroupId(currentReq.groupId || null);
      setEditMethod(currentReq.method);
      setEditUrl(currentReq.url);
      setEditHeaders(currentReq.headers || []);
      setEditBody(currentReq.body || '');
      setEditBodyMode(currentReq.bodyMode || 'raw');
      setEditBodyJson(currentReq.bodyJson || '');
      setEditBodyUrlencoded(currentReq.bodyUrlencoded || '');
      setEditBodyMultipart(currentReq.bodyMultipart || '');
      setEditUrlParams(currentReq.urlParams || '');
      setEditExtract(currentReq.extract || {});
      setEditDescription(currentReq.description || '');
      setDocViewMode('preview');
    }
  }, [currentReq?.id]);

  const trafficMapped: Traffic[] = repeaterRequests.map(req => {
    const groupName = req.groupId ? repeaterGroups.find(g => g.id === req.groupId)?.name : 'Default';
    return {
      id: req.id,
      method: req.method,
      url: req.name,
      status_code: req.response?.status ?? 0,
      host: '',
      phase: 'history',
      request_headers: [],
      response_headers: [],
      request_body: '',
      response_body: '',
      is_intercepted: false,
      group: groupName || 'Default',
      hit_count: req.hitCount,
      duration_ms: req.response?.time
    };
  });

  const handleAdd = async () => {
    const targetGroup = (activeGroupId !== 'All' && activeGroupId !== 'null') ? activeGroupId : null;
    const newId = await addEmptyRequest(targetGroup, notify);
    if (newId) setSelectedId(newId);
  };

  const handleDuplicate = async () => {
    if (!currentReq) return;
    const newId = await duplicateRequest(currentReq, notify);
    if (newId) setSelectedId(newId);
  };

  const handleSend = async () => {
    if (!currentReq) return;
    setIsLoading(true);
    try {
      // 1. Sync builder UI to backend before execution
      const updatedReq: RepeaterRequest = {
        ...currentReq,
        method: editMethod,
        url: editUrl,
        headers: editHeaders,
        body: editBody,
        bodyMode: editBodyMode,
        bodyJson: editBodyJson,
        bodyUrlencoded: editBodyUrlencoded,
        bodyMultipart: editBodyMultipart,
        urlParams: editUrlParams,
        name: editName,
        extract: editExtract,
        groupId: editGroupId,
        description: editDescription
      };

      await updateRequest(currentReq.id, {
        method: editMethod,
        url: editUrl,
        headers: editHeaders,
        body: editBody,
        bodyMode: editBodyMode,
        bodyJson: editBodyJson,
        bodyUrlencoded: editBodyUrlencoded,
        bodyMultipart: editBodyMultipart,
        urlParams: editUrlParams,
        name: editName,
        extract: editExtract,
        groupId: editGroupId,
        description: editDescription
      });

      // 2. Map variables for interpolation
      const currentEnvVars = variables.filter(v => v.environmentId === activeEnvId);
      const varMap: Record<string, string> = {};
      currentEnvVars.forEach(v => {
        const val = v.values[v.activeIndex]?.value ?? v.values[0]?.value ?? '';
        varMap[v.name] = val;
      });

      // 3. Execute via Rust backend
      const response = await invoke<Traffic>('execute_repeater_request', {
        id: currentReq.id
      });

      const updatedWithRes: RepeaterRequest = {
        ...updatedReq,
        response: {
          status: response.status_code,
          headers: response.response_headers,
          body: response.response_body,
          time: response.duration_ms || undefined
        },
        hitCount: (currentReq.hitCount || 0) + 1
      };

      _setRawRepeater((prev: RepeaterRequest[]) => prev.map((r: RepeaterRequest) => r.id === currentReq.id ? updatedWithRes : r));

      // --- EXTRACTION LOGIC ---
      const collectionExtract = activeGroup?.extract || {};
      const mergedExtract = { ...collectionExtract, ...editExtract };

      if (mergedExtract && Object.keys(mergedExtract).length > 0) {
        try {
          const respJson = JSON.parse(response.response_body);
          Object.entries(mergedExtract).forEach(([varName, rawPath]) => {
            const cleanPath = typeof rawPath === 'string' ? rawPath.replace(/^\$\.?/, '') : '';
            if (!cleanPath) return;
            const value = cleanPath.split('.').reduce((obj, key) => (obj as any)?.[key], respJson);
            if (value !== undefined) {
              updateVariableAutoValue(varName, String(value));
            }
          });
        } catch {
          console.error("Failed to parse response for extraction");
        }
      }

      notify.success('Execution complete');
    } catch (error) {
      notify.error(`Execution failed: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const getPreviewRequestText = () => {
    if (!currentReq) return '';
    const currentEnvVars = variables.filter(v => v.environmentId === activeEnvId);
    const varMap: Record<string, string> = {};
    currentEnvVars.forEach(v => {
      const val = v.values[v.activeIndex]?.value ?? v.values[0]?.value ?? '';
      varMap[v.name] = val;
    });

    let interpolatedUrl = editUrl;
    let interpolatedBody = editBody;

    Object.entries(varMap).forEach(([k, v]) => {
      const regex = new RegExp(`\\{\\{${k}\\}\\}`, 'g');
      interpolatedUrl = interpolatedUrl.replace(regex, v);
      interpolatedBody = interpolatedBody.replace(regex, v);
    });

    const headersText = editHeaders.map(([k, v]) => {
      let finalV = v;
      Object.entries(varMap).forEach(([vk, vv]) => {
        finalV = finalV.replace(new RegExp(`\\{\\{${vk}\\}\\}`, 'g'), vv);
      });
      return `${k}: ${finalV}`;
    }).join('\n');

    return `${editMethod} ${interpolatedUrl} HTTP/1.1\n${headersText}\n\n${interpolatedBody}`;
  };

  return (
    <>
      <WorkspaceLayout
        uiLayout={uiLayout}
        onUpdateLayout={updateUILayout}
        listComponent={() => (
          <TrafficList
            items={trafficMapped}
            activeId={selectedId}
            onSelect={setSelectedId}
            onDelete={deleteRequest}
            onReorder={reorderRequests}
            activeColor="purple"
            layout="sidebar"
          />
        )}

        toolbarLeft={!simpleMode ? (
          <div className="flex items-center gap-2 bg-zinc-950 p-1 rounded-full border border-zinc-800 px-3 shadow-inner shadow-app-shadow/50">
            <span className="text-[9px] text-zinc-500 font-black uppercase tracking-widest hidden sm:inline-block">Collection:</span>
            <Select
              value={activeGroupId}
              onChange={switchGroup}
              options={[
                { value: "All", label: "All Groups", color: 'text-sky-400' },
                { isDivider: true },
                { value: "null", label: "Default (Uncategorized)", color: 'text-zinc-500' },
                { isDivider: true },
                { isHeader: true, label: "My Collections" },
                ...repeaterGroups.map(g => ({ value: g.id, label: g.name }))
              ]}
              className="w-44"
            />

            <div className="flex items-center gap-1 border-l border-zinc-800 pl-2 ml-1">
              <button
                onClick={() => setGroupDocModalOpen(true)}
                disabled={activeGroupId === 'All' || activeGroupId === 'null'}
                className="p-1 text-zinc-500 hover:text-purple-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors flex items-center gap-1"
                title="Collection Documentation & Notes (Markdown)"
              >
                <span className="text-[10px] font-bold">📝 Docs</span>
              </button>
              <button
                onClick={async () => {
                  if (activeGroupObj) {
                    const newName = await prompt('Rename Collection', 'Enter new collection name:', activeGroupObj.name);
                    if (newName) renameGroup(activeGroupObj.id, newName);
                  }
                }}
                disabled={activeGroupId === 'All' || activeGroupId === 'null'}
                className="p-1 text-zinc-500 hover:text-purple-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors"
                title="Rename Collection"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
              </button>
              <button
                onClick={() => setGroupExtractionModalOpen(true)}
                disabled={activeGroupId === 'All' || activeGroupId === 'null'}
                className="p-1 text-zinc-500 hover:text-amber-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors"
                title="Collection Extraction Rules"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"></path><circle cx="12" cy="12" r="3"></circle></svg>
              </button>
              <button
                onClick={async () => {
                  if (activeGroupObj) {
                    if (await confirm(
                      'Delete Collection',
                      `Are you sure you want to delete "${activeGroupObj.name}"? ALL requests inside this collection will be permanently destroyed.`,
                      true
                    )) {
                      deleteGroup(activeGroupObj.id);
                    }
                  }
                }}
                disabled={activeGroupId === 'All' || activeGroupId === 'null'}
                className="p-1 text-zinc-500 hover:text-rose-500 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors"
                title="Delete Collection"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
              </button>
            </div>
          </div>
        ) : undefined}

        toolbarRight={
          <>
            <Button variant="ghost" size="sm" onClick={() => setHistoryModalOpen(true)} disabled={!currentReq} title="View Request History" className="mr-2">History</Button>
            <Button variant="destructive" size="sm" onClick={() => currentReq && updateRequest(currentReq.id, { response: undefined })} disabled={!currentReq?.response} className="mr-2">Clear</Button>

            <div className="flex items-center gap-px">
              <Button variant="secondary" size="sm" onClick={handleAdd} className="rounded-r-none border-r-0 text-emerald-text" title="New Request">+ New</Button>
              <Button variant="secondary" size="sm" onClick={handleDuplicate} disabled={!currentReq} className="rounded-l-none" title="Duplicate Request">Copy</Button>
            </div>

            <Button variant="purple" size="sm" onClick={handleSend} disabled={isLoading || !currentReq} className="ml-2 min-w-24">
              {isLoading ? 'Executing...' : 'Execute'}
            </Button>
          </>
        }

        mainContent={(splitMode) => (
          currentReq ? (
            <div className={`w-full mx-auto pb-24 space-y-10 ${splitMode === 'horizontal' ? 'max-w-360' : 'max-w-5xl'}`}>
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                    <span className="opacity-50">#</span> Request_Metadata
                  </h3>
                  {currentReq.hitCount !== undefined && currentReq.hitCount > 0 && (
                    <span className="text-[9px] font-mono text-zinc-500 uppercase tracking-widest bg-zinc-950 px-2 py-0.5 rounded border border-zinc-800">
                      Hits: <span className="text-emerald-text font-bold">{currentReq.hitCount}</span>
                    </span>
                  )}
                </div>
                <div className="flex flex-col gap-4">
                  <div className={`grid ${simpleMode ? 'grid-cols-1' : 'grid-cols-2'} gap-4`}>
                    <div>
                      <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest block mb-1.5">Request Name</label>
                      <input
                        value={editName}
                        onChange={(e) => { setEditName(e.target.value); debouncedUpdateName(currentReq.id, e.target.value); }}
                        className="w-full bg-zinc-950 border border-zinc-700 px-3 py-2 rounded text-zinc-300 text-[11px] font-mono focus:border-purple-500 outline-none transition-colors"
                      />
                    </div>
                    {!simpleMode && (
                      <div>
                        <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest block mb-1.5">Collection Assignment</label>
                        <div className="flex gap-2">
                          <Select
                            value={editGroupId || 'null'}
                            onChange={(val) => {
                              const newGroupId = val === 'null' ? null : val;
                              setEditGroupId(newGroupId);
                              updateRequest(currentReq.id, { groupId: newGroupId });
                            }}
                            options={[
                              { value: 'null', label: 'Default (Uncategorized)' },
                              ...repeaterGroups.map(g => ({ value: g.id, label: g.name }))
                            ]}
                            className="flex-1"
                          />
                          <Button
                            variant="secondary"
                            size="sm"
                            onClick={async () => {
                              const name = await prompt('New Collection', 'Enter collection name:');
                              if (name) {
                                const newId = await createGroup(name);
                                if (newId) { setEditGroupId(newId); updateRequest(currentReq.id, { groupId: newId }); }
                              }
                            }}
                            className="text-purple-400"
                          >
                            + New
                          </Button>
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Request & Collection Extractions */}
                  {!simpleMode && (
                    <div className="space-y-3">
                      <div>
                        <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest block mb-1.5">Request Extractions</label>
                        <button
                          onClick={() => setExtractionModalOpen(true)}
                          className="w-full bg-zinc-950 border border-zinc-700 px-3 py-2 rounded text-amber-400 text-[11px] font-mono text-left hover:border-amber-500 transition-colors truncate"
                        >
                          {Object.keys(editExtract).length > 0 ? `${Object.keys(editExtract).length} Rules Configured` : 'Configure Request Rules...'}
                        </button>
                      </div>
                      {currentReq?.groupId && (
                        <div>
                          <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest block mb-1.5">Collection Extractions</label>
                          <button
                            onClick={() => setGroupExtractionModalOpen(true)}
                            className="w-full bg-zinc-950 border border-zinc-700 px-3 py-2 rounded text-purple-text text-[11px] font-mono text-left hover:border-purple-border transition-colors truncate"
                          >
                            {Object.keys(activeGroup?.extract || {}).length > 0 ? `${Object.keys(activeGroup?.extract || {}).length} Rules Configured (Collection)` : 'Configure Collection Rules...'}
                          </button>
                        </div>
                      )}
                    </div>
                  )}

                  {/* Markdown Documentation & Testing Notes Section */}
                  <div className="space-y-2 pt-3 border-t border-zinc-800/60">
                    <div className="flex items-center justify-between">
                      <label className="text-[9px] text-purple-400 font-bold uppercase tracking-widest flex items-center gap-1.5">
                        <span>📝 API Documentation & Testing Notes (Markdown)</span>
                      </label>
                      <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
                        <button
                          onClick={() => setDocViewMode('preview')}
                          className={`px-2 py-0.5 text-[9px] font-bold uppercase tracking-wider rounded transition-all ${
                            docViewMode === 'preview' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
                          }`}
                        >
                          Preview
                        </button>
                        <button
                          onClick={() => setDocViewMode('edit')}
                          className={`px-2 py-0.5 text-[9px] font-bold uppercase tracking-wider rounded transition-all ${
                            docViewMode === 'edit' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
                          }`}
                        >
                          Edit Markdown
                        </button>
                      </div>
                    </div>

                    {docViewMode === 'edit' ? (
                      <Textarea
                        value={editDescription}
                        onChange={(e) => {
                          setEditDescription(e.target.value);
                          debouncedUpdateDescription(currentReq.id, e.target.value);
                        }}
                        placeholder="# Request Notes & Test Scenario&#10;&#10;Write Markdown documentation, sample payload examples, parameters, or test instructions here..."
                        className="w-full h-36 font-mono text-[11px] bg-zinc-950/80 border border-zinc-800 p-3 leading-relaxed text-zinc-300"
                      />
                    ) : (
                      <div className="p-3 bg-zinc-950/50 rounded border border-zinc-800/80">
                        <MarkdownViewer content={editDescription} />
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <div className="space-y-3">
                <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Request_Line</h3>
                <UrlEditor 
                  method={editMethod} 
                  onMethodChange={setEditMethod} 
                  url={editUrl} 
                  onChange={(newUrl) => {
                    setEditUrl(newUrl);
                    updateRequest(currentReq.id, { url: newUrl });
                  }} 
                  urlParams={editUrlParams}
                  onUrlParamsChange={(paramsJson) => {
                    setEditUrlParams(paramsJson);
                    updateRequest(currentReq.id, { urlParams: paramsJson });
                  }}
                />
              </div>

              <div className={`grid ${splitMode === 'horizontal' ? 'grid-cols-2 gap-8' : 'grid-cols-1 gap-10'}`}>
                <div className="flex flex-col gap-4">
                  <div className="flex items-center justify-between">
                    <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Outbound_Payload</h3>
                    <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
                      <button onClick={() => setShowPreview(false)} className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${!showPreview ? 'bg-purple-highlight-bg text-purple-text' : 'text-zinc-500 hover:text-zinc-300'}`}>Builder</button>
                      <button onClick={() => setShowPreview(true)} className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${showPreview ? 'bg-purple-highlight-bg text-purple-text' : 'text-zinc-500 hover:text-zinc-300'}`}>Interpolated</button>
                    </div>
                  </div>

                  {!showPreview ? (
                    <div className="flex flex-col gap-8 flex-1">
                      <div className="flex flex-col space-y-3">
                        <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Request_Headers</h3>
                        <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-75"><HeaderEditor initialHeaders={editHeaders} onChange={setEditHeaders} /></div>
                      </div>
                      <div className="flex flex-col space-y-3">
                        <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Request_Body</h3>
                        <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-87.5">
                          <BodyEditor
                            body={editBody}
                            bodyMode={editBodyMode}
                            bodyJson={editBodyJson}
                            bodyUrlencoded={editBodyUrlencoded}
                            bodyMultipart={editBodyMultipart}
                            headers={editHeaders}
                            onChange={(newBody) => {
                              setEditBody(newBody);
                              updateRequest(currentReq.id, { body: newBody });
                            }}
                            onModeChange={(m) => {
                              setEditBodyMode(m);
                              updateRequest(currentReq.id, { bodyMode: m });
                            }}
                            onBodyJsonChange={(val) => {
                              setEditBodyJson(val);
                              updateRequest(currentReq.id, { bodyJson: val });
                            }}
                            onBodyUrlencodedChange={(val) => {
                              setEditBodyUrlencoded(val);
                              updateRequest(currentReq.id, { bodyUrlencoded: val });
                            }}
                            onBodyMultipartChange={(val) => {
                              setEditBodyMultipart(val);
                              updateRequest(currentReq.id, { bodyMultipart: val });
                            }}
                            onHeadersChange={(newHeaders) => {
                              setEditHeaders(newHeaders);
                              updateRequest(currentReq.id, { headers: newHeaders });
                            }}
                          />
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-150 flex flex-col shadow-inner shadow-app-shadow/50"><HttpResponseViewer text={getPreviewRequestText()} /></div>
                  )}
                </div>

                <div className="flex flex-col space-y-3">
                  <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                    <span className="opacity-50">#</span> Inbound_Response
                    {currentReq.response && (
                      <span className="flex items-center gap-2 ml-auto font-mono">
                        <span className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
                          currentReq.response.status >= 200 && currentReq.response.status < 300 ? 'bg-emerald-500/20 text-emerald-400' :
                          currentReq.response.status >= 300 && currentReq.response.status < 400 ? 'bg-sky-500/20 text-sky-400' :
                          currentReq.response.status >= 400 && currentReq.response.status < 500 ? 'bg-amber-500/20 text-amber-400' :
                          'bg-red-500/20 text-red-400'
                        }`}>{currentReq.response.status}</span>
                        {currentReq.response.time !== undefined && currentReq.response.time > 0 && (
                          <span className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
                            currentReq.response.time < 300 ? 'bg-emerald-500/20 text-emerald-400' :
                            currentReq.response.time < 1000 ? 'bg-amber-500/20 text-amber-400' :
                            'bg-red-500/20 text-red-400'
                          }`}>{currentReq.response.time}ms</span>
                        )}
                      </span>
                    )}
                  </h3>
                  <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-150 flex flex-col shadow-inner shadow-app-shadow/50">
                    <HttpResponseViewer text={currentReq.response ? `HTTP/1.1 ${currentReq.response.status}\n${(currentReq.response.headers || []).map(([k, v]) => `${k}: ${v}`).join('\n')}\n\n${currentReq.response.body || ''}` : ''} />
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-zinc-600 space-y-4">
              <span className="text-xs uppercase font-bold tracking-widest">No request selected</span>
              <Button variant="purple" size="sm" onClick={handleAdd}>+ Create Request</Button>
            </div>
          )
        )}
      />

      {currentReq && (
        <ExtractionModal
          isOpen={extractionModalOpen}
          onClose={() => setExtractionModalOpen(false)}
          onSave={(rules) => {
            setEditExtract(rules);
            updateRequest(currentReq.id, { extract: rules });
          }}
          initialRules={editExtract}
          availableVariables={variables}
        />
      )}

      {activeGroup && (
        <ExtractionModal
          isOpen={groupExtractionModalOpen}
          onClose={() => setGroupExtractionModalOpen(false)}
          onSave={(rules) => {
            updateGroupExtractions(activeGroup.id, rules);
          }}
          initialRules={(activeGroup.extract as Record<string, string>) || {}}
          availableVariables={variables}
        />
      )}

      {currentReq && (
        <RepeaterHistoryModal
          isOpen={historyModalOpen}
          onClose={() => setHistoryModalOpen(false)}
          repeaterId={currentReq.id}
          repeaterName={currentReq.name}
        />
      )}

      {activeGroupObj && (
        <CollectionDocModal
          isOpen={groupDocModalOpen}
          groupName={activeGroupObj.name}
          initialDescription={activeGroupObj.description}
          onClose={() => setGroupDocModalOpen(false)}
          onSave={async (desc) => {
            await invoke('update_repeater_group_description', { id: activeGroupObj.id, description: desc });
            await refreshRepeater();
          }}
        />
      )}
    </>
  );
}
