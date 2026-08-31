import React, { useState, useRef, useEffect, useMemo } from 'react';
import type { TrafficItem } from '../../../types';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl } from '../../common/ui';
import { MediaResponsePreview } from '../../common/MediaResponsePreview';
import { JsonTreeViewerRoot } from '../../common/JsonTreeViewer';
import { detectMediaResponse } from '../../../utils/mediaDetector';
import { isJsonString } from '../../../utils/bodyConverters';

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
  layoutMode: 'horizontal' | 'vertical';
  onToggleLayoutMode: () => void;
}

export const RequestResponseInspector: React.FC<RequestResponseInspectorProps> = ({
  item,
  layoutMode,
  onToggleLayoutMode,
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
  const [reqWidthPercent, setReqWidthPercent] = useState<number>(50);
  const splitContainerRef = useRef<HTMLDivElement>(null);
  const isResizingWidth = useRef(false);

  const handleMouseDownSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingWidth.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingWidth.current || !splitContainerRef.current) return;
      const rect = splitContainerRef.current.getBoundingClientRect();

      if (layoutMode === 'vertical') {
        const relativeX = moveEvent.clientX - rect.left;
        const newPercent = (relativeX / rect.width) * 100;
        setReqWidthPercent(Math.min(Math.max(newPercent, 15), 85));
      } else {
        const relativeY = moveEvent.clientY - rect.top;
        const newPercent = (relativeY / rect.height) * 100;
        setReqWidthPercent(Math.min(Math.max(newPercent, 15), 85));
      }
    };

    const handleMouseUp = () => {
      isResizingWidth.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
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
          <div className="h-full flex flex-col gap-2 overflow-hidden p-1">
            <div className="flex items-center gap-2 font-mono text-xs">
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
            <div className="flex-1 overflow-auto bg-background p-2 rounded border border-border font-mono">
              <JsonTreeViewerRoot value={parsed} searchTerm={search} filterMode={filterMode} />
            </div>
          </div>
        );
      } catch {
        return <CodeEditor value={bodyText} language="plaintext" readOnly />;
      }
    } else if (format === 'pretty') {
      try {
        const parsed = JSON.parse(bodyText);
        const prettyJson = JSON.stringify(parsed, null, 2);
        return <CodeEditor value={prettyJson} language="json" readOnly />;
      } catch (e) {
        return <CodeEditor value={bodyText} language="plaintext" readOnly />;
      }
    } else if (format === 'hex') {
      return <HexViewer content={bodyText} />;
    } else if (format === 'html') {
      return (
        <div className="w-full h-full bg-white text-black p-4 overflow-auto text-xs font-sans border rounded">
          <div dangerouslySetInnerHTML={{ __html: bodyText }} />
        </div>
      );
    }

    return <CodeEditor value={bodyText} language="plaintext" readOnly />;
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
            {isPendingResponse || isPendingIntercept ? '-' : `${item.size} bytes`}
          </span>

          {/* Layout Split Mode Switcher */}
          <button
            onClick={onToggleLayoutMode}
            className="p-1 rounded bg-surface border border-border text-muted-foreground hover:text-foreground"
            title={`Switch to ${layoutMode === 'vertical' ? 'Horizontal' : 'Vertical'} split`}
          >
            <MingCuteIcon name={layoutMode === 'vertical' ? 'grid_line' : 'storage_line'} size={14} />
          </button>
        </div>
      </div>

      {/* Main Split Body: Request vs Response (With Resizer Handle) */}
      <div
        ref={splitContainerRef}
        className={`flex-1 flex ${layoutMode === 'vertical' ? 'flex-row' : 'flex-col'} overflow-hidden`}
      >
        {/* REQUEST PANEL */}
        <div
          className={`flex flex-col overflow-hidden min-h-[120px] min-w-[120px] ${
            layoutMode === 'vertical' ? 'border-r border-border' : 'border-b border-border'
          }`}
          style={
            layoutMode === 'vertical'
              ? { width: `${reqWidthPercent}%` }
              : { height: `${reqWidthPercent}%` }
          }
        >
          <div className="bg-header border-b border-border px-2 py-1 flex items-center justify-between shrink-0 select-none">
            <span className="font-semibold text-foreground text-[11px] uppercase tracking-wider">Request</span>
            <SegmentedControl
              value={reqTab}
              onChange={(val) => setReqTab(val as any)}
              options={REQ_TABS}
            />
          </div>

          <div className="flex-1 p-2 overflow-auto bg-surface">
            {reqTab === 'headers' && (
              <KeyValueEditor
                items={(item.requestHeaders || []).map((h, i) => ({ id: `rh-${i}`, key: h.key, value: h.value, enabled: true }))}
                onChange={() => {}}
                readOnly
              />
            )}
            {reqTab === 'body' && (
              <div className="h-full flex flex-col gap-1">
                <div className="flex items-center gap-1 mb-1">
                  <SegmentedControl
                    value={reqBodyFormat}
                    onChange={(val) => setReqBodyFormat(val)}
                    options={reqBodyFormats}
                  />
                </div>
                <div className="flex-1 overflow-hidden">{renderBodyContent(item.requestBody, reqBodyFormat, reqMediaInfo)}</div>
              </div>
            )}
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
          onMouseDown={handleMouseDownSplit}
          className={`${
            layoutMode === 'vertical'
              ? 'w-1 cursor-col-resize -mx-0.5'
              : 'h-1 cursor-row-resize -my-0.5'
          } bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative`}
          title="Drag to adjust Request vs Response split size"
        >
          {layoutMode === 'vertical' ? (
            <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
          ) : (
            <div className="w-8 h-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
          )}
        </div>

        {/* RESPONSE PANEL */}
        <div
          className="flex flex-col overflow-hidden min-h-[120px] min-w-[120px]"
          style={
            layoutMode === 'vertical'
              ? { width: `${100 - reqWidthPercent}%` }
              : { height: `${100 - reqWidthPercent}%` }
          }
        >
          <div className="bg-header border-b border-border px-2 py-1 flex items-center justify-between shrink-0 select-none">
            <span className="font-semibold text-foreground text-[11px] uppercase tracking-wider">Response</span>
            <SegmentedControl
              value={resTab}
              onChange={(val) => setResTab(val as any)}
              options={RES_TABS}
            />
          </div>

          <div className="flex-1 p-2 overflow-auto bg-surface">
            {resTab === 'headers' && (
              <KeyValueEditor
                items={(item.responseHeaders || []).map((h, i) => ({ id: `resh-${i}`, key: h.key, value: h.value, enabled: true }))}
                onChange={() => {}}
                readOnly
              />
            )}
            {resTab === 'body' && (
              <div className="h-full flex flex-col gap-1">
                <div className="flex items-center gap-1 mb-1">
                  <SegmentedControl
                    value={resBodyFormat}
                    onChange={(val) => setResBodyFormat(val)}
                    options={resBodyFormats}
                  />
                </div>
                <div className="flex-1 overflow-hidden">{renderBodyContent(item.responseBody, resBodyFormat, resMediaInfo)}</div>
              </div>
            )}
            {resTab === 'cookies' && (
              <KeyValueEditor items={resCookies} onChange={() => {}} readOnly />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
