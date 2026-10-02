import React, { useState, useRef, useEffect, useMemo } from 'react';
import type { TrafficItem } from '../../../types';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl } from '../../common/ui';
import { PaneHeader } from '../../common/PaneHeader';
import { MediaResponsePreview } from '../../common/MediaResponsePreview';
import { JsonTreeViewerRoot } from '../../common/JsonTreeViewer';
import { detectMediaResponse } from '../../../utils/mediaDetector';
import { isJsonString } from '../../../utils/bodyConverters';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { clampPercent, startDragResize } from '../../../utils/dragResize';

const REQ_TABS = [
  { value: 'body', label: 'Body' },
  { value: 'headers', label: 'Headers' },
  { value: 'params', label: 'Params' },
  { value: 'cookies', label: 'Cookies' },
] as const;

const RES_TABS = [
  { value: 'body', label: 'Body' },
  { value: 'headers', label: 'Headers' },
  { value: 'cookies', label: 'Cookies' },
] as const;

interface RequestResponseInspectorProps {
  item: TrafficItem | null;
  /** Hides the inspector; selecting a row opens it again */
  onHide?: () => void;
}

export const RequestResponseInspector: React.FC<RequestResponseInspectorProps> = ({
  item,
  onHide,
}) => {
  const [reqTab, setReqTab] = useState<'headers' | 'body' | 'params' | 'cookies'>('body');
  const [resTab, setResTab] = useState<'headers' | 'body' | 'cookies'>('body');

  const [reqTreeSearch, setReqTreeSearch] = useState('');
  const [reqTreeFilterMode, setReqTreeFilterMode] = useState(false);
  const [resTreeSearch, setResTreeSearch] = useState('');
  const [resTreeFilterMode, setResTreeFilterMode] = useState(false);

  const reqMediaInfo = useMemo(() => {
    if (!item) return null;
    return detectMediaResponse(item.requestBody, item.requestHeaders);
  }, [item?.requestBody, item?.requestHeaders]);

  const resMediaInfo = useMemo(() => {
    if (!item) return null;
    return detectMediaResponse(item.responseBody, item.responseHeaders);
  }, [item?.responseBody, item?.responseHeaders]);

  const isReqJson = useMemo(() => {
    if (!item?.requestBody) return false;
    const hasJsonHeader = (item.requestHeaders || []).some(
      (h) => h.key?.toLowerCase() === 'content-type' && h.value?.toLowerCase().includes('json')
    );
    if (hasJsonHeader) return true;
    return isJsonString(item.requestBody);
  }, [item?.requestBody, item?.requestHeaders]);

  const isResJson = useMemo(() => {
    if (!item?.responseBody) return false;
    const hasJsonHeader = (item.responseHeaders || []).some(
      (h) => h.key?.toLowerCase() === 'content-type' && h.value?.toLowerCase().includes('json')
    );
    if (hasJsonHeader) return true;
    return isJsonString(item.responseBody);
  }, [item?.responseBody, item?.responseHeaders]);

  // Independent body format state for Request vs Response
  const [reqBodyFormat, setReqBodyFormat] = useState<string>('pretty');
  const [resBodyFormat, setResBodyFormat] = useState<string>('pretty');

  // Auto-switch to preview if media is detected on response
  useEffect(() => {
    if (resMediaInfo) {
      setResBodyFormat('preview');
    } else if (resBodyFormat === 'tree' && !isResJson) {
      setResBodyFormat('pretty');
    }
  }, [resMediaInfo, isResJson, item?.id]);

  useEffect(() => {
    if (reqMediaInfo) {
      setReqBodyFormat('preview');
    } else if (reqBodyFormat === 'tree' && !isReqJson) {
      setReqBodyFormat('pretty');
    }
  }, [reqMediaInfo, isReqJson, item?.id]);

  const reqBodyFormats = useMemo(() => {
    const list: Array<{ value: string; label: string }> = [];
    if (reqMediaInfo) {
      list.push({ value: 'preview', label: `Preview (${reqMediaInfo.previewType.toUpperCase()})` });
    }
    list.push({ value: 'pretty', label: 'Pretty' });
    if (isReqJson) {
      list.push({ value: 'tree', label: 'Tree' });
    }
    list.push(
      { value: 'raw', label: 'Raw' },
      { value: 'hex', label: 'Hex' }
    );
    return list;
  }, [reqMediaInfo, isReqJson]);

  const resBodyFormats = useMemo(() => {
    const list: Array<{ value: string; label: string }> = [];
    if (resMediaInfo) {
      list.push({ value: 'preview', label: `Preview (${resMediaInfo.previewType.toUpperCase()})` });
    }
    list.push({ value: 'pretty', label: 'Pretty' });
    if (isResJson) {
      list.push({ value: 'tree', label: 'Tree' });
    }
    list.push(
      { value: 'raw', label: 'Raw' },
      { value: 'hex', label: 'Hex' },
      { value: 'html', label: 'HTML' }
    );
    return list;
  }, [resMediaInfo, isResJson]);

  // Draggable width/height split between Request and Response panels
  const [reqWidthPercent, setReqWidthPercent, resetReqWidthPercent] = useUiPref('history.inspectorSplitPercent');
  const splitContainerRef = useRef<HTMLDivElement>(null);
  const reqPaneRef = useRef<HTMLDivElement>(null);
  const resPaneRef = useRef<HTMLDivElement>(null);

  const handleSplitPointerDown = (e: React.PointerEvent) => {
    let percent = reqWidthPercent;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!splitContainerRef.current || !reqPaneRef.current || !resPaneRef.current) return;
        const rect = splitContainerRef.current.getBoundingClientRect();
        percent = clampPercent(((ev.clientX - rect.left) / rect.width) * 100);
        reqPaneRef.current.style.width = `${percent}%`;
        resPaneRef.current.style.width = `${100 - percent}%`;
      },
      onEnd: () => setReqWidthPercent(percent),
    });
  };

  // Parse params from URL
  const parsedParams = React.useMemo(() => {
    if (!item || !item.url) return [];
    try {
      const urlObj = new URL(item.url);
      const list: Array<{ id: string; key: string; value: string; enabled: boolean }> = [];
      urlObj.searchParams.forEach((val, key) => {
        list.push({ id: key, key, value: val, enabled: true });
      });
      return list;
    } catch (e) {
      return [];
    }
  }, [item?.url]);

  // Parse cookies from headers
  const reqCookies = React.useMemo(() => {
    if (!item || !item.requestHeaders) return [];
    const cookieHeader = (item.requestHeaders || []).find((h) => h.key?.toLowerCase() === 'cookie')?.value || '';
    if (!cookieHeader) return [];
    return cookieHeader.split(';').map((pair, idx) => {
      const [k, v] = pair.trim().split('=');
      return { id: `c-${idx}`, key: k || '', value: v || '', enabled: true };
    });
  }, [item?.requestHeaders]);

  const resCookies = React.useMemo(() => {
    if (!item || !item.responseHeaders) return [];
    const cookieHeaders = (item.responseHeaders || []).filter((h) => h.key?.toLowerCase() === 'set-cookie');
    return cookieHeaders.map((h, idx) => {
      const [k, v] = (h.value || '').split(';')[0].trim().split('=');
      return { id: `sc-${idx}`, key: k || '', value: v || '', enabled: true };
    });
  }, [item?.responseHeaders]);

  const reqCounts: Record<string, number | undefined> = {
    headers: item?.requestHeaders?.length,
    params: parsedParams.length,
    cookies: reqCookies.length,
  };
  const resCounts: Record<string, number | undefined> = {
    headers: item?.responseHeaders?.length,
    cookies: resCookies.length,
  };

  if (!item) {
    return (
      <div className="h-full flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs italic">
        <MingCuteIcon name="search_line" size={32} className="mb-2 opacity-30" />
        Select a request row from the log table above to view headers, body, params, and cookies.
      </div>
    );
  }

  const renderBodyContent = (
    bodyText: string,
    format: string,
    media: ReturnType<typeof detectMediaResponse>
  ) => {
    if (format === 'preview' && media) {
      return <MediaResponsePreview media={media} title={item?.url} />;
    }

    if (!bodyText) {
      return <div className="p-4 text-muted-foreground italic text-xs">No body content</div>;
    }

    if (format === 'tree') {
      const isReq = bodyText === item?.requestBody;
      const search = isReq ? reqTreeSearch : resTreeSearch;
      const setSearch = isReq ? setReqTreeSearch : setResTreeSearch;
      const filterMode = isReq ? reqTreeFilterMode : resTreeFilterMode;
      const setFilterMode = isReq ? setReqTreeFilterMode : setResTreeFilterMode;

      try {
        const parsed = JSON.parse(bodyText);
        return (
          <div className="h-full flex flex-col overflow-hidden">
            <div className="flex items-center gap-2 font-mono text-xs px-2 py-1.5 border-b border-border">
              <input
                type="text"
                placeholder="Search JSON tree..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary w-48"
              />
              <button
                type="button"
                onClick={() => setFilterMode(!filterMode)}
                className={`px-2 py-0.5 text-[10px] font-bold rounded border cursor-pointer ${
                  filterMode
                    ? 'bg-amber-500/20 text-amber-400 border-amber-500/40'
                    : 'bg-background text-muted-foreground border-border hover:text-foreground'
                }`}
                title="Filter mode: show matching nodes only"
              >
                {filterMode ? 'Filter On' : 'Filter Off'}
              </button>
            </div>
            <div className="flex-1 overflow-auto p-2 font-mono">
              <JsonTreeViewerRoot value={parsed} searchTerm={search} filterMode={filterMode} />
            </div>
          </div>
        );
      } catch {
        return <CodeEditor value={bodyText} language="plaintext" readOnly bare />;
      }
    } else if (format === 'pretty') {
      try {
        const parsed = JSON.parse(bodyText);
        const prettyJson = JSON.stringify(parsed, null, 2);
        return <CodeEditor value={prettyJson} language="json" readOnly bare />;
      } catch (e) {
        return <CodeEditor value={bodyText} language="plaintext" readOnly bare />;
      }
    } else if (format === 'hex') {
      return <HexViewer content={bodyText} />;
    } else if (format === 'html') {
      // Empty sandbox: no scripts, no same-origin access to the app or its Tauri bridge
      return <iframe sandbox="" srcDoc={bodyText} title="HTML preview" className="w-full h-full bg-white border-0" />;
    }

    return <CodeEditor value={bodyText} language="plaintext" readOnly bare />;
  };

  const isPendingResponse = item.phase === 'request';
  const isPendingIntercept = item.phase === 'intercepted_request' || item.phase === 'intercepted_response';
  const isFailed = item.isFailed && !isPendingResponse && !isPendingIntercept;

  return (
    <div className="h-full flex flex-col bg-surface border-t border-border overflow-hidden text-xs">
      {/* Top Banner Header */}
      <div className="p-2 bg-header border-b border-border flex items-center justify-between gap-2 shrink-0 font-mono">
        <div className="flex items-center gap-2 overflow-hidden truncate">
          <MethodBadge method={item.method} />
          <StatusBadge
            code={item.statusCode}
            isFailed={isFailed}
            isPending={isPendingResponse}
            isInterceptedPending={isPendingIntercept}
          />
          {item.listenerLabel && item.listenerLabel !== 'Default' && (
            <span
              className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-primary/15 text-primary border border-primary/25 shrink-0 select-none"
              title={`Source: ${item.listenerLabel}`}
            >
              <MingCuteIcon name="route_line" size={11} />
              <span>{item.listenerLabel}</span>
            </span>
          )}
          {isPendingIntercept && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-500/20 text-amber-600 dark:text-amber-400 border border-amber-500/40 shrink-0 animate-pulse">
              PAUSED IN INTERCEPTOR
            </span>
          )}
          {isPendingResponse && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-rose-500/20 text-rose-600 dark:text-rose-400 border border-rose-500/40 shrink-0 animate-pulse">
              WAITING FOR RESPONSE...
            </span>
          )}
          {item.isIntercepted && !isPendingIntercept && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30 shrink-0">
              INTERCEPTED
            </span>
          )}
          {item.isRewritten && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30 shrink-0">
              REWRITTEN
            </span>
          )}
          {isFailed && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-rose-500/15 text-rose-600 dark:text-rose-400 border border-rose-500/30 shrink-0">
              FAILED (NO RESPONSE)
            </span>
          )}
          <span className="font-semibold text-foreground truncate">{item.url}</span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <span className="text-muted-foreground text-[11px]">
            {isPendingResponse || isPendingIntercept ? '...' : item.durationMs != null ? `${item.durationMs}ms` : '-'}
          </span>
          <span className="text-muted-foreground text-[11px]">
            {isPendingResponse || isPendingIntercept ? '-' : `${item.size} B`}
          </span>

          {onHide && (
            <button
              onClick={onHide}
              className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
              title="Hide panel (select a request to show it again)"
            >
              <MingCuteIcon name="close_line" size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Main Split Body: Request vs Response (With Resizer Handle) */}
      <div
        ref={splitContainerRef}
        className="flex-1 flex flex-row overflow-hidden"
      >
        {/* REQUEST PANEL */}
        <div
          ref={reqPaneRef}
          className="flex flex-col overflow-hidden min-w-[120px] border-r border-border"
          style={{ width: `${reqWidthPercent}%` }}
        >
          <PaneHeader
            title="Request"
            tabs={REQ_TABS.map((t) => ({ ...t, count: reqCounts[t.value] }))}
            activeTab={reqTab}
            onTabChange={(val) => setReqTab(val as typeof reqTab)}
            right={
              reqTab === 'body' && (
                <SegmentedControl value={reqBodyFormat} onChange={setReqBodyFormat} options={reqBodyFormats} />
              )
            }
          />

          <div className={`flex-1 bg-surface ${reqTab === 'body' ? 'overflow-hidden' : 'p-2 overflow-auto'}`}>
            {reqTab === 'headers' && (
              <KeyValueEditor
                items={(item.requestHeaders || []).map((h, i) => ({ id: `rh-${i}`, key: h.key, value: h.value, enabled: true }))}
                onChange={() => {}}
                readOnly
              />
            )}
            {reqTab === 'body' && renderBodyContent(item.requestBody, reqBodyFormat, reqMediaInfo)}
            {reqTab === 'params' && (
              <KeyValueEditor items={parsedParams} onChange={() => {}} readOnly />
            )}
            {reqTab === 'cookies' && (
              <KeyValueEditor items={reqCookies} onChange={() => {}} readOnly />
            )}
          </div>
        </div>

        {/* INVISIBLE DRAGGABLE RESIZER HANDLE BETWEEN REQUEST & RESPONSE */}
        <div
          onPointerDown={handleSplitPointerDown}
          onDoubleClick={resetReqWidthPercent}
          className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
          title="Drag to resize, double-click to reset"
        >
          <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
        </div>

        {/* RESPONSE PANEL */}
        <div
          ref={resPaneRef}
          className="flex flex-col overflow-hidden min-w-[120px]"
          style={{ width: `${100 - reqWidthPercent}%` }}
        >
          <PaneHeader
            title="Response"
            tabs={RES_TABS.map((t) => ({ ...t, count: resCounts[t.value] }))}
            activeTab={resTab}
            onTabChange={(val) => setResTab(val as typeof resTab)}
            right={
              resTab === 'body' && (
                <SegmentedControl value={resBodyFormat} onChange={setResBodyFormat} options={resBodyFormats} />
              )
            }
          />

          <div className={`flex-1 bg-surface ${resTab === 'body' ? 'overflow-hidden' : 'p-2 overflow-auto'}`}>
            {resTab === 'headers' && (
              <KeyValueEditor
                items={(item.responseHeaders || []).map((h, i) => ({ id: `resh-${i}`, key: h.key, value: h.value, enabled: true }))}
                onChange={() => {}}
                readOnly
              />
            )}
            {resTab === 'body' && renderBodyContent(item.responseBody, resBodyFormat, resMediaInfo)}
            {resTab === 'cookies' && (
              <KeyValueEditor items={resCookies} onChange={() => {}} readOnly />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
