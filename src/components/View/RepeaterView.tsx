import { useState, useCallback, useRef, useEffect } from 'react';
import { HeaderEditor } from '../Editor/HeaderEditor';
import { BodyEditor } from '../Editor/BodyEditor';
import { UrlEditor } from '../Editor/UrlEditor';
import { TrafficList } from '../Sidebar/TrafficList';
import { Traffic } from '@/types/traffic';
import HttpResponseViewer from '../ui/HttpResponseViewer';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { useTraffic } from '@/hooks/traffic';
import { PromptModal, ConfirmModal, ExtractionModal, RepeaterHistoryModal } from '../Modals';
import { Button } from '../ui/Button';

export interface RepeaterRequest {
  id: string; name: string; groupId: string | null; method: string; url: string; headers: Record<string, string>; body: string; timestamp: number;
  extract?: Record<string, string>;
  hitCount?: number;
  response?: { status: number; headers: Record<string, string>; body: string; time?: number; };
}

export function RepeaterView() {
  const {
    repeaterRequests, repeaterGroups, activeGroupId, switchGroup,
    addEmptyRequest, duplicateRequest, updateRequest, deleteRequest,
    createGroup, renameGroup, deleteGroup, reorderRequests,
    variables, activeEnvId, updateVariableAutoValue,
    uiLayout, updateUILayout,
    repeaterSelectedId: selectedId, setRepeaterSelectedId: setSelectedId,
    simpleMode
  } = useTraffic();

  const [isLoading, setIsLoading] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [editName, setEditName] = useState('');
  const [editGroupId, setEditGroupId] = useState<string | null>(null);
  const [editMethod, setEditMethod] = useState('GET');
  const [editUrl, setEditUrl] = useState('');
  const [editHeaders, setEditHeaders] = useState<Record<string, string>>({});
  const [editBody, setEditBody] = useState('');
  const [editExtract, setEditExtract] = useState<Record<string, string>>({});

  // Modals
  const [promptConfig, setPromptConfig] = useState({ isOpen: false, title: '', initialValue: '', action: (_: string) => { } });
  const openPrompt = (title: string, initialValue: string, action: (val: string) => void) => setPromptConfig({ isOpen: true, title, initialValue, action });
  const closePrompt = () => setPromptConfig(prev => ({ ...prev, isOpen: false }));

  const [confirmConfig, setConfirmConfig] = useState({ isOpen: false, title: '', message: '', action: () => { } });
  const openConfirm = (title: string, message: string, action: () => void) => setConfirmConfig({ isOpen: true, title, message, action });

  const [extractionModalOpen, setExtractionModalOpen] = useState(false);
  const [historyModalOpen, setHistoryModalOpen] = useState(false);

  // Debounce for name updates
  const nameDebounceRef = useRef<NodeJS.Timeout | null>(null);
  const debouncedUpdateName = useCallback((id: string, name: string) => {
    if (nameDebounceRef.current) clearTimeout(nameDebounceRef.current);
    nameDebounceRef.current = setTimeout(() => {
      updateRequest(id, { name });
    }, 300);
  }, [updateRequest]);

  const currentReq = repeaterRequests.find(r => r.id === selectedId) || repeaterRequests[0];

  const [prevReqId, setPrevReqId] = useState<string | null>(null);

  // Auto-update selectedId if we defaulted to repeaterRequests[0]
  useEffect(() => {
    if (currentReq && currentReq.id !== selectedId && setSelectedId) {
      setSelectedId(currentReq.id);
    }
  }, [currentReq, selectedId, setSelectedId]);

  if (currentReq && currentReq.id !== prevReqId) {
    setPrevReqId(currentReq.id);
    setEditName(currentReq.name);
    setEditGroupId(currentReq.groupId || null);
    setEditMethod(currentReq.method);
    setEditUrl(currentReq.url);
    setEditHeaders(currentReq.headers || {});
    setEditBody(currentReq.body || '');
    setEditExtract(currentReq.extract || {});
  }

  const trafficMapped: Traffic[] = repeaterRequests.map(req => {
    const groupName = req.groupId ? repeaterGroups.find(g => g.id === req.groupId)?.name : 'Default';
    return { id: req.id, method: req.method, url: req.name, status_code: req.response?.status ?? 0, host: '', phase: 'history', request_headers: {}, response_headers: {}, request_body: '', response_body: '', is_intercepted: false, group: groupName || 'Default', hit_count: req.hitCount };
  });

  const handleAdd = async () => {
    const targetGroup = (activeGroupId !== 'All' && activeGroupId !== 'null') ? activeGroupId : null;
    const newId = await addEmptyRequest(targetGroup);
    if (newId) setSelectedId(newId);
  };

  const handleDuplicate = async () => {
    if (!currentReq) return;
    const newId = await duplicateRequest(currentReq);
    if (newId) setSelectedId(newId);
  };

  const handleSend = async () => {
    if (!currentReq) return;
    setIsLoading(true);
    try {
      const varDict: Record<string, string> = {};
      variables.filter(v => v.environmentId === activeEnvId).forEach(v => {
        if (v.name.trim()) {
          const activeVal = v.values[v.activeIndex] || v.values[0];
          varDict[v.name.trim()] = activeVal ? activeVal.value : '';
        }
      });

      const response = await fetch('/api/repeater', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: currentReq.id, method: editMethod, url: editUrl, headers: editHeaders, body: editBody, variables: varDict }),
      });
      const data = await response.json();
      if (!data.success) return alert('Error: ' + (data.error || 'Unknown error'));

      // --- EXTRACTION LOGIC ---
      if (editExtract && Object.keys(editExtract).length > 0) {
        try {
          const respJson = JSON.parse(data.body);
          Object.entries(editExtract).forEach(([varName, path]) => {
            const value = path.split('.').reduce((obj, key) => obj?.[key], respJson);
            if (value !== undefined) {
              updateVariableAutoValue(varName, String(value));
            }
          });
        } catch {
          console.error("Failed to parse response for extraction");
        }
      }

      await updateRequest(currentReq.id, {
        method: editMethod, url: editUrl, headers: editHeaders, body: editBody, extract: editExtract,
        hitCount: (currentReq.hitCount || 0) + 1,
        response: { status: data.status ?? 0, headers: data.headers || {}, body: data.body || '', time: Date.now() },
      });
    } catch (error) { alert('Error: ' + error); } finally { setIsLoading(false); }
  };

  const getPreviewRequestText = () => {
    const varDict: Record<string, string> = {};
    variables.filter(v => v.environmentId === activeEnvId).forEach(v => {
      if (v.name.trim()) {
        const activeVal = v.values[v.activeIndex] || v.values[0];
        varDict[v.name.trim()] = activeVal ? activeVal.value : '';
      }
    });

    const interpolate = (text: string) => {
      if (!text) return '';
      let result = text.replace(/\{\{([^}]+)\}\}/g, (match, key) => varDict[key.trim()] ?? match);
      result = result.replace(/%7B%7B(.*?)%7D%7D/gi, (match, key) => varDict[decodeURIComponent(key).trim()] ?? match);
      return result;
    };

    const reqUrl = interpolate(editUrl);
    let path = reqUrl;
    let host = '';
    try {
      const parsed = new URL(reqUrl);
      path = parsed.pathname + parsed.search + parsed.hash;
      host = parsed.host;
    } catch { /* Ignore */ }

    let headerStr = `${editMethod} ${path} HTTP/1.1\n`;
    let hasHost = false;

    Object.entries(editHeaders).forEach(([k, v]) => {
      if (k.toLowerCase() === 'host') hasHost = true;
      headerStr += `${interpolate(k)}: ${interpolate(v)}\n`;
    });

    if (host && !hasHost) headerStr += `Host: ${host}\n`;

    let finalBody = editBody;
    if (editBody.startsWith('{') && editBody.includes('"__form_data"')) {
      try {
        const parsed = JSON.parse(editBody);
        if (parsed.__form_data) {
          finalBody = (parsed.__form_data as Array<{ k: string, v: string, type?: string, fileName?: string }>).map((e) => `${interpolate(e.k)}: ${e.type === 'file' ? `[FILE: ${e.fileName}]` : interpolate(e.v)}`).join('\n');
        }
      } catch { /* fallback to raw */ }
    } else {
      finalBody = interpolate(editBody);
    }

    return `${headerStr}\n${finalBody}`;
  };

  const getRawResponseText = () => {
    if (!currentReq?.response) return '';
    const firstLine = `HTTP/1.1 ${currentReq.response.status}`;
    const headerText = Object.entries(currentReq.response.headers).map(([k, v]) => `${k}: ${v}`).join('\n');
    return `${firstLine}\n${headerText}\n\n${currentReq.response.body}`;
  };

  const activeGroupObj = repeaterGroups.find(g => g.id === activeGroupId);

  return (
    <>
      <PromptModal isOpen={promptConfig.isOpen} title={promptConfig.title} initialValue={promptConfig.initialValue} onClose={closePrompt} onSubmit={promptConfig.action} />
      <ConfirmModal
        isOpen={confirmConfig.isOpen} title={confirmConfig.title} message={confirmConfig.message} isDestructive={true} confirmText="Delete"
        onClose={() => setConfirmConfig(prev => ({ ...prev, isOpen: false }))} onConfirm={confirmConfig.action}
      />
      <ExtractionModal
        isOpen={extractionModalOpen}
        onClose={() => setExtractionModalOpen(false)}
        onSave={(rules) => {
          setEditExtract(rules);
          updateRequest(currentReq.id, { extract: rules });
        }}
        initialRules={editExtract}
        availableVariables={variables.filter(v => v.environmentId === activeEnvId)}
      />
      <RepeaterHistoryModal
        isOpen={historyModalOpen}
        onClose={() => setHistoryModalOpen(false)}
        repeaterId={currentReq?.id || ''}
        repeaterName={currentReq?.name || ''}
      />

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
            <select
              value={activeGroupId}
              onChange={(e) => switchGroup(e.target.value)}
              className="bg-transparent text-purple-400 text-[10px] uppercase font-bold outline-none cursor-pointer min-w-30 max-w-50 truncate"
            >
              <option value="All" className="bg-zinc-900 text-zinc-300">All Groups</option>
              <option value="null" className="bg-zinc-900 text-zinc-300">Default (Uncategorized)</option>
              <option disabled className="bg-zinc-900 text-zinc-600">──────────</option>
              {repeaterGroups.map(g => <option key={g.id} value={g.id} className="bg-zinc-900 text-zinc-300">{g.name}</option>)}
            </select>

            <div className="flex items-center gap-1 border-l border-zinc-800 pl-2 ml-1">
              <button
                onClick={() => activeGroupObj && openPrompt('Rename Collection', activeGroupObj.name, (newName) => renameGroup(activeGroupObj.id, newName))}
                disabled={activeGroupId === 'All' || activeGroupId === 'null'}
                className="p-1 text-zinc-500 hover:text-purple-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors"
                title="Rename Collection"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
              </button>
              <button
                onClick={() => {
                  if (activeGroupObj) {
                    openConfirm(
                      'Delete Collection',
                      `Are you sure you want to delete "${activeGroupObj.name}"? ALL requests inside this collection will be permanently destroyed.`,
                      () => deleteGroup(activeGroupObj.id)
                    );
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
                  <h3 className="text-purple-500 font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
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
                          <select
                            value={editGroupId || 'null'}
                            onChange={(e) => {
                              const newGroupId = e.target.value === 'null' ? null : e.target.value;
                              setEditGroupId(newGroupId);
                              updateRequest(currentReq.id, { groupId: newGroupId });
                            }}
                            className="flex-1 bg-zinc-950 border border-zinc-700 px-3 py-2 rounded text-zinc-300 text-[11px] font-mono focus:border-purple-500 outline-none transition-colors cursor-pointer"
                          >
                            <option value="null">Default (Uncategorized)</option>
                            {repeaterGroups.map(g => <option key={g.id} value={g.id}>{g.name}</option>)}
                          </select>
                          <Button
                            variant="secondary"
                            size="sm"
                            onClick={() => openPrompt('New Collection Name', '', async (name) => {
                              const newId = await createGroup(name);
                              if (newId) { setEditGroupId(newId); updateRequest(currentReq.id, { groupId: newId }); }
                            })}
                            className="text-purple-400"
                          >
                            + New
                          </Button>
                        </div>
                      </div>
                    )}
                  </div>
                  {!simpleMode && (
                    <div>
                      <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest block mb-1.5">Variable Extractions</label>
                      <button
                        onClick={() => setExtractionModalOpen(true)}
                        className="w-full bg-zinc-950 border border-zinc-700 px-3 py-2 rounded text-amber-400 text-[11px] font-mono text-left hover:border-amber-500 transition-colors truncate"
                      >
                        {Object.keys(editExtract).length > 0 ? `${Object.keys(editExtract).length} Rules Configured` : 'Configure Extractions...'}
                      </button>
                    </div>
                  )}
                </div>
              </div>

              <div className="space-y-3">
                <h3 className="text-purple-500 font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Request_Line</h3>
                <UrlEditor method={editMethod} onMethodChange={setEditMethod} url={editUrl} onChange={setEditUrl} />
              </div>

              <div className={`grid ${splitMode === 'horizontal' ? 'grid-cols-2 gap-8' : 'grid-cols-1 gap-10'}`}>
                <div className="flex flex-col gap-4">
                  <div className="flex items-center justify-between">
                    <h3 className="text-purple-500 font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Outbound_Payload</h3>
                    <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
                      <button onClick={() => setShowPreview(false)} className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${!showPreview ? 'bg-purple-600/20 text-purple-400' : 'text-zinc-500 hover:text-zinc-300'}`}>Builder</button>
                      <button onClick={() => setShowPreview(true)} className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${showPreview ? 'bg-purple-600/20 text-purple-400' : 'text-zinc-500 hover:text-zinc-300'}`}>Interpolated</button>
                    </div>
                  </div>

                  {!showPreview ? (
                    <div className="flex flex-col gap-8 flex-1">
                      <div className="flex flex-col space-y-3">
                        <h3 className="text-purple-500 font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Request_Headers</h3>
                        <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-75"><HeaderEditor initialHeaders={editHeaders} onChange={setEditHeaders} /></div>
                      </div>
                      <div className="flex flex-col space-y-3">
                        <h3 className="text-purple-500 font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Request_Body</h3>
                        <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-87.5">
                          <BodyEditor 
                            body={editBody} 
                            headers={editHeaders} 
                            onChange={setEditBody} 
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
                  <div className="flex items-center justify-between">
                    <h3 className="text-amber-500 font-bold uppercase text-[10px] tracking-widest flex items-center gap-2"><span className="opacity-50">#</span> Response_Received</h3>
                    {currentReq.response && (
                      <div className={`px-3 py-1.5 rounded text-[10px] font-black uppercase tracking-widest ${currentReq.response.status >= 400 ? 'bg-rose-500/10 border border-rose-500/30 text-rose-500' : currentReq.response.status >= 300 ? 'bg-amber-500/10 border border-amber-500/30 text-amber-500' : 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-text'}`}>Status: {currentReq.response.status}</div>
                    )}
                  </div>
                  <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-100">
                    {currentReq.response ? <HttpResponseViewer text={getRawResponseText()} /> : <div className="flex items-center justify-center h-full text-zinc-600 text-[10px] uppercase tracking-widest border border-zinc-800 border-dashed rounded">Hit Execute to get a response...</div>}
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center opacity-50 relative z-10 min-h-[60vh]">
              <div className="text-[60px] font-black tracking-tighter text-zinc-700 mb-6">REPEATER_IDLE</div>
              <Button variant="purple" size="lg" onClick={handleAdd}>+ Create New Specification</Button>
            </div>
          )
        )}
      />
    </>
  );
}
