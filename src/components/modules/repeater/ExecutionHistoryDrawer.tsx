import React, { useEffect, useState } from 'react';
import type { RepeaterHistoryItem } from '../../../services/tauri/bridge';
import type { TrafficItem } from '../../../types';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { StatusBadge } from '../../common/StatusBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';
import { CopyCustomModal } from '../history/CopyCustomModal';
import {
  formatRawCurl,
  formatReqAndRes,
  formatUrlBodyAndRes,
} from '../../../utils/reqResFormatter';

interface ExecutionHistoryDrawerProps {
  requestId: string;
  widthPx?: number;
}

const historyItemToTrafficItem = (hist: RepeaterHistoryItem): TrafficItem => {
  let host = '';
  let path = '/';
  try {
    const urlWithScheme = !hist.url.startsWith('http://') && !hist.url.startsWith('https://')
      ? `https://${hist.url}`
      : hist.url;
    const urlObj = new URL(urlWithScheme);
    host = urlObj.host;
    path = urlObj.pathname + urlObj.search;
  } catch {
    host = hist.url;
  }

  const ctHeader = (hist.responseHeaders || []).find(
    (h) => h.key?.toLowerCase() === 'content-type'
  );
  const contentType = ctHeader ? ctHeader.value : '';

  return {
    id: `rep-hist-${hist.id}`,
    method: hist.method,
    url: hist.url,
    host,
    path,
    contentType,
    statusCode: hist.statusCode,
    size: hist.responseBody ? hist.responseBody.length : 0,
    requestHeaders: (hist.requestHeaders || []).map((h) => ({ key: h.key, value: h.value })),
    responseHeaders: (hist.responseHeaders || []).map((h) => ({ key: h.key, value: h.value })),
    requestBody: hist.requestBody || '',
    responseBody: hist.responseBody || '',
    durationMs: hist.durationMs,
    timestamp: hist.executedAtMs,
    phase: 'done',
    isIntercepted: false,
    isRewritten: false,
    isFailed: hist.statusCode === 0,
  };
};

export const ExecutionHistoryDrawer: React.FC<ExecutionHistoryDrawerProps> = ({ requestId, widthPx = 288 }) => {
  const {
    executionHistory,
    isHistoryDrawerOpen,
    setHistoryDrawerOpen,
    fetchHistory,
    setExecutionResult,
    restoreHistoryToTab,
    lastExecutionResult,
  } = useRepeaterStore();

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: RepeaterHistoryItem } | null>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);

  useEffect(() => {
    if (isHistoryDrawerOpen && requestId) {
      fetchHistory(requestId);
    }
  }, [isHistoryDrawerOpen, requestId, fetchHistory]);

  if (!isHistoryDrawerOpen) return null;

  const history = executionHistory[requestId] || [];
  const currentResult = lastExecutionResult[requestId];

  const handleContextMenu = (e: React.MouseEvent, hist: RepeaterHistoryItem) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, item: hist });
  };

  const getContextMenuItems = (hist: RepeaterHistoryItem): ContextMenuItem[] => {
    const trafficItem = historyItemToTrafficItem(hist);
    return [
      {
        label: 'View Response in Panel',
        icon: 'eye_line',
        action: () => setExecutionResult(requestId, hist),
      },
      {
        label: 'Restore Request to Editor',
        icon: 'refresh_line',
        action: () => restoreHistoryToTab(requestId, hist),
      },
      {
        label: 'Copy',
        icon: 'copy_line',
        children: [
          {
            label: 'Copy Curl',
            icon: 'terminal_line',
            action: () => {
              const curl = formatRawCurl(trafficItem);
              navigator.clipboard.writeText(curl);
            },
          },
          {
            label: 'Copy Curl and Res',
            icon: 'transfer_line',
            action: () => {
              const formatted = formatReqAndRes(trafficItem);
              navigator.clipboard.writeText(formatted);
            },
          },
          {
            label: 'Copy URL, Body, and Response',
            icon: 'file_code_line',
            action: () => {
              const formatted = formatUrlBodyAndRes(trafficItem);
              navigator.clipboard.writeText(formatted);
            },
          },
          {
            label: 'Copy custom',
            icon: 'settings_3_line',
            action: () => setCustomCopyItem(trafficItem),
          },
        ],
      },
    ];
  };

  return (
    <div
      className="bg-surface border-l border-border h-full flex flex-col overflow-hidden text-xs shrink-0 select-none"
      style={{ width: `${widthPx}px` }}
    >
      <div
        onClick={() => setHistoryDrawerOpen(false)}
        className="p-3 bg-header border-b border-border font-semibold flex items-center justify-between cursor-pointer hover:bg-neutral-subtle transition-colors group"
        title="Click title to close Execution History"
      >
        <div className="flex items-center gap-2">
          <MingCuteIcon name="history_line" size={16} className="text-primary" />
          <span className="group-hover:text-primary transition-colors">Execution History ({history.length})</span>
        </div>
        <button className="text-muted-foreground group-hover:text-foreground p-0.5 rounded cursor-pointer">
          <MingCuteIcon name="close_line" size={14} />
        </button>
      </div>

      <div className="flex-1 p-2 overflow-y-auto space-y-2">
        {history.length === 0 ? (
          <div className="py-8 text-center text-muted-foreground italic">No historical executions logged</div>
        ) : (
          history.map((hist) => {
            const isSelected =
              currentResult &&
              'historyId' in currentResult
                ? currentResult.historyId === hist.id
                : 'id' in (currentResult || {})
                ? (currentResult as any).id === hist.id
                : false;

            return (
              <div
                key={hist.id}
                onClick={() => setExecutionResult(requestId, hist)}
                onContextMenu={(e) => handleContextMenu(e, hist)}
                className={`p-2 border rounded transition-colors space-y-1 font-mono text-xs cursor-pointer ${
                  isSelected
                    ? 'border-primary bg-primary/10 ring-1 ring-primary/40'
                    : 'border-border bg-background hover:border-primary/50'
                }`}
                title="Left click to view response, Right click for copy & restore options"
              >
                <div className="flex items-center justify-between">
                  <StatusBadge code={hist.statusCode} />
                  <span className="text-[10px] text-muted-foreground">{new Date(hist.executedAtMs).toLocaleTimeString()}</span>
                </div>
                <div className="flex items-center justify-between text-[11px] text-muted-foreground">
                  <span>Latency: {hist.durationMs}ms</span>
                  <span>Size: {hist.responseBody?.length || 0}B</span>
                </div>
                <div className="text-foreground text-[11px] truncate">{hist.url}</div>
              </div>
            );
          })
        )}
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getContextMenuItems(contextMenu.item)}
          onClose={() => setContextMenu(null)}
        />
      )}

      {/* Copy Custom Modal */}
      <CopyCustomModal
        isOpen={!!customCopyItem}
        item={customCopyItem}
        onClose={() => setCustomCopyItem(null)}
      />
    </div>
  );
};
