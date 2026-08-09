import { useState, useRef, useMemo, useCallback, useEffect } from 'react';
import { useHotkeys } from '@/hooks/ui/useHotkeys';
import { Traffic } from '@/types/traffic';
import { TrafficList } from '../Sidebar/TrafficList';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { UrlEditor } from '../Editor/UrlEditor';
import HttpResponseViewer from '../ui/HttpResponseViewer';
import { useTraffic } from '@/hooks/traffic';
import { useNotification } from '../ui/NotificationProvider';
import { Button, useDialog } from '../ui';
import { invoke } from '@/lib/utils/tauri';
import { TrafficFilterModal } from '../Modals/TrafficFilterModal';

// === NEW: HTTP Formatters for the Viewer ===
const buildRawRequestMessage = (req: Traffic) => {
  let path = req.url;
  try {
    const parsed = new URL(req.url);
    path = parsed.pathname + parsed.search + parsed.hash;
  } catch { }
  const firstLine = `${req.method} ${path} HTTP/1.1`;
  const headerText = (req.request_headers || []).map(([k, v]) => `${k}: ${v}`).join('\n');
  return `${firstLine}\n${headerText}\n\n${req.request_body || ''}`;
};

const buildRawResponseMessage = (req: Traffic) => {
  const firstLine = `HTTP/1.1 ${req.status_code}`;
  const headerText = (req.response_headers || []).map(([k, v]) => `${k}: ${v}`).join('\n');
  return `${firstLine}\n${headerText}\n\n${req.response_body || ''}`;
};

type HistorySection = 'sidebar' | 'header' | 'url' | 'request' | 'response';

export function HistoryView() {
  const {
    traffic, setTraffic, selectedReq, selectedId, setSelectedId,
    historyLimit, setHistoryLimit, isLimitEnabled, setIsLimitEnabled,
    filterConfig, updateFilterConfig,
    uiLayout, updateUILayout,
    refreshRepeater, setRepeaterSelectedId,
    applyAllReplacements,
    simpleMode, prefs
  } = useTraffic();

  const { notify } = useNotification();
  const { confirm } = useDialog();

  const [localLimit, setLocalLimit] = useState(historyLimit.toString());
  const [prevHistoryLimit, setPrevHistoryLimit] = useState(historyLimit);
  const [activeSection, setActiveSection] = useState<HistorySection>('sidebar');
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false);

  // Refs for navigation
  const headerRef = useRef<HTMLElement>(null);
  const requestRef = useRef<HTMLDivElement>(null);
  const responseRef = useRef<HTMLDivElement>(null);

  if (historyLimit !== prevHistoryLimit) {
    setPrevHistoryLimit(historyLimit);
    setLocalLimit(historyLimit.toString());
  }
  const [headerButtonIndex, setHeaderButtonIndex] = useState(0);

  const handleSelect = useCallback((id: string | null) => {
    setSelectedId(id);
    if (activeSection !== 'sidebar') setActiveSection('sidebar');
  }, [activeSection, setSelectedId]);

  const handleDeleteHistoryRequest = useCallback((id: string) => {
    setTraffic(traffic.filter(t => t.id !== id));
    if (selectedId === id) setSelectedId(null);
  }, [traffic, selectedId, setTraffic, setSelectedId]);

  const handleClearHistory = async () => {
    if (await confirm('Clear History', 'Are you sure you want to permanently delete all intercepted traffic? This action cannot be undone.', true)) {
      try {
        await invoke('clear_history');
        setTraffic([]);
        setSelectedId(null);
        notify.success('History cleared');
      } catch (e) {
        notify.error('Failed to clear history: ' + e);
      }
    }
  };

  useEffect(() => {
    const handler = () => {
      if (activeSection === 'sidebar' && traffic.length > 0) {
        handleClearHistory();
      }
    };
    window.addEventListener('shortcut-clear-history', handler);
    return () => window.removeEventListener('shortcut-clear-history', handler);
  }, [activeSection, traffic.length, handleClearHistory]);

  const handleAddToRepeater = async (req: Traffic, raw: boolean = false) => {
    try {
      let path = req.url;
      try { path = new URL(req.url).pathname; } catch { }

      const isRaw = simpleMode || raw;

      const { url: transformedUrl, headers: transformedHeaders, body: transformedBody } = isRaw
        ? { url: req.url, headers: req.request_headers || [], body: req.request_body || '' }
        : applyAllReplacements({ url: req.url, headers: req.request_headers || [], body: req.request_body || '' });

      // Auto-detect body format from Content-Type header
      const contentType = (transformedHeaders || []).find(([k]) => k.toLowerCase() === 'content-type')?.[1]?.toLowerCase() || '';
      let bodyMode: 'raw' | 'json' | 'urlencoded' | 'multipart' = 'raw';
      let bodyJson: string | undefined;
      let bodyUrlencoded: string | undefined;
      let bodyMultipart: string | undefined;

      if (contentType.includes('application/json') || (transformedBody && (transformedBody.trimStart().startsWith('{') || transformedBody.trimStart().startsWith('[')))) {
        bodyMode = 'json';
        bodyJson = transformedBody;
      } else if (contentType.includes('x-www-form-urlencoded')) {
        bodyMode = 'urlencoded';
        bodyUrlencoded = transformedBody;
      } else if (contentType.includes('multipart/form-data')) {
        bodyMode = 'multipart';
        bodyMultipart = transformedBody;
      }

      const itemPayload = {
        name: `${req.method} ${path}`,
        method: req.method,
        url: transformedUrl,
        headers: transformedHeaders,
        body: transformedBody,
        bodyMode,
        bodyJson,
        bodyUrlencoded,
        bodyMultipart,
        response: req.status_code !== 0 ? {
          status_code: req.status_code,
          response_headers: req.response_headers || [],
          response_body: req.response_body || '',
        } : undefined
      };

      const itemId = await invoke<string>('create_repeater_item', { item: itemPayload });

      if (itemId) {
        if (refreshRepeater) await refreshRepeater();
        if (setRepeaterSelectedId) setRepeaterSelectedId(itemId);
        notify.success(isRaw ? `Staged Raw to Repeater!` : `Staged to Repeater!`);
      }
    } catch (error) {
      notify.error(`Error sending to Repeater: ` + error);
    }
  };

  const copyAsCurl = () => {
    if (!selectedReq) return;
    const curl = `curl -X ${selectedReq.method} '${selectedReq.url}' ${(selectedReq.request_headers || []).map(([k, v]) => `-H '${k}: ${v}'`).join(' ')}`;
    navigator.clipboard.writeText(curl);
  };

  useHotkeys([
    {
      key: 'Enter',
      enabled: activeSection === 'sidebar' && !!selectedId,
      handler: () => {
        setActiveSection('header');
        headerRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      },
    },
    {
      key: 'Enter',
      enabled: activeSection === 'header',
      handler: () => {
        if (headerButtonIndex === 0) handleAddToRepeater(selectedReq!, false);
        else if (headerButtonIndex === 1 && !simpleMode) handleAddToRepeater(selectedReq!, true);
        else if (headerButtonIndex === 2 || (headerButtonIndex === 1 && simpleMode)) copyAsCurl();
      }
    },
    {
      key: 'ArrowLeft',
      enabled: activeSection === 'header',
      handler: () => setHeaderButtonIndex(prev => Math.max(0, prev - 1)),
    },
    {
      key: 'ArrowRight',
      enabled: activeSection === 'header',
      handler: () => setHeaderButtonIndex(prev => Math.min(simpleMode ? 1 : 2, prev + 1)),
    },
    {
      key: 'Escape',
      enabled: activeSection !== 'sidebar',
      handler: () => setActiveSection('sidebar'),
    },
    {
      key: 'ArrowDown',
      enabled: activeSection !== 'sidebar',
      handler: () => {
        if (activeSection === 'header') {
          setActiveSection('url');
        }
        else if (activeSection === 'url') {
          setActiveSection('request');
          requestRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
        else if (activeSection === 'request') {
          setActiveSection('response');
          responseRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      },
    },
    {
      key: 'ArrowUp',
      enabled: activeSection !== 'sidebar',
      handler: () => {
        if (activeSection === 'response') {
          setActiveSection('request');
          requestRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
        } else if (activeSection === 'request') {
          setActiveSection('url');
        } else if (activeSection === 'url') {
          setActiveSection('header');
          headerRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
      },
    }
  ], [activeSection, selectedId, headerButtonIndex, simpleMode, selectedReq, traffic.length, prefs.shortcuts], false);

  const listElement = useMemo(() => (
    <TrafficList
      items={traffic}
      activeId={selectedId}
      isFocused={activeSection === 'sidebar'}
      onSelect={handleSelect}
      onDelete={handleDeleteHistoryRequest}
      layout="sidebar"
    />
  ), [traffic, selectedId, activeSection, handleSelect, handleDeleteHistoryRequest]);

  const handleLimitCommit = () => {
    const val = Number(localLimit);
    if (!isNaN(val) && val > 0) setHistoryLimit(val);
    else setLocalLimit(historyLimit.toString());
  };

  return (
    <>
      <WorkspaceLayout
        uiLayout={uiLayout}
        onUpdateLayout={updateUILayout}
        listComponent={() => listElement}

        toolbarLeft={
          <div className="flex items-center gap-2">
            <Button
              variant={filterConfig.rules.some(r => r.is_active) ? 'primary' : 'secondary'}
              size="sm"
              onClick={() => setIsFilterModalOpen(true)}
              className={filterConfig.rules.some(r => r.is_active) ? 'bg-sky-highlight-bg border-sky-highlight-border text-sky-text' : ''}
            >
              Traffic_Filter
            </Button>
            <div className="w-px h-4 bg-zinc-800 mx-1"></div>
          </div>
        }

        toolbarRight={
          <>
            <Button
              variant={isLimitEnabled ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setIsLimitEnabled(!isLimitEnabled)}
              className={!isLimitEnabled ? 'border-dashed border-zinc-700' : ''}
            >
              {isLimitEnabled ? 'Limit: ON' : 'Limit: OFF'}
            </Button>

            {isLimitEnabled && (
              <div className="flex items-center gap-2 mr-2">
                <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">Max:</span>
                <input
                  type="number"
                  value={localLimit}
                  onChange={(e) => setLocalLimit(e.target.value)}
                  onBlur={handleLimitCommit}
                  onKeyDown={(e) => e.key === 'Enter' && handleLimitCommit()}
                  className="w-16 bg-zinc-950 border border-zinc-700 text-emerald-text text-[10px] font-bold tracking-widest p-1.5 rounded outline-none focus:border-emerald-500 text-center"
                />
              </div>
            )}

            <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest ml-2 mr-2">
              Total: {traffic.length}
            </span>

            {selectedReq?.duration_ms !== undefined && selectedReq.duration_ms > 0 && (
              <span className="text-[10px] text-sky-text font-bold uppercase tracking-widest ml-2 mr-2">
                Latency: {selectedReq.duration_ms}ms
              </span>
            )}

            <Button
              variant="destructive"
              size="sm"
              onClick={handleClearHistory}
            >
              Clear_History
            </Button>
          </>
        }

        mainContent={(splitMode) => (
          selectedReq ? (
            <div className={`w-full mx-auto pb-24 space-y-10 ${splitMode === 'horizontal' ? 'max-w-360' : 'max-w-5xl'}`}>
              <header ref={headerRef} className={`flex flex-col items-start border-b pb-6 transition-all duration-200 scroll-mt-24 ${activeSection === 'header' ? 'border-amber-highlight-border bg-amber-highlight-bg -mx-4 px-4 py-2 rounded-lg' : 'border-zinc-800'}`}>
                <div className="ml-auto flex gap-3 mb-4">
                  <Button
                    variant="purple"
                    size="sm"
                    onClick={() => handleAddToRepeater(selectedReq, false)}
                    className={activeSection === 'header' && headerButtonIndex === 0 ? 'ring-2 ring-purple-500 ring-offset-2 ring-offset-zinc-950 scale-105' : ''}
                  >
                    Send_to_Repeater
                  </Button>

                  {!simpleMode && (
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => handleAddToRepeater(selectedReq, true)}
                      className={activeSection === 'header' && headerButtonIndex === 1 ? 'ring-2 ring-zinc-500 ring-offset-2 ring-offset-zinc-950 scale-105' : ''}
                    >
                      Raw
                    </Button>
                  )}

                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={copyAsCurl}
                    className={`hover:bg-emerald-600 hover:border-emerald-500 ${activeSection === 'header' && (headerButtonIndex === 2 || (headerButtonIndex === 1 && simpleMode)) ? 'ring-2 ring-emerald-highlight-border ring-offset-2 ring-offset-zinc-950 scale-105 bg-btn-primary text-btn-primary-text' : ''}`}
                  >
                    Copy_as_cURL
                  </Button>
                </div>
                <div className={`w-full transition-all ${activeSection === 'url' ? 'ring-2 ring-sky-highlight-border ring-offset-4 ring-offset-zinc-950 rounded' : ''}`}>
                  <UrlEditor method={selectedReq.method} onMethodChange={() => { }} url={selectedReq.url} onChange={() => { }} readOnly={true} />
                </div>
              </header>

              <div className={`grid ${splitMode === 'horizontal' ? 'grid-cols-2 gap-8' : 'grid-cols-1 gap-10'}`}>
                <div ref={requestRef} className={`flex flex-col space-y-3 transition-all p-2 rounded ${activeSection === 'request' ? 'bg-sky-highlight-bg ring-1 ring-sky-highlight-border' : ''}`}>
                  <h3 className={`text-sky-text text-[10px] font-bold uppercase tracking-widest flex items-center gap-2 ${activeSection === 'request' ? 'text-sky-text' : ''}`}>
                    <span className="opacity-50">#</span> Request_Payload
                    {activeSection === 'request' && <span className="ml-auto text-[8px] bg-btn-sky text-btn-sky-text px-1 rounded">ACTIVE</span>}
                  </h3>
                  <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-100">
                    <HttpResponseViewer text={buildRawRequestMessage(selectedReq)} />
                  </div>
                </div>

                <div ref={responseRef} className={`flex flex-col space-y-3 transition-all p-2 rounded ${activeSection === 'response' ? 'bg-amber-highlight-bg ring-1 ring-amber-highlight-border' : ''}`}>
                  <h3 className={`text-amber-text text-[10px] font-bold uppercase tracking-widest flex items-center gap-2 ${activeSection === 'response' ? 'text-amber-text' : ''}`}>
                    <span className="opacity-50">#</span> Response_Payload
                    {activeSection === 'response' && <span className="ml-auto text-[8px] bg-btn-amber text-btn-amber-text px-1 rounded font-black">ACTIVE</span>}
                  </h3>
                  <div className="flex-1 bg-zinc-900/20 border border-zinc-800/50 rounded overflow-hidden min-h-100">
                    {selectedReq.status_code === 0 ? (
                      <div className="h-full flex flex-col items-center justify-center text-amber-400/80 text-[10px] font-mono uppercase tracking-widest gap-3 py-16">
                        <div className="w-5 h-5 border-2 border-amber-400/30 border-t-amber-400 rounded-full animate-spin"></div>
                        <span className="animate-pulse">Awaiting Response...</span>
                      </div>
                    ) : (
                      <HttpResponseViewer text={buildRawResponseMessage(selectedReq)} />
                    )}
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full min-h-[60vh] opacity-20 grayscale pointer-events-none select-none">
              <div className="text-[40px] font-black tracking-tighter">HISTORY_IDLE</div>
            </div>
          )
        )}
      />

      <TrafficFilterModal
        isOpen={isFilterModalOpen}
        onClose={() => setIsFilterModalOpen(false)}
        config={filterConfig}
        onSave={updateFilterConfig}
      />
    </>
  );
}
