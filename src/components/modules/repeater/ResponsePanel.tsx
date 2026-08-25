import React, { useState } from 'react';
import type { RepeaterHistoryItem, RepeaterExecutionResult } from '../../../services/tauri/bridge';
import { StatusBadge } from '../../common/StatusBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';

interface ResponsePanelProps {
  response: RepeaterHistoryItem | RepeaterExecutionResult | null;
}

export const ResponsePanel: React.FC<ResponsePanelProps> = ({ response }) => {
  const [activeTab, setActiveTab] = useState<'body' | 'headers'>('body');
  const [bodyFormat, setBodyFormat] = useState<'pretty' | 'raw' | 'hex' | 'html'>('pretty');
  const [copied, setCopied] = useState(false);

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

  const handleCopyBody = () => {
    if (!responseBody) return;
    navigator.clipboard.writeText(responseBody);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const renderBodyContent = () => {
    if (!responseBody) {
      return (
        <div className="h-full flex items-center justify-center text-muted-foreground italic text-xs">
          No body content returned
        </div>
      );
    }

    if (bodyFormat === 'pretty') {
      try {
        const parsed = JSON.parse(responseBody);
        const prettyJson = JSON.stringify(parsed, null, 2);
        return <CodeEditor value={prettyJson} language="json" readOnly />;
      } catch (e) {
        // Fallback for HTML/Plaintext if JSON parsing fails
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
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs select-none">
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
                ? 'bg-primary text-white font-semibold shadow-xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            Body
          </button>
          <button
            onClick={() => setActiveTab('headers')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer ${
              activeTab === 'headers'
                ? 'bg-primary text-white font-semibold shadow-xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            Headers ({responseHeaders.length})
          </button>
        </div>
      </div>

      {/* Format Selector Bar for Body View */}
      {activeTab === 'body' && (
        <div className="px-3 py-1.5 bg-surface/80 border-b border-border/80 flex items-center justify-between shrink-0 font-mono text-[11px]">
          <div className="flex items-center gap-1 bg-background/60 p-0.5 rounded border border-border/60">
            {(['pretty', 'raw', 'hex', 'html'] as const).map((fmt) => (
              <button
                key={fmt}
                onClick={() => setBodyFormat(fmt)}
                className={`px-2 py-0.5 rounded uppercase text-[10px] font-semibold transition-colors cursor-pointer ${
                  bodyFormat === fmt
                    ? 'bg-zinc-800 text-primary border border-primary/40 shadow-2xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-zinc-800/40'
                }`}
              >
                {fmt}
              </button>
            ))}
          </div>

          <button
            onClick={handleCopyBody}
            disabled={!responseBody}
            className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground transition-colors cursor-pointer disabled:opacity-40"
            title="Copy Response Body"
          >
            <MingCuteIcon name={copied ? 'check_line' : 'copy_line'} size={13} className={copied ? 'text-emerald-400' : ''} />
            <span>{copied ? 'Copied!' : 'Copy'}</span>
          </button>
        </div>
      )}

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
