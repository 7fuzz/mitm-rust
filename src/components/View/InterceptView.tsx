import { useState, useEffect } from 'react';
import { TrafficList } from '../Sidebar/TrafficList';
import { HeaderEditor } from '../Editor/HeaderEditor';
import { BodyEditor } from '../Editor/BodyEditor';
import { UrlEditor } from '../Editor/UrlEditor';
import { InterceptTimer } from '../ui/InterceptTimer';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { useTraffic } from '@/hooks/traffic';
import { useNotification } from '../ui/NotificationProvider';
import { Button, Select } from '../ui';
import { invoke } from '@tauri-apps/api/core';

export function InterceptView() {
  const {
    traffic, isIntercepting, interceptMode, ignoredMethods, urlFilter,
    updateConfig, resumeRequest, uiLayout, updateUILayout,
    refreshRepeater, setRepeaterSelectedId, variables, activeEnvId,
    applyAllReplacements,
    simpleMode
  } = useTraffic();

  const { notify } = useNotification();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [localUrlFilter, setLocalUrlFilter] = useState(urlFilter);

  // Sync local state when global state changes (e.g. on load)
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLocalUrlFilter(urlFilter);
  }, [urlFilter]);

  const [editMethod, setEditMethod] = useState('');
  const [editUrl, setEditUrl] = useState('');
  const [editStatusCode, setEditStatusCode] = useState(200);
  const [editHeaders, setEditHeaders] = useState<[string, string][]>([]);
  const [editBody, setEditBody] = useState('');

  const pendingQueue = traffic.filter((t) => t.is_intercepted);
  const currentReq = pendingQueue.find((t) => t.id === selectedId) || pendingQueue[0];
  const isRes = currentReq?.phase === 'response';

  const [prevReqId, setPrevReqId] = useState<string | null>(null);
  const [prevPhase, setPrevPhase] = useState<string | null>(null);

  if (currentReq && (currentReq.id !== prevReqId || currentReq.phase !== prevPhase)) {
    setPrevReqId(currentReq.id);
    setPrevPhase(currentReq.phase);
    
    if (currentReq.phase === 'response') {
      setEditStatusCode(currentReq.status_code || 200);
      setEditHeaders(currentReq.response_headers || []);
    } else {
      setEditMethod(currentReq.method);
      setEditUrl(currentReq.url);
      setEditHeaders(currentReq.request_headers || []);
    }

    const targetBody = currentReq.phase === 'response' ? currentReq.response_body : currentReq.request_body;
    let formattedBody = targetBody || '';
    try {
      if (formattedBody.trim()) {
        const parsed = JSON.parse(formattedBody);
        formattedBody = JSON.stringify(parsed, null, 2);
      }
    } catch { /* ignore */ }
    setEditBody(formattedBody);
  }

  const handleStageToRepeater = async (raw: boolean = false) => {
    if (!currentReq) return;
    try {
      let path = currentReq.url;
      try { path = new URL(currentReq.url).pathname; } catch { }

      const isRaw = simpleMode || raw;

      // Apply replacements only if not raw
      const { url: finalUrl, headers: finalHeaders, body: finalBody } = isRaw 
        ? { url: editUrl || currentReq.url, headers: editHeaders, body: editBody }
        : applyAllReplacements({ url: editUrl || currentReq.url, headers: editHeaders, body: editBody });

      const payload = {
        name: `${currentReq.method} ${path} (Intercept)`,
        method: editMethod || currentReq.method,
        url: finalUrl,
        headers: finalHeaders,
        body: finalBody,
        response: isRes ? {
          status: editStatusCode,
          headers: finalHeaders,
          body: finalBody
        } : undefined
      };

      const itemId = await invoke<string>('create_repeater_item', { item: payload });
      
      if (itemId) {
        if (refreshRepeater) await refreshRepeater();
        if (setRepeaterSelectedId) setRepeaterSelectedId(itemId);
        notify.success(isRaw ? 'Staged Raw to Repeater' : 'Staged in Repeater');
      }
    } catch (error) {
      notify.error(`Failed to stage in Repeater: ${error}`);
    }
  };

  const handleForward = () => {
    if (currentReq) {
      const varDict: Record<string, string> = {};
      variables.filter(v => v.environmentId === activeEnvId).forEach(v => {
        if (v.name.trim()) {
          const activeVal = v.values[v.activeIndex] || v.values[0];
          varDict[v.name.trim()] = activeVal ? activeVal.value : '';
        }
      });

      if (currentReq.phase === 'response') {
        resumeRequest(currentReq.id, {
          status_code: editStatusCode, headers: editHeaders, body: editBody, variables: varDict
        });
      } else {
        resumeRequest(currentReq.id, {
          method: editMethod, url: editUrl, headers: editHeaders, body: editBody, variables: varDict
        });
      }
      setSelectedId(null);
    }
  };

  const handleDrop = () => {
    if (currentReq) {
      resumeRequest(currentReq.id, { drop: true });
      setSelectedId(null);
    }
  };

  const toggleIntercept = () => updateConfig(!isIntercepting, interceptMode, ignoredMethods, urlFilter);

  const toggleMethodIgnore = (method: string) => {
    const newMethods = ignoredMethods.includes(method)
      ? ignoredMethods.filter(m => m !== method)
      : [...ignoredMethods, method];
    updateConfig(isIntercepting, interceptMode, newMethods, urlFilter);
  };

  const handleUrlFilterBlur = () => {
    if (localUrlFilter !== urlFilter) {
      updateConfig(isIntercepting, interceptMode, ignoredMethods, localUrlFilter);
    }
  };

  const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS', 'PATCH'];

  return (
    <WorkspaceLayout
      uiLayout={uiLayout}
      onUpdateLayout={updateUILayout}
      listComponent={() => (
        <TrafficList items={pendingQueue} activeId={currentReq?.id || null} onSelect={setSelectedId} layout="sidebar" />
      )}
      toolbarRight={
        <>
          <Button
            onClick={toggleIntercept}
            variant={isIntercepting ? 'destructive' : 'secondary'}
            size="sm"
            className={isIntercepting ? 'bg-rose-highlight-bg border-rose-highlight-border text-rose-text shadow-[0_0_15px_rgba(244,63,94,0.3)]' : ''}
          >
            {isIntercepting ? 'Intercept_On' : 'Intercept_Off'}
          </Button>

          <Select 
            value={interceptMode} 
            onChange={(val) => updateConfig(isIntercepting, val as "both" | "request" | "response", ignoredMethods, urlFilter)} 
            options={[
              { value: "both", label: "Req & Res" },
              { value: "request", label: "Request Only" },
              { value: "response", label: "Response Only" }
            ]}
            className="w-40 ml-2"
          />
          <div className="w-px h-4 bg-zinc-800 mx-2"></div>
          
          <Button
            variant="destructive"
            size="sm"
            onClick={handleDrop}
            disabled={!currentReq}
            className="min-w-20"
          >
            Drop
          </Button>

          <Button
            variant="primary"
            size="sm"
            onClick={handleForward}
            disabled={!currentReq}
            className="ml-2 min-w-24"
          >
            Forward
          </Button>
        </>
      }
      extraHeader={
        <>
          <div className="flex items-center gap-4 p-3 bg-zinc-900/50">
            <div className="flex items-center gap-1 bg-zinc-950 p-1 rounded border border-zinc-800 w-full max-w-sm">
              <span className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest px-2 whitespace-nowrap">Ignore:</span>
              <div className="flex flex-wrap gap-1">
                {HTTP_METHODS.map(m => (
                  <button
                    key={m} onClick={() => toggleMethodIgnore(m)}
                    className={`text-[9px] uppercase font-bold tracking-widest px-2 py-1 rounded transition-all ${ignoredMethods.includes(m) ? 'bg-rose-500/20 text-rose-400 border border-rose-500/30' : 'bg-transparent text-zinc-600 hover:text-zinc-400 border border-transparent'}`}
                  >
                    {m}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex items-center gap-1 bg-zinc-950 p-1 rounded border border-zinc-800 flex-1">
              <span className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest px-2 whitespace-nowrap">URL Filter:</span>
              <input
                type="text"
                placeholder="Only intercept if URL includes... (e.g. /api/)"
                value={localUrlFilter}
                onChange={(e) => setLocalUrlFilter(e.target.value)}
                onBlur={handleUrlFilterBlur}
                onKeyDown={(e) => e.key === 'Enter' && handleUrlFilterBlur()}
                className="bg-transparent border-none text-[10px] text-zinc-300 w-full outline-none px-2 font-mono"
              />
            </div>
          </div>
          {currentReq && (
            <div className="flex border-t border-zinc-800 shrink-0">
              <div className={`flex-1 px-4 py-1.5 text-[9px] uppercase font-black tracking-[0.3em] flex items-center ${isRes ? 'bg-amber-highlight-bg text-amber-text' : 'bg-sky-highlight-bg text-sky-text'}`}>
                Currently Modifying: {currentReq.phase} Phase
              </div>
              {currentReq.intercepted_at && (
                <div className="border-l border-zinc-800 bg-zinc-900/50">
                  <InterceptTimer key={currentReq.id} startTime={currentReq.intercepted_at} />
                </div>
              )}
            </div>
          )}
        </>
      }
      mainContent={(splitMode) => (
        currentReq ? (
          <div className={`w-full mx-auto pb-24 space-y-10 ${splitMode === 'horizontal' ? 'max-w-360' : 'max-w-5xl'}`}>

            <div className="p-3 bg-zinc-900/50 border border-zinc-800 rounded flex items-center justify-between shadow-inner shadow-app-shadow/20">
              <div className="flex items-center gap-3">
                <span className="text-zinc-500 text-[10px] font-bold uppercase tracking-widest">Target Endpoint:</span>
                <span className={`text-xs font-black ${isRes ? 'text-amber-text' : 'text-emerald-text'}`}>{currentReq.method}</span>
                <span className="text-zinc-300 text-xs font-mono break-all">{currentReq.url}</span>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="purple"
                  size="sm"
                  onClick={() => handleStageToRepeater(false)}
                >
                  Stage_to_Repeater
                </Button>
                
                {!simpleMode && (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => handleStageToRepeater(true)}
                  >
                    Raw
                  </Button>
                )}
              </div>
            </div>

            <div className="space-y-3">
              <h3 className={`${isRes ? 'text-amber-text' : 'text-emerald-text'} font-bold uppercase text-[10px] tracking-widest flex items-center gap-2`}>
                <span className="opacity-50">#</span> 1. {isRes ? 'Response_Status' : 'Request_Line'}
              </h3>

              {isRes ? (
                <div className="flex items-center gap-3">
                  <span className="text-zinc-500 text-xs font-mono">HTTP/2.0</span>
                  <input
                    type="number" value={editStatusCode} onChange={(e) => setEditStatusCode(Number(e.target.value))}
                    className="w-24 bg-zinc-950 border border-zinc-800 p-3 rounded text-emerald-text font-black outline-none focus:border-amber-500 transition-colors text-sm text-center"
                  />
                </div>
              ) : (
                <div className="w-full">
                  <UrlEditor
                    method={editMethod} onMethodChange={setEditMethod}
                    url={editUrl} onChange={setEditUrl}
                  />
                </div>
              )}
            </div>

            <div className={`grid ${splitMode === 'horizontal' ? 'grid-cols-2 gap-8' : 'grid-cols-1 gap-10'}`}>
              <div className="flex flex-col space-y-3">
                <h3 className={`${isRes ? 'text-amber-text' : 'text-sky-text'} font-bold uppercase text-[10px] tracking-widest flex items-center gap-2`}>
                  <span className="opacity-50">#</span> 2. {isRes ? 'Response_Headers' : 'Request_Headers'}
                </h3>
                <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-75">
                  <HeaderEditor initialHeaders={isRes ? (currentReq.response_headers || {}) : (currentReq.request_headers || {})} onChange={setEditHeaders} />
                </div>
              </div>

              <div className="flex flex-col space-y-3">
                <h3 className={`${isRes ? 'text-amber-text' : 'text-sky-text'} font-bold uppercase text-[10px] tracking-widest flex items-center gap-2`}>
                  <span className="opacity-50">#</span> 3. {isRes ? 'Response_Body' : 'Request_Body'}
                </h3>
                <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-87.5">
                  <BodyEditor body={editBody} headers={editHeaders} onChange={setEditBody} />
                </div>
              </div>
            </div>

          </div>
        ) : (
          <div className="flex flex-col items-center justify-center opacity-30 grayscale pointer-events-none select-none min-h-[60vh]">
            <div className="text-[60px] font-black tracking-tighter text-zinc-700">MITM_QUEUE_EMPTY</div>
            <div className="text-[10px] uppercase tracking-[0.3em] mt-2">Listening for traffic...</div>
          </div>
        )
      )}
    />
  );
}
