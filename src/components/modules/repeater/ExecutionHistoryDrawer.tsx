import React from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { StatusBadge } from '../../common/StatusBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';

interface ExecutionHistoryDrawerProps {
  requestId: string;
  widthPx?: number;
}

export const ExecutionHistoryDrawer: React.FC<ExecutionHistoryDrawerProps> = ({ requestId, widthPx = 288 }) => {
  const { executionHistory, isHistoryDrawerOpen, setHistoryDrawerOpen } = useRepeaterStore();

  if (!isHistoryDrawerOpen) return null;

  const history = executionHistory[requestId] || [];

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
        <button className="text-muted-foreground group-hover:text-foreground p-0.5 rounded">
          <MingCuteIcon name="close_line" size={14} />
        </button>
      </div>

      <div className="flex-1 p-2 overflow-y-auto space-y-2">
        {history.length === 0 ? (
          <div className="py-8 text-center text-muted-foreground italic">No historical executions logged</div>
        ) : (
          history.map((hist) => (
            <div
              key={hist.id}
              className="p-2 border border-border rounded bg-background hover:border-primary/50 transition-colors space-y-1 font-mono text-xs"
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
          ))
        )}
      </div>
    </div>
  );
};
