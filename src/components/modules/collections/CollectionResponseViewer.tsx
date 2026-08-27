import React, { useState, useEffect } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { StatusBadge } from '../../common/StatusBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { getRequestHistories, isTauriAvailable, type RequestHistoryItem } from '../../../services/tauri/bridge';

interface CollectionResponseViewerProps {
  requestId: string;
}

export const CollectionResponseViewer: React.FC<CollectionResponseViewerProps> = ({ requestId }) => {
  const { executionResult, isExecuting } = useCollectionStore();
  const [activeTab, setActiveTab] = useState<'body' | 'headers'>('body');
  const [bodyFormat, setBodyFormat] = useState<'pretty' | 'raw' | 'hex'>('pretty');
  const [historyList, setHistoryList] = useState<RequestHistoryItem[]>([]);
  const [selectedHistoryId, setSelectedHistoryId] = useState<number | null>(null);

  const latestResult = executionResult[requestId];
  const isLoading = isExecuting[requestId];

  useEffect(() => {
    if (isTauriAvailable() && requestId) {
      getRequestHistories(requestId)
        .then((items) => {
          setHistoryList(items);
          if (items.length > 0) {
            setSelectedHistoryId(items[0].id);
          } else {
            setSelectedHistoryId(null);
          }
        })
        .catch((err) => console.error('Failed to load request histories:', err));
    }
  }, [requestId, latestResult]);

  if (isLoading) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs select-none">
        <MingCuteIcon name="loading_line" size={32} className="animate-spin text-primary mb-2" />
        <span>Executing request...</span>
      </div>
    );
  }

  const activeHistoryItem = historyList.find((h) => h.id === selectedHistoryId);

  const currentStatus = activeHistoryItem?.statusCode ?? latestResult?.statusCode;
  const currentStatusText = activeHistoryItem?.statusText || latestResult?.statusText || (currentStatus === 200 ? 'OK' : 'Response');
  const currentDuration = activeHistoryItem?.durationMs ?? latestResult?.durationMs ?? 0;
  const currentResponseBody = activeHistoryItem?.responseBody ?? latestResult?.responseBody ?? '';
  const currentResponseHeaders = activeHistoryItem?.responseHeaders ?? latestResult?.responseHeaders ?? [];
  const currentSize = activeHistoryItem?.responseBody ? activeHistoryItem.responseBody.length : (latestResult?.responseSize || 0);

  if (!latestResult && historyList.length === 0) {
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

  const formatTime = (ms: number) => {
    const d = new Date(ms);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  };

  const renderBodyContent = () => {
    if (!currentResponseBody) {
      return (
        <div className="h-full flex items-center justify-center text-muted-foreground italic text-xs">
          No response body returned
        </div>
      );
    }

    if (bodyFormat === 'pretty') {
      try {
        const parsed = JSON.parse(currentResponseBody);
        return <CodeEditor value={JSON.stringify(parsed, null, 2)} language="json" readOnly />;
      } catch {
        return <CodeEditor value={currentResponseBody} language="plaintext" readOnly />;
      }
    } else if (bodyFormat === 'hex') {
      return <HexViewer content={currentResponseBody} />;
    }

    return <CodeEditor value={currentResponseBody} language="plaintext" readOnly />;
  };

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs">
      {/* Response Status Bar */}
      <div className="p-2.5 bg-header border-b border-border flex items-center justify-between font-mono shrink-0 select-none">
        <div className="flex items-center gap-3">
          {currentStatus && <StatusBadge code={currentStatus} />}
          {currentStatusText && <span className="text-foreground font-semibold">{currentStatusText}</span>}
          <span className="text-muted-foreground text-[11px]">
            Time: <strong className="text-foreground">{currentDuration}ms</strong>
          </span>
          <span className="text-muted-foreground text-[11px]">
            Size: <strong className="text-foreground">{formatBytes(currentSize)}</strong>
          </span>
        </div>

        {/* Compact History Combobox */}
        {historyList.length > 0 && (
          <div className="flex items-center gap-1 bg-background px-2 py-0.5 rounded border border-border shrink-0">
            <MingCuteIcon name="history_line" size={12} className="text-muted-foreground shrink-0" />
            <select
              value={selectedHistoryId || ''}
              onChange={(e) => setSelectedHistoryId(Number(e.target.value))}
              className="bg-transparent text-[11px] font-mono text-foreground focus:outline-none cursor-pointer max-w-[140px] truncate"
            >
              {historyList.map((h, idx) => (
                <option key={h.id} value={h.id}>
                  {idx === 0 ? 'Latest' : `#${historyList.length - idx}`} ({h.statusCode} - {formatTime(h.executedAtMs)})
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      {/* Toolbar: Primary Tabs (Body / Headers) & Body Format Selectors */}
      <div className="px-3 py-1.5 bg-surface border-b border-border flex items-center justify-between shrink-0 font-mono text-[11px] select-none">
        {/* Left Side: Body vs Headers Tabs */}
        <div className="flex items-center gap-1 bg-background p-0.5 rounded border border-border">
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
            Headers ({currentResponseHeaders.length})
          </button>
        </div>

        {/* Right Side: Format Selector (Pretty / Raw / Hex) */}
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
              items={currentResponseHeaders.map((h, i) => ({
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
