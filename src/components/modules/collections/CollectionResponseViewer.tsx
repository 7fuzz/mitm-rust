import React from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ResponsePanel } from '../repeater/ResponsePanel';

interface CollectionResponseViewerProps {
  requestId: string;
}

/** Response of the run picked in the history drawer (the latest one by default). */
export const CollectionResponseViewer: React.FC<CollectionResponseViewerProps> = ({ requestId }) => {
  const latestResult = useCollectionStore((s) => s.executionResult[requestId]);
  const isLoading = useCollectionStore((s) => s.isExecuting[requestId]);
  const history = useCollectionStore((s) => s.runHistory[requestId]) || [];
  const selectedId = useCollectionStore((s) => s.selectedRunId[requestId]);
  const selectRun = useCollectionStore((s) => s.selectRun);

  if (isLoading) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs select-none">
        <MingCuteIcon name="loading_line" size={32} className="animate-spin text-primary mb-2" />
        <span>Executing request...</span>
      </div>
    );
  }

  const runIndex = selectedId !== undefined ? history.findIndex((h) => h.id === selectedId) : 0;
  const run = history[Math.max(runIndex, 0)];
  // Stored runs have no status text; the fresh result of the same run does
  const statusText = run && latestResult?.historyId === run.id ? latestResult.statusText : undefined;
  const response = run ? { ...run, statusText } : latestResult ?? null;
  const isOlderRun = !!run && runIndex > 0;

  return (
    <ResponsePanel
      response={response}
      toolbar={
        isOlderRun && (
          <div className="flex items-center gap-1 pl-2 pr-0.5 py-0.5 rounded bg-amber-500/10 border border-amber-500/30 text-amber-600 dark:text-amber-300 text-2xs">
            <MingCuteIcon name="history_line" size={12} />
            <span title={new Date(run.executedAtMs).toLocaleString()}>
              Run {history.length - runIndex} · {new Date(run.executedAtMs).toLocaleTimeString()}
            </span>
            <button
              onClick={() => selectRun(requestId, undefined)}
              className="ml-1 px-1.5 rounded font-semibold hover:bg-amber-500/20 cursor-pointer"
              title="Show the latest run"
            >
              Latest
            </button>
          </div>
        )
      }
    />
  );
};
