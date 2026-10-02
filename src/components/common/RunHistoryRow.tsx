import React from 'react';
import { StatusBadge } from './StatusBadge';
import { MingCuteIcon } from './MingCuteIcon';
import { runPath, type RunRecord } from '../../utils/repeaterTraffic';

interface RunHistoryRowProps {
  run: RunRecord;
  isSelected: boolean;
  onSelect: () => void;
  onOpenDetails: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

/** Flat list row for one run in a history drawer: status, timing, and the path it was sent to. */
export const RunHistoryRow: React.FC<RunHistoryRowProps> = ({ run, isSelected, onSelect, onOpenDetails, onContextMenu }) => (
  <div
    onClick={onSelect}
    onDoubleClick={onOpenDetails}
    onContextMenu={onContextMenu}
    className={`group px-3 py-2 border-l-2 transition-colors font-mono text-xs cursor-pointer flex flex-col gap-1 ${
      isSelected ? 'bg-primary/10 border-l-primary' : 'border-l-transparent hover:bg-neutral-subtle'
    }`}
    title="Click to view the response, double-click for details, right-click for more"
  >
    <div className="flex items-center gap-1.5">
      <StatusBadge code={run.statusCode} />
      <span className="text-[10px] text-muted-foreground">{run.durationMs}ms</span>
      <span className="ml-auto text-[10px] text-muted-foreground">
        {new Date(run.executedAtMs).toLocaleTimeString()}
      </span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onOpenDetails();
        }}
        className="opacity-0 group-hover:opacity-100 -my-1 p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle transition-opacity cursor-pointer"
        title="Open details"
      >
        <MingCuteIcon name="expand_line" size={12} />
      </button>
    </div>
    <div className="flex items-center gap-1.5 min-w-0">
      <span className="text-[10px] font-bold text-muted-foreground shrink-0">{run.method}</span>
      <span className="text-[11px] text-foreground truncate" title={run.url}>
        {runPath(run.url)}
      </span>
    </div>
  </div>
);
