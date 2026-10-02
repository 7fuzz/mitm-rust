import React, { useEffect, useState } from 'react';
import type { RequestHistoryItem } from '../../../services/tauri/bridge';
import type { TrafficItem } from '../../../types';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { RunHistoryRow } from '../../common/RunHistoryRow';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';
import { CopyCustomModal } from '../history/CopyCustomModal';
import { RunDetailsDialog } from '../../common/RunDetailsDialog';
import { historyItemToTrafficItem, RUN_COPY_ACTIONS } from '../../../utils/repeaterTraffic';

interface CollectionHistoryDrawerProps {
  requestId: string;
}

/**
 * Run history of a collection request. Runs record the request with variables resolved,
 * so there is no "load into editor" here: it would overwrite the request's {{templates}}.
 */
export const CollectionHistoryDrawer: React.FC<CollectionHistoryDrawerProps> = ({ requestId }) => {
  const { runHistory, selectedRunId, selectRun, clearRunHistory, toggleHistoryDrawer } = useCollectionStore();

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: RequestHistoryItem } | null>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);
  const [dialogIndex, setDialogIndex] = useState<number | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    setConfirmClear(false);
  }, [requestId]);

  const history = runHistory[requestId] || [];
  const selectedId = selectedRunId[requestId] ?? history[0]?.id;

  const handleClear = async () => {
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    setConfirmClear(false);
    await clearRunHistory(requestId);
  };

  const getContextMenuItems = (run: RequestHistoryItem): ContextMenuItem[] => {
    const trafficItem = historyItemToTrafficItem(run);
    return [
      {
        label: 'Open Details',
        icon: 'expand_line',
        action: () => setDialogIndex(history.indexOf(run)),
      },
      {
        label: 'View Response in Panel',
        icon: 'eye_line',
        action: () => selectRun(requestId, run.id),
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
    <div className="bg-surface border-l border-border h-full w-full flex flex-col overflow-hidden text-xs select-none">
      <div className="h-12 px-3 bg-header border-b border-border font-semibold flex items-center gap-2 shrink-0">
        <MingCuteIcon name="history_line" size={16} className="text-primary" />
        <span>Run History ({history.length})</span>
        <div className="ml-auto flex items-center gap-1">
          {history.length > 0 && (
            <button
              onClick={handleClear}
              onMouseLeave={() => setConfirmClear(false)}
              className={`px-1.5 py-0.5 rounded text-[11px] font-medium cursor-pointer transition-colors flex items-center gap-1 ${
                confirmClear
                  ? 'bg-rose-500/15 text-rose-500'
                  : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
              }`}
              title="Delete all recorded runs of this request"
            >
              <MingCuteIcon name="delete_2_line" size={13} />
              {confirmClear && <span>Clear all?</span>}
            </button>
          )}
          <button
            onClick={() => toggleHistoryDrawer(false)}
            className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
            title="Close Run History"
          >
            <MingCuteIcon name="close_line" size={14} />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto divide-y divide-border/50">
        {history.length === 0 ? (
          <div className="py-8 text-center text-muted-foreground italic">No runs recorded yet</div>
        ) : (
          history.map((run, index) => (
            <RunHistoryRow
              key={run.id}
              run={run}
              isSelected={run.id === selectedId}
              // Picking the newest run follows future sends again
              onSelect={() => selectRun(requestId, index === 0 ? undefined : run.id)}
              onOpenDetails={() => setDialogIndex(index)}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, item: run });
              }}
            />
          ))
        )}
      </div>

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
      />

      <CopyCustomModal isOpen={!!customCopyItem} item={customCopyItem} onClose={() => setCustomCopyItem(null)} />
    </div>
  );
};
