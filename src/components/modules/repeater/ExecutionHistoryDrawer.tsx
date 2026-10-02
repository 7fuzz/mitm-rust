import React, { useEffect, useState } from 'react';
import type { RepeaterHistoryItem } from '../../../services/tauri/bridge';
import type { TrafficItem } from '../../../types';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { RunHistoryRow } from '../../common/RunHistoryRow';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';
import { CopyCustomModal } from '../history/CopyCustomModal';
import { historyItemToTrafficItem, RUN_COPY_ACTIONS } from '../../../utils/repeaterTraffic';
import { RunDetailsDialog } from '../../common/RunDetailsDialog';

interface ExecutionHistoryDrawerProps {
  requestId: string;
}

export const ExecutionHistoryDrawer: React.FC<ExecutionHistoryDrawerProps> = ({ requestId }) => {
  const {
    executionHistory,
    isHistoryDrawerOpen,
    setHistoryDrawerOpen,
    fetchHistory,
    setExecutionResult,
    restoreHistoryToTab,
    executeActiveRequest,
    lastExecutionResult,
    tabs,
  } = useRepeaterStore();

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: RepeaterHistoryItem } | null>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);
  const [dialogIndex, setDialogIndex] = useState<number | null>(null);

  useEffect(() => {
    if (isHistoryDrawerOpen && requestId) {
      fetchHistory(requestId);
    }
  }, [isHistoryDrawerOpen, requestId, fetchHistory]);

  if (!isHistoryDrawerOpen) return null;

  const history = executionHistory[requestId] || [];
  const currentResult = lastExecutionResult[requestId];

  // Edits made after the latest run would be lost by loading an older one
  const tab = tabs.find((t) => t.id === requestId);
  const hasUnsentChanges = !!tab && (history.length === 0 || tab.updatedAtMs > history[0].executedAtMs);

  const handleLoadAndSend = async (run: RepeaterHistoryItem) => {
    await restoreHistoryToTab(requestId, run);
    await executeActiveRequest(requestId);
  };

  const handleContextMenu = (e: React.MouseEvent, hist: RepeaterHistoryItem) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, item: hist });
  };

  const getContextMenuItems = (hist: RepeaterHistoryItem): ContextMenuItem[] => {
    const trafficItem = historyItemToTrafficItem(hist);
    return [
      {
        label: 'Open Details',
        icon: 'expand_line',
        action: () => setDialogIndex(history.indexOf(hist)),
      },
      {
        label: 'View Response in Panel',
        icon: 'eye_line',
        action: () => setExecutionResult(requestId, hist),
      },
      {
        label: 'Load into Editor',
        icon: 'refresh_line',
        action: () => restoreHistoryToTab(requestId, hist),
      },
      {
        label: 'Copy',
        icon: 'copy_line',
        children: [
          ...RUN_COPY_ACTIONS.map((copyAction) => ({
            label: copyAction.label,
            icon: copyAction.icon,
            action: () => navigator.clipboard.writeText(copyAction.format(trafficItem)),
          })),
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
      className="bg-surface border-l border-border h-full w-full flex flex-col overflow-hidden text-xs select-none"
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

      <div className="flex-1 overflow-y-auto divide-y divide-border/50">
        {history.length === 0 ? (
          <div className="py-8 text-center text-muted-foreground italic">No historical executions logged</div>
        ) : (
          history.map((hist, index) => {
            const isSelected =
              currentResult &&
              'historyId' in currentResult
                ? currentResult.historyId === hist.id
                : 'id' in (currentResult || {})
                ? (currentResult as any).id === hist.id
                : false;

            return (
              <RunHistoryRow
                key={hist.id}
                run={hist}
                isSelected={!!isSelected}
                onSelect={() => setExecutionResult(requestId, hist)}
                onOpenDetails={() => setDialogIndex(index)}
                onContextMenu={(e) => handleContextMenu(e, hist)}
              />
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

      <RunDetailsDialog
        runs={history}
        index={dialogIndex}
        onIndexChange={setDialogIndex}
        onClose={() => setDialogIndex(null)}
        hasUnsentChanges={hasUnsentChanges}
        onLoad={(run) => restoreHistoryToTab(requestId, run)}
        onLoadAndSend={handleLoadAndSend}
      />

      {/* Copy Custom Modal */}
      <CopyCustomModal
        isOpen={!!customCopyItem}
        item={customCopyItem}
        onClose={() => setCustomCopyItem(null)}
      />
    </div>
  );
};
