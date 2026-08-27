import React, { useState, useRef, useEffect } from 'react';
import type { RepeaterHistoryItem, RepeaterExecutionResult } from '../../../services/tauri/bridge';
import { StatusBadge } from '../../common/StatusBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { JsonTreeViewer } from '../../common/JsonTreeViewer';

interface ResponsePanelProps {
  response: RepeaterHistoryItem | RepeaterExecutionResult | null;
}

export const ResponsePanel: React.FC<ResponsePanelProps> = ({ response }) => {
  const [activeTab, setActiveTab] = useState<'body' | 'headers'>('body');
  const [bodyFormat, setBodyFormat] = useState<'pretty' | 'tree' | 'raw' | 'hex' | 'html'>('pretty');
  const [treeSearch, setTreeSearch] = useState('');
  const [treeFilterMode, setTreeFilterMode] = useState(false);
  const [copyMenuOpen, setCopyMenuOpen] = useState(false);
  const [copyNotification, setCopyNotification] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setCopyMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  if (!response) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs italic select-none">
        <MingCuteIcon name="send_plane_line" size={36} className="mb-2 opacity-30" />
        Click "Send" above to execute the request and view response headers & payload here.
      </div>
    );
  }

  const responseBody = response.responseBody || '';
  const responseHeaders = response.responseHeaders || [];
  const durationMs = response.durationMs || 0;
  const size = 'responseSize' in response ? response.responseSize : responseBody.length;
  const statusText = 'statusText' in response ? response.statusText : '';

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

  const renderBodyContent = () => {
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
          <div className="h-full flex flex-col gap-2 overflow-hidden p-1">
            <div className="flex items-center gap-2 font-mono text-xs">
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
            <div className="flex-1 overflow-auto bg-background p-2 rounded border border-border font-mono">
              <JsonTreeViewer value={parsed} searchTerm={treeSearch} filterMode={treeFilterMode} />
            </div>
          </div>
        );
      } catch {
        return <CodeEditor value={responseBody} language="plaintext" readOnly />;
      }
    } else if (bodyFormat === 'pretty') {
      try {
        const parsed = JSON.parse(responseBody);
        const prettyJson = JSON.stringify(parsed, null, 2);
        return <CodeEditor value={prettyJson} language="json" readOnly />;
      } catch (e) {
        if (responseBody.trim().startsWith('<')) {
          return <CodeEditor value={responseBody} language="html" readOnly />;
        }
        return <CodeEditor value={responseBody} language="plaintext" readOnly />;
      }
    } else if (bodyFormat === 'hex') {
      return <HexViewer content={responseBody} />;
    } else if (bodyFormat === 'html') {
      return (
        <div className="w-full h-full bg-white text-black p-4 overflow-auto text-xs font-sans border rounded select-text">
          <div dangerouslySetInnerHTML={{ __html: responseBody }} />
        </div>
      );
    }

    return <CodeEditor value={responseBody} language="plaintext" readOnly />;
  };

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs">
      {/* Response Status Bar */}
      <div className="p-2.5 bg-header border-b border-border flex items-center justify-between font-mono shrink-0">
        <div className="flex items-center gap-3">
          <StatusBadge code={response.statusCode} />
          {statusText && <span className="text-foreground font-semibold">{statusText}</span>}
          <span className="text-muted-foreground text-[11px]">
            Time: <strong className="text-foreground">{durationMs}ms</strong>
          </span>
          <span className="text-muted-foreground text-[11px]">
            Size: <strong className="text-foreground">{formatBytes(size)}</strong>
          </span>
        </div>

        {/* Primary Tabs: Body vs Headers */}
        <div className="flex items-center gap-1">
          <button
            onClick={() => setActiveTab('body')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer ${
              activeTab === 'body'
                ? 'bg-primary text-primary-foreground font-semibold shadow-xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            Body
          </button>
          <button
            onClick={() => setActiveTab('headers')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer ${
              activeTab === 'headers'
                ? 'bg-primary text-primary-foreground font-semibold shadow-xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            Headers ({responseHeaders.length})
          </button>
        </div>
      </div>

      {/* Format Selector Bar & Copy Response Dropdown */}
      <div className="px-3 py-1.5 bg-surface border-b border-border flex items-center justify-between shrink-0 font-mono text-[11px]">
        {/* Format selectors (active in Body tab) */}
        {activeTab === 'body' ? (
          <div className="flex items-center gap-1 bg-background p-0.5 rounded border border-border">
            {(['pretty', 'tree', 'raw', 'hex', 'html'] as const).map((fmt) => (
              <button
                key={fmt}
                onClick={() => setBodyFormat(fmt)}
                className={`px-2 py-0.5 rounded uppercase text-[10px] font-semibold transition-colors cursor-pointer ${
                  bodyFormat === fmt
                    ? 'bg-primary text-primary-foreground font-bold shadow-2xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
                }`}
              >
                {fmt}
              </button>
            ))}
          </div>
        ) : (
          <div className="text-muted-foreground text-[11px]">Response Headers</div>
        )}

        {/* Copy Response Actions Dropdown */}
        <div className="relative" ref={menuRef}>
          <button
            onClick={() => setCopyMenuOpen(!copyMenuOpen)}
            className="flex items-center gap-1 px-2.5 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground transition-colors cursor-pointer text-xs font-medium"
            title="Copy Response Options"
          >
            <MingCuteIcon name={copyNotification ? 'check_line' : 'copy_line'} size={13} className={copyNotification ? 'text-emerald-400' : ''} />
            <span>{copyNotification || 'Copy Response'}</span>
            <MingCuteIcon name="down_line" size={12} className="opacity-60" />
          </button>

          {copyMenuOpen && (
            <div className="absolute right-0 mt-1 w-44 bg-surface border border-border rounded-lg shadow-xl py-1 z-50 font-mono text-xs flex flex-col">
              <button
                onClick={handleCopyBody}
                disabled={!responseBody}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer disabled:opacity-40"
              >
                <MingCuteIcon name="file_text_line" size={14} className="text-primary" />
                <span>Copy Body</span>
              </button>
              <button
                onClick={handleCopyHeaders}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer"
              >
                <MingCuteIcon name="list_check_line" size={14} className="text-emerald-500" />
                <span>Copy Headers</span>
              </button>
              <button
                onClick={handleCopyAll}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer border-t border-border/80"
              >
                <MingCuteIcon name="copy_line" size={14} className="text-amber-500" />
                <span>Copy All (Full Res)</span>
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Response Panel Main Content */}
      <div className="flex-1 p-2 overflow-hidden">
        {activeTab === 'body' && (
          <div className="h-full flex flex-col overflow-hidden">
            {renderBodyContent()}
          </div>
        )}

        {activeTab === 'headers' && (
          <div className="h-full overflow-y-auto">
            <KeyValueEditor
              items={responseHeaders.map((h, i) => ({ id: h.id || `resh-${i}`, key: h.key, value: h.value, enabled: true }))}
              onChange={() => {}}
              readOnly
            />
          </div>
        )}
      </div>
    </div>
  );
};
