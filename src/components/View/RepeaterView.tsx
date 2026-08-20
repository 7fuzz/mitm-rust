import { useState, useCallback, useRef, useEffect } from 'react';
import { HeaderEditor } from '../Editor/HeaderEditor';
import { BodyEditor } from '../Editor/BodyEditor';
import { UrlEditor } from '../Editor/UrlEditor';
import { RepeaterSidebarTree } from '../Sidebar/RepeaterSidebarTree';
import { Traffic } from '@/types/traffic';
import HttpResponseViewer from '../ui/HttpResponseViewer';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { useTraffic, RepeaterRequest } from '@/hooks/traffic';
import { useNotification } from '../ui/NotificationProvider';
import { ExtractionModal, RepeaterHistoryModal, CollectionDocModal, CurlImportModal } from '../Modals';
import { useDialog } from '../ui';
import { invoke } from '@/lib/utils/tauri';
import { RepeaterWebhookModal } from '../Repeater/RepeaterWebhookModal';
import { RepeaterToolbarLeft, RepeaterToolbarRight } from '../Repeater/RepeaterToolbar';
import { MarkdownViewer } from '../ui/MarkdownViewer';
import { buildCurlCommand, interpolateVariables } from '@/lib/utils/interpolation';

export function RepeaterView() {
  const { notify } = useNotification();
  const { confirm, prompt } = useDialog();
  const {
    repeaterRequests,
    repeaterGroups,
    activeGroupId,
    addEmptyRequest,
    createGroup,
    duplicateRequest,
    createFromCurl,
    updateRequest,
    deleteRequest,
    deleteGroup,
    renameGroup,
    variables,
    activeEnvId,
    updateVariableAutoValue,
    uiLayout,
    updateUILayout,
    repeaterSelectedId: selectedId,
    setRepeaterSelectedId: setSelectedId,
    _setRawRepeater,
    updateGroupExtractions,
    refreshRepeater,
    clearUncategorizedRequests,
    clearGroupRequests,
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
  const [curlModalOpen, setCurlModalOpen] = useState(false);
  const [webhookModalOpen, setWebhookModalOpen] = useState(false);
  const [showNewMenu, setShowNewMenu] = useState(false);

  // Debounce for name updates
  const nameDebounceRef = useRef<NodeJS.Timeout | null>(null);
  const debouncedUpdateName = useCallback(
    (id: string, name: string) => {
      if (nameDebounceRef.current) clearTimeout(nameDebounceRef.current);
      nameDebounceRef.current = setTimeout(() => {
        updateRequest(id, { name });
      }, 300);
    },
    [updateRequest]
  );

  // Debounce for description updates
  const descDebounceRef = useRef<NodeJS.Timeout | null>(null);
  const debouncedUpdateDescription = useCallback(
    (id: string, description: string) => {
      if (descDebounceRef.current) clearTimeout(descDebounceRef.current);
      descDebounceRef.current = setTimeout(() => {
        updateRequest(id, { description });
      }, 300);
    },
    [updateRequest]
  );

  const currentReq = repeaterRequests.find((r) => r.id === selectedId) || repeaterRequests[0] || null;
  const activeGroupObj = repeaterGroups.find((g) => g.id === (currentReq?.groupId || activeGroupId));
  const activeGroup = activeGroupObj;

  useEffect(() => {
    if (!repeaterRequests.length) {
      if (selectedId && setSelectedId) setSelectedId(null);
      return;
    }

    if (!selectedId || !repeaterRequests.some((r) => r.id === selectedId)) {
      setSelectedId(repeaterRequests[0].id);
    }
  }, [repeaterRequests, selectedId, setSelectedId]);

  // Auto-update selectedId if defaulted to first request
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


  const handleAdd = async () => {
    const defaultGroup = activeGroupId === 'All' || activeGroupId === 'null' ? null : activeGroupId;
    const newId = await addEmptyRequest(defaultGroup);
    if (newId) setSelectedId(newId);
  };

  const handleImportCurl = async (curl: string) => {
    const defaultGroup = activeGroupId === 'All' || activeGroupId === 'null' ? null : activeGroupId;
    const newId = await createFromCurl(curl, defaultGroup);
    if (newId) setSelectedId(newId);
  };

  const handleCopyAsCurl = () => {
    if (!currentReq) return;
    const command = buildCurlCommand(editMethod, editUrl, editHeaders, editBody, variables, activeEnvId);
    navigator.clipboard.writeText(command);
    notify.success('cURL command copied to clipboard (variables filled)');
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
        description: editDescription,
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
        description: editDescription,
      });

      const currentEnvVars = variables.filter((v) => v.environmentId === activeEnvId);
      const varMap: Record<string, string> = {};
      currentEnvVars.forEach((v) => {
        const val = v.values[v.activeIndex]?.value ?? v.values[0]?.value ?? '';
        varMap[v.name] = val;
      });

      const response = await invoke<Traffic>('execute_repeater_request', {
        id: currentReq.id,
      });

      const updatedWithRes: RepeaterRequest = {
        ...updatedReq,
        response: {
          status: response.status_code,
          headers: response.response_headers,
          body: response.response_body,
          time: response.duration_ms || undefined,
        },
        hitCount: (currentReq.hitCount || 0) + 1,
      };

      _setRawRepeater((prev: RepeaterRequest[]) =>
        prev.map((r: RepeaterRequest) => (r.id === currentReq.id ? updatedWithRes : r))
      );

      // Merge extraction rules hierarchically: Root Collection -> Subfolder -> Direct Folder -> Request
      let folderChainExtract: Record<string, string> = {};
      if (currentReq.groupId) {
        const folderChain: Record<string, string>[] = [];
        let currentGroupId: string | null | undefined = currentReq.groupId;
        while (currentGroupId) {
          const g = repeaterGroups.find((grp) => grp.id === currentGroupId);
          if (!g) break;
          if (g.extract) {
            folderChain.unshift(g.extract as Record<string, string>);
          }
          currentGroupId = g.parentId;
        }
        for (const ext of folderChain) {
          folderChainExtract = { ...folderChainExtract, ...ext };
        }
      }

      const mergedExtract = { ...folderChainExtract, ...editExtract };

      if (mergedExtract && Object.keys(mergedExtract).length > 0) {
        try {
          let respJson: any = null;
          try {
            respJson = JSON.parse(response.response_body);
          } catch {
            respJson = null;
          }

          Object.entries(mergedExtract).forEach(([varName, rawPath]) => {
            if (typeof rawPath !== 'string' || !rawPath.trim()) return;
            let p = rawPath.trim();

            // 1. Header Extraction: e.g. "header:X-Auth-Token" or "header:Authorization"
            if (p.toLowerCase().startsWith('header:')) {
              const headerName = p.substring(7).trim().toLowerCase();
              if (response.response_headers) {
                const foundHeader = Object.entries(response.response_headers).find(
                  ([hk]) => hk.toLowerCase() === headerName
                );
                if (foundHeader && foundHeader[1] !== undefined) {
                  updateVariableAutoValue(varName, String(foundHeader[1]));
                }
              }
              return;
            }

            // 2. JSON / Body Extraction
            if (respJson) {
              if (p.toLowerCase().startsWith('json:')) {
                p = p.substring(5).trim();
              } else if (p.toLowerCase().startsWith('body:')) {
                p = p.substring(5).trim();
              }

              p = p.replace(/^\$\.?/, '');
              if (!p) return;

              // Convert array index notation data[0].id -> data.0.id
              p = p.replace(/\[(\d+)\]/g, '.$1');

              const keys = p.split('.').filter(Boolean);
              let val = respJson;
              for (const k of keys) {
                if (val === null || val === undefined) {
                  val = undefined;
                  break;
                }
                val = val[k];
              }

              if (val !== undefined && val !== null) {
                updateVariableAutoValue(varName, typeof val === 'object' ? JSON.stringify(val) : String(val));
              }
            }
          });
        } catch (e) {
          console.error('Failed to parse response for extraction:', e);
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
    const headersText = editHeaders.map(([k, v]) => `${interpolateVariables(k, variables, activeEnvId)}: ${interpolateVariables(v, variables, activeEnvId)}`).join('\n');
    return `${editMethod} ${interpolateVariables(editUrl, variables, activeEnvId)} HTTP/1.1\n${headersText}\n\n${interpolateVariables(editBody, variables, activeEnvId)}`;
  };


  return (
    <>
      <WorkspaceLayout
        uiLayout={uiLayout}
        onUpdateLayout={updateUILayout}
        listComponent={() => (
          <RepeaterSidebarTree
            repeaterGroups={repeaterGroups}
            repeaterRequests={repeaterRequests}
            activeId={selectedId}
            onSelectRequest={setSelectedId}
            onDeleteRequest={deleteRequest}
            onCreateRequest={(groupId) => addEmptyRequest(groupId)}
            onCreateGroup={(name, parentId) => createGroup(name, parentId)}
            onRenameGroup={async (g) => {
              const newName = await prompt('Rename Collection / Folder', 'Enter new name:', g.name);
              if (newName) renameGroup(g.id, newName);
            }}
            onDeleteGroup={async (g) => {
              if (await confirm('Delete Folder', `Are you sure you want to delete "${g.name}" and all requests/subfolders inside?`, true)) {
                deleteGroup(g.id);
              }
            }}
            onOpenDocModal={() => setGroupDocModalOpen(true)}
            onOpenExtractionModal={() => setGroupExtractionModalOpen(true)}
            openPrompt={async (title, initialValue, action) => {
              const val = await prompt(title, 'Enter name:', initialValue);
              if (val) action(val);
            }}
            onClearUncategorized={clearUncategorizedRequests}
            onClearGroupRequests={(g) => clearGroupRequests(g.id)}
            openConfirm={async (title, message, action) => {
              if (await confirm(title, message, true)) {
                action();
              }
            }}
          />
        )}
        toolbarLeft={<RepeaterToolbarLeft />}
        toolbarRight={
          <RepeaterToolbarRight
            hasCurrentReq={!!currentReq}
            hasCurrentResponse={!!currentReq?.response}
            isLoading={isLoading}
            showNewMenu={showNewMenu}
            setShowNewMenu={setShowNewMenu}
            onOpenHistoryModal={() => setHistoryModalOpen(true)}
            onClearResponse={() => currentReq && updateRequest(currentReq.id, { response: undefined })}
            onAddEmptyRequest={handleAdd}
            onOpenCurlModal={() => setCurlModalOpen(true)}
            onOpenWebhookModal={() => setWebhookModalOpen(true)}
            onDuplicateRequest={handleDuplicate}
            onCopyAsCurl={handleCopyAsCurl}
            onExecute={handleSend}
          />
        }
        mainContent={(splitMode) =>
          currentReq ? (
            <div className={`w-full mx-auto pb-24 space-y-8 ${splitMode === 'vertical' ? 'max-w-[1800px]' : 'max-w-5xl'}`}>
              {/* TOP SECTION: Always Full Width (Metadata & Target Endpoint) */}
              <div className="space-y-6">
                {/* Request Metadata */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                      <span className="opacity-50">#</span> Request_Metadata
                    </h3>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => setExtractionModalOpen(true)}
                        className="px-2 py-0.5 text-[9px] font-bold text-amber-400 bg-amber-500/10 border border-amber-500/30 rounded hover:bg-amber-500/20 transition-all flex items-center gap-1 cursor-pointer"
                        title="Configure Auto Extraction Rules for this Request (Extract response values into variables)"
                      >
                        <span>⚡ Request Auto Extract</span>
                      </button>
                      {currentReq.hitCount !== undefined && currentReq.hitCount > 0 && (
                        <span className="text-[9px] font-mono text-zinc-500 uppercase tracking-widest bg-zinc-950 px-2 py-0.5 rounded border border-zinc-800">
                          Hits: <strong className="text-zinc-300 font-bold">{currentReq.hitCount}</strong>
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Request Name + Collection */}
                  <div className="flex items-center gap-2">
                    <input
                      type="text"
                      value={editName}
                      onChange={(e) => {
                        setEditName(e.target.value);
                        if (currentReq) debouncedUpdateName(currentReq.id, e.target.value);
                      }}
                      placeholder="Request Name"
                      className="flex-1 bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-purple-500 font-bold"
                    />
                    <select
                      value={editGroupId || 'null'}
                      onChange={(e) => {
                        const gid = e.target.value === 'null' ? null : e.target.value;
                        setEditGroupId(gid);
                        if (currentReq) updateRequest(currentReq.id, { groupId: gid });
                      }}
                      className="bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-[10px] text-zinc-400 focus:outline-none focus:border-purple-500 cursor-pointer max-w-[180px]"
                      title="Move request to collection"
                    >
                      <option value="null">Default (Uncategorized)</option>
                      {repeaterGroups.map((g) => (
                        <option key={g.id} value={g.id}>{g.name}</option>
                      ))}
                    </select>
                  </div>

                  {/* Request Description / Docs */}
                  {(editDescription || docViewMode === 'edit') && (
                    <div className="space-y-1">
                      <div className="flex items-center justify-between">
                        <span className="text-[9px] font-bold text-zinc-600 uppercase tracking-widest">Request Docs</span>
                        <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
                          <button
                            onClick={() => setDocViewMode('preview')}
                            className={`px-2 py-0.5 text-[9px] font-bold uppercase tracking-widest rounded transition-all ${
                              docViewMode === 'preview' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
                            }`}
                          >Preview</button>
                          <button
                            onClick={() => setDocViewMode('edit')}
                            className={`px-2 py-0.5 text-[9px] font-bold uppercase tracking-widest rounded transition-all ${
                              docViewMode === 'edit' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
                            }`}
                          >Edit</button>
                        </div>
                      </div>
                      {docViewMode === 'preview' ? (
                        <MarkdownViewer
                          content={editDescription || '_No documentation yet. Click to add._'}
                          collapsible
                          maxCollapsedHeight="max-h-32"
                        />
                      ) : (
                        <textarea
                          value={editDescription}
                          onChange={(e) => {
                            setEditDescription(e.target.value);
                            if (currentReq) debouncedUpdateDescription(currentReq.id, e.target.value);
                          }}
                          placeholder="Request notes / docs (Markdown supported)..."
                          rows={4}
                          autoFocus
                          className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-[11px] text-zinc-300 font-mono focus:outline-none focus:border-purple-500 resize-y placeholder:text-zinc-700"
                        />
                      )}
                    </div>
                  )}
                  {!editDescription && docViewMode !== 'edit' && (
                    <button
                      onClick={() => setDocViewMode('edit')}
                      className="text-[9px] font-bold text-zinc-700 hover:text-purple-400 transition-colors uppercase tracking-widest"
                    >
                      + Add Request Docs
                    </button>
                  )}
                </div>

                {/* Target Endpoint Editor */}
                <div className="space-y-3">
                  <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                    <span className="opacity-50">#</span> Target_Endpoint
                  </h3>
                  <UrlEditor
                    method={editMethod}
                    onMethodChange={(m) => {
                      setEditMethod(m);
                      if (currentReq) updateRequest(currentReq.id, { method: m });
                    }}
                    url={editUrl}
                    onChange={(u) => {
                      setEditUrl(u);
                      if (currentReq) updateRequest(currentReq.id, { url: u });
                    }}
                    urlParams={editUrlParams}
                    onUrlParamsChange={(p) => {
                      setEditUrlParams(p);
                      if (currentReq) updateRequest(currentReq.id, { urlParams: p });
                    }}
                  />
                </div>
              </div>

              {/* BOTTOM SECTION: Split Request Payload vs Response Viewer */}
              <div className={`w-full ${splitMode === 'vertical' ? 'flex flex-col lg:flex-row gap-6' : 'flex flex-col gap-8'}`}>
                {/* Left/Top: Outbound Payload with Builder/Interpolated tabs */}
                <div className={`flex flex-col gap-6 ${splitMode === 'vertical' ? 'w-full lg:w-1/2 min-w-0' : 'w-full'}`}>
                  {/* Outbound Payload Header with Builder / Interpolated toggle */}
                  <div className="space-y-3">
                    <div className="flex items-center justify-between">
                      <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                        <span className="opacity-50">#</span> Outbound_Payload
                      </h3>
                      <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
                        <button
                          onClick={() => setShowPreview(false)}
                          className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${
                            !showPreview ? 'bg-purple-highlight-bg text-purple-text' : 'text-zinc-500 hover:text-zinc-300'
                          }`}
                        >Builder</button>
                        <button
                          onClick={() => setShowPreview(true)}
                          className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${
                            showPreview ? 'bg-purple-highlight-bg text-purple-text' : 'text-zinc-500 hover:text-zinc-300'
                          }`}
                        >Interpolated</button>
                      </div>
                    </div>

                    {!showPreview ? (
                      <div className="flex flex-col gap-6">
                        {/* Headers Editor */}
                        <div className="flex flex-col space-y-3">
                          <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                            <span className="opacity-50">#</span> Request_Headers
                          </h3>
                          <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-[300px]">
                            <HeaderEditor
                              initialHeaders={editHeaders}
                              onChange={(h) => {
                                setEditHeaders(h);
                                if (currentReq) updateRequest(currentReq.id, { headers: h });
                              }}
                            />
                          </div>
                        </div>

                        {/* Body Editor */}
                        <div className="flex flex-col space-y-3">
                          <h3 className="text-purple-text font-bold uppercase text-[10px] tracking-widest flex items-center gap-2">
                            <span className="opacity-50">#</span> Request_Body
                          </h3>
                          <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-[350px]">
                            <BodyEditor
                              body={editBody}
                              bodyMode={editBodyMode}
                              bodyJson={editBodyJson}
                              bodyUrlencoded={editBodyUrlencoded}
                              bodyMultipart={editBodyMultipart}
                              headers={editHeaders}
                              onChange={(b) => {
                                setEditBody(b);
                                if (currentReq) updateRequest(currentReq.id, { body: b });
                              }}
                              onModeChange={(m) => {
                                setEditBodyMode(m);
                                if (currentReq) updateRequest(currentReq.id, { bodyMode: m });
                              }}
                              onBodyJsonChange={(j) => {
                                setEditBodyJson(j);
                                if (currentReq) updateRequest(currentReq.id, { bodyJson: j });
                              }}
                              onBodyUrlencodedChange={(u) => {
                                setEditBodyUrlencoded(u);
                                if (currentReq) updateRequest(currentReq.id, { bodyUrlencoded: u });
                              }}
                              onBodyMultipartChange={(m) => {
                                setEditBodyMultipart(m);
                                if (currentReq) updateRequest(currentReq.id, { bodyMultipart: m });
                              }}
                            />
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-[600px] flex flex-col shadow-inner shadow-app-shadow/50">
                        <HttpResponseViewer text={getPreviewRequestText()} />
                      </div>
                    )}
                  </div>
                </div>

                {/* Right/Bottom: Inbound Response */}
                <div className={`flex flex-col space-y-3 ${splitMode === 'vertical' ? 'w-full lg:w-1/2 min-w-0 border-t lg:border-t-0 lg:border-l border-zinc-800 lg:pl-6 pt-6 lg:pt-0' : 'w-full pt-6 border-t border-zinc-800'}`}>
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
                  <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-[600px] flex flex-col shadow-inner shadow-app-shadow/50">
                    <HttpResponseViewer text={currentReq.response
                      ? `HTTP/1.1 ${currentReq.response.status}\n${(currentReq.response.headers || []).map(([k, v]) => `${k}: ${v}`).join('\n')}\n\n${currentReq.response.body || ''}`
                      : ''
                    } />
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="h-full flex items-center justify-center text-zinc-500 text-xs">
              Select or create a request to begin testing in Repeater.
            </div>
          )
        }
      />

      {/* Modals */}
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

      <CurlImportModal
        isOpen={curlModalOpen}
        onClose={() => setCurlModalOpen(false)}
        onSubmit={handleImportCurl}
      />

      <RepeaterWebhookModal
        isOpen={webhookModalOpen}
        activeGroupId={activeGroupId}
        onClose={() => setWebhookModalOpen(false)}
        onSuccess={async (newId) => {
          await refreshRepeater();
          if (newId) setSelectedId(newId);
        }}
      />
    </>
  );
}
