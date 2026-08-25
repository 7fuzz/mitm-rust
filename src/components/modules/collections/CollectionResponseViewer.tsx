import React, { useState } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { StatusBadge } from '../../common/StatusBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';

interface CollectionResponseViewerProps {
  requestId: string;
}

export const CollectionResponseViewer: React.FC<CollectionResponseViewerProps> = ({ requestId }) => {
  const { executionResult, isExecuting } = useCollectionStore();
  const [activeTab, setActiveTab] = useState<'body' | 'headers'>('body');
  const [bodyFormat, setBodyFormat] = useState<'pretty' | 'raw' | 'hex'>('pretty');

  const result = executionResult[requestId];
  const isLoading = isExecuting[requestId];

  if (isLoading) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs select-none">
        <MingCuteIcon name="loading_line" size={32} className="animate-spin text-primary mb-2" />
        <span>Executing request...</span>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs italic select-none">
        <MingCuteIcon name="send_plane_line" size={36} className="mb-2 opacity-30" />
        <span>Click "Send" above to execute request and inspect response data.</span>
      </div>
    );
  }

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const renderBodyContent = () => {
    const responseBody = result.responseBody || '';
    if (!responseBody) {
      return (
        <div className="h-full flex items-center justify-center text-muted-foreground italic text-xs">
          No response body returned
        </div>
      );
    }

    if (bodyFormat === 'pretty') {
      try {
        const parsed = JSON.parse(responseBody);
        return <CodeEditor value={JSON.stringify(parsed, null, 2)} language="json" readOnly />;
      } catch {
        return <CodeEditor value={responseBody} language="plaintext" readOnly />;
      }
    } else if (bodyFormat === 'hex') {
      return <HexViewer content={responseBody} />;
    }

    return <CodeEditor value={responseBody} language="plaintext" readOnly />;
  };

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs select-none">
      {/* Response Status Bar */}
      <div className="p-2.5 bg-header border-b border-border flex items-center justify-between font-mono shrink-0">
        <div className="flex items-center gap-3">
          <StatusBadge code={result.statusCode} />
          {result.statusText && <span className="text-foreground font-semibold">{result.statusText}</span>}
          <span className="text-muted-foreground text-[11px]">
            Time: <strong className="text-foreground">{result.durationMs}ms</strong>
          </span>
          <span className="text-muted-foreground text-[11px]">
            Size: <strong className="text-foreground">{formatBytes(result.responseSize)}</strong>
          </span>
        </div>

        {/* Primary Tabs: Body vs Headers */}
        <div className="flex items-center gap-1">
          <button
            onClick={() => setActiveTab('body')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer ${
              activeTab === 'body'
                ? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            Body
          </button>
          <button
            onClick={() => setActiveTab('headers')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer ${
              activeTab === 'headers'
                ? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            Headers ({result.responseHeaders?.length || 0})
          </button>
        </div>
      </div>

      {/* Format Selector Bar */}
      <div className="px-3 py-1.5 bg-surface border-b border-border flex items-center justify-between shrink-0 font-mono text-[11px]">
        {activeTab === 'body' ? (
          <div className="flex items-center gap-1 bg-background p-0.5 rounded border border-border">
            {(['pretty', 'raw', 'hex'] as const).map((fmt) => (
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
      </div>

      {/* Content Area */}
      <div className="flex-1 p-2 overflow-hidden">
        {activeTab === 'body' && renderBodyContent()}

        {activeTab === 'headers' && (
          <div className="h-full overflow-y-auto">
            <KeyValueEditor
              items={(result.responseHeaders || []).map((h, i) => ({
                id: `resh-${i}`,
                key: h.key,
                value: h.value,
                enabled: true,
              }))}
              onChange={() => {}}
              readOnly
            />
          </div>
        )}
      </div>
    </div>
  );
};
