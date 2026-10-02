import React, { useState, useRef, useEffect, useMemo } from 'react';
import type { RepeaterHistoryItem, RepeaterExecutionResult } from '../../../services/tauri/bridge';
import { StatusBadge } from '../../common/StatusBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { JsonTreeViewerRoot } from '../../common/JsonTreeViewer';
import { MediaResponsePreview } from '../../common/MediaResponsePreview';
import { detectMediaResponse } from '../../../utils/mediaDetector';
import { isJsonString } from '../../../utils/bodyConverters';
import { historyItemToTrafficItem, RUN_COPY_ACTIONS } from '../../../utils/repeaterTraffic';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import type { TrafficItem } from '../../../types';
import { PaneHeader } from '../../common/PaneHeader';
import { SegmentedControl } from '../../common/ui';
import { CopyCustomModal } from '../history/CopyCustomModal';

interface ResponsePanelProps {
  response: RepeaterHistoryItem | RepeaterExecutionResult | null;
}

export const ResponsePanel: React.FC<ResponsePanelProps> = ({ response }) => {
  const [activeTab, setActiveTab] = useState<'body' | 'headers'>('body');
  const [bodyFormat, setBodyFormat] = useState<string>('pretty');
  const [treeSearch, setTreeSearch] = useState('');
  const [treeFilterMode, setTreeFilterMode] = useState(false);
  const [copyMenuOpen, setCopyMenuOpen] = useState(false);
  const [copyNotification, setCopyNotification] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);
  const executionHistory = useRepeaterStore((s) => s.executionHistory);

  // The request exactly as sent: a history item carries it; a fresh result is matched to its run
  const sentRun = useMemo<RepeaterHistoryItem | null>(() => {
    if (!response) return null;
    if ('requestHeaders' in response) return response;
    return executionHistory[response.repeaterId]?.find((h) => h.id === response.historyId) ?? null;
  }, [response, executionHistory]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setCopyMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const responseBody = response?.responseBody || '';
  const responseHeaders = response?.responseHeaders || [];
  const durationMs = response?.durationMs || 0;
  const size = response && 'responseSize' in response ? response.responseSize : responseBody.length;
  const statusText = response && 'statusText' in response ? response.statusText : '';

  const mediaInfo = useMemo(
    () => (response ? detectMediaResponse(responseBody, responseHeaders) : null),
    [response, responseBody, responseHeaders]
  );

  const isJson = useMemo(() => {
    if (!responseBody) return false;
    const hasJsonHeader = (responseHeaders || []).some(
      (h) => h.key?.toLowerCase() === 'content-type' && h.value?.toLowerCase().includes('json')
    );
    if (hasJsonHeader) return true;
    return isJsonString(responseBody);
  }, [responseBody, responseHeaders]);

  // Auto-switch to preview when media is detected
  useEffect(() => {
    if (mediaInfo) {
      setBodyFormat('preview');
    } else if (bodyFormat === 'tree' && !isJson) {
      setBodyFormat('pretty');
    }
  }, [mediaInfo, isJson, response]);

  const availableFormats = useMemo(() => {
    const list: Array<{ value: string; label: string }> = [];
    if (mediaInfo) {
      list.push({ value: 'preview', label: `Preview (${mediaInfo.previewType.toUpperCase()})` });
    }
    list.push({ value: 'pretty', label: 'Pretty' });
    if (isJson) {
      list.push({ value: 'tree', label: 'Tree' });
    }
    list.push(
      { value: 'raw', label: 'Raw' },
      { value: 'hex', label: 'Hex' },
      { value: 'html', label: 'HTML' }
    );
    return list;
  }, [mediaInfo, isJson]);

  if (!response) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs italic select-none">
        <MingCuteIcon name="send_plane_line" size={36} className="mb-2 opacity-30" />
        Click "Send" above to execute the request and view response headers & payload here.
      </div>
    );
  }

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const showNotification = (msg: string) => {
    setCopyNotification(msg);
    setCopyMenuOpen(false);
    setTimeout(() => setCopyNotification(null), 1800);
  };

  const handleCopyBody = () => {
    if (!responseBody) return;
    navigator.clipboard.writeText(responseBody);
    showNotification('Copied Body!');
  };

  const handleCopyHeaders = () => {
    const statusLine = `HTTP/1.1 ${response.statusCode} ${statusText}`.trim();
    const rawHeaders = (responseHeaders || [])
      .map((h) => `${h.key}: ${h.value}`)
      .join('\n');
    navigator.clipboard.writeText(`${statusLine}\n${rawHeaders}`);
    showNotification('Copied Headers!');
  };

  const handleCopyAll = () => {
    const statusLine = `HTTP/1.1 ${response.statusCode} ${statusText}`.trim();
    const rawHeaders = (responseHeaders || [])
      .map((h) => `${h.key}: ${h.value}`)
      .join('\n');
    const fullRes = `${statusLine}\n${rawHeaders}${responseBody ? `\n\n${responseBody}` : ''}`;
    navigator.clipboard.writeText(fullRes);
    showNotification('Copied Full Response!');
  };

  const copyWithRequest = (format: (item: TrafficItem) => string, message: string) => {
    if (!sentRun) return;
    navigator.clipboard.writeText(format(historyItemToTrafficItem(sentRun)));
    showNotification(message);
  };

  const renderBodyContent = () => {
    if (bodyFormat === 'preview' && mediaInfo) {
      return <MediaResponsePreview media={mediaInfo} title="repeater-response" />;
    }

    if (!responseBody) {
      return (
        <div className="h-full flex items-center justify-center text-muted-foreground italic text-xs">
          No body content returned
        </div>
      );
    }

    if (bodyFormat === 'tree') {
      try {
        const parsed = JSON.parse(responseBody);
        return (
          <div className="h-full flex flex-col overflow-hidden">
            <div className="flex items-center gap-2 font-mono text-xs px-2 py-1.5 border-b border-border">
              <input
                type="text"
                placeholder="Search JSON tree..."
                value={treeSearch}
                onChange={(e) => setTreeSearch(e.target.value)}
                className="bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary w-48"
              />
              <button
                type="button"
                onClick={() => setTreeFilterMode(!treeFilterMode)}
                className={`px-2 py-0.5 text-[10px] font-bold rounded border cursor-pointer ${
                  treeFilterMode
                    ? 'bg-amber-500/20 text-amber-400 border-amber-500/40'
                    : 'bg-background text-muted-foreground border-border hover:text-foreground'
                }`}
                title="Filter mode: show matching nodes only"
              >
                {treeFilterMode ? 'Filter On' : 'Filter Off'}
              </button>
            </div>
            <div className="flex-1 overflow-auto p-2 font-mono">
              <JsonTreeViewerRoot value={parsed} searchTerm={treeSearch} filterMode={treeFilterMode} />
            </div>
          </div>
        );
      } catch {
        return <CodeEditor value={responseBody} language="plaintext" readOnly bare />;
      }
    } else if (bodyFormat === 'pretty') {
      try {
        const parsed = JSON.parse(responseBody);
        const prettyJson = JSON.stringify(parsed, null, 2);
        return <CodeEditor value={prettyJson} language="json" readOnly bare />;
      } catch (e) {
        if (responseBody.trim().startsWith('<')) {
          return <CodeEditor value={responseBody} language="html" readOnly bare />;
        }
        return <CodeEditor value={responseBody} language="plaintext" readOnly bare />;
      }
    } else if (bodyFormat === 'hex') {
      return <HexViewer content={responseBody} />;
    } else if (bodyFormat === 'html') {
      // Empty sandbox: no scripts, no same-origin access to the app or its Tauri bridge
      return <iframe sandbox="" srcDoc={responseBody} title="HTML preview" className="w-full h-full bg-white border-0" />;
    }

    return <CodeEditor value={responseBody} language="plaintext" readOnly bare />;
  };

  const menuItemClass =
    'px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed';

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs">
      {/* Response Status Bar (same height as the request URL bar so the tab rows line up) */}
      <div className="h-12 px-3 bg-header border-b border-border flex items-center justify-between gap-3 font-mono shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <StatusBadge code={response.statusCode} />
          {statusText && <span className="text-foreground font-semibold truncate">{statusText}</span>}
          <span className="text-muted-foreground text-[11px] shrink-0">
            {durationMs}ms · {formatBytes(size)}
          </span>
        </div>

        {/* Copy Response Actions Dropdown */}
        <div className="relative shrink-0" ref={menuRef}>
          <button
            onClick={() => setCopyMenuOpen(!copyMenuOpen)}
            className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground transition-colors cursor-pointer text-xs font-medium"
            title="Copy Response Options"
          >
            <MingCuteIcon name={copyNotification ? 'check_line' : 'copy_line'} size={13} className={copyNotification ? 'text-emerald-400' : ''} />
            <span>{copyNotification || 'Copy'}</span>
            <MingCuteIcon name="down_line" size={12} className="opacity-60" />
          </button>

          {copyMenuOpen && (
            <div className="absolute right-0 mt-1 w-56 bg-surface border border-border rounded-lg shadow-xl py-1 z-50 font-mono text-xs flex flex-col">
              <div className="px-3 pt-1 pb-0.5 text-[10px] uppercase tracking-wider font-sans font-semibold text-muted-foreground">Response</div>
              <button onClick={handleCopyBody} disabled={!responseBody} className={menuItemClass}>
                <MingCuteIcon name="file_text_line" size={14} className="text-primary" />
                <span>Copy Body</span>
              </button>
              <button onClick={handleCopyHeaders} className={menuItemClass}>
                <MingCuteIcon name="list_check_line" size={14} className="text-emerald-500" />
                <span>Copy Headers</span>
              </button>
              <button onClick={handleCopyAll} className={menuItemClass}>
                <MingCuteIcon name="copy_line" size={14} className="text-amber-500" />
                <span>Copy Full Response</span>
              </button>

              <div className="my-1 border-t border-border/80" />
              <div
                className="px-3 pt-0.5 pb-0.5 text-[10px] uppercase tracking-wider font-sans font-semibold text-muted-foreground"
                title={sentRun ? undefined : 'The request for this response is not in the run history yet'}
              >
                Request + Response
              </div>
              {RUN_COPY_ACTIONS.map((action) => (
                <button
                  key={action.label}
                  onClick={() => copyWithRequest(action.format, action.message)}
                  disabled={!sentRun}
                  className={menuItemClass}
                >
                  <MingCuteIcon name={action.icon} size={14} className={action.iconClass} />
                  <span>{action.label}</span>
                </button>
              ))}
              <button
                onClick={() => {
                  setCopyMenuOpen(false);
                  if (sentRun) setCustomCopyItem(historyItemToTrafficItem(sentRun));
                }}
                disabled={!sentRun}
                className={menuItemClass}
              >
                <MingCuteIcon name="settings_3_line" size={14} className="text-muted-foreground" />
                <span>Copy custom...</span>
              </button>
            </div>
          )}
        </div>
      </div>

      <PaneHeader
        tabs={[
          { value: 'body', label: 'Body' },
          { value: 'headers', label: 'Headers', count: responseHeaders.length },
        ]}
        activeTab={activeTab}
        onTabChange={(val) => setActiveTab(val as typeof activeTab)}
        right={
          activeTab === 'body' && (
            <SegmentedControl value={bodyFormat} onChange={setBodyFormat} options={availableFormats} />
          )
        }
      />

      {/* Response Panel Main Content */}
      <div className={`flex-1 bg-surface ${activeTab === 'body' ? 'overflow-hidden' : 'p-2 overflow-y-auto'}`}>
        {activeTab === 'body' && renderBodyContent()}

        {activeTab === 'headers' && (
          <KeyValueEditor
            items={responseHeaders.map((h, i) => ({ id: h.id || `resh-${i}`, key: h.key, value: h.value, enabled: true }))}
            onChange={() => {}}
            readOnly
          />
        )}
      </div>

      <CopyCustomModal isOpen={!!customCopyItem} item={customCopyItem} onClose={() => setCustomCopyItem(null)} />
    </div>
  );
};
