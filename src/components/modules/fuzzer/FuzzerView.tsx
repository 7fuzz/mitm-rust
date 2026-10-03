import React, { useEffect, useRef, useState } from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { clamp, startDragResize } from '../../../utils/dragResize';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';
import { FuzzerRequestEditor } from './FuzzerRequestEditor';
import { MatchRulesConfig, OptionsConfig, PayloadsConfig } from './FuzzerPayloadPanel';
import { FuzzerResultsTable } from './FuzzerResultsTable';
import { FuzzerRunDialog } from './FuzzerRunDialog';
import type { FuzzRunMeta } from '../../../services/tauri/bridge';

const CONFIG_H = { min: 160, max: 700 };

type ConfigTab = 'request' | 'payloads' | 'match' | 'options';

export const FuzzerView: React.FC = () => {
  const {
    phase,
    estimate,
    estimateError,
    variables,
    attackType,
    config,
    rows,
    total,
    savedToDb,
    startError,
    savedRuns,
    start,
    stop,
    saveRun,
    initListeners,
    fetchSavedRuns,
    openSavedRun,
    removeSavedRun,
  } = useFuzzerStore();
  const [runDialog, setRunDialog] = useState<'start' | 'save' | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [tab, setTab] = useState<ConfigTab>('request');
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; run: FuzzRunMeta } | null>(null);
  const [configHeight, setConfigHeight] = useUiPref('fuzzer.configHeight');
  const configRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    initListeners().then((d) => {
      if (active) dispose = d;
      else d();
    });
    fetchSavedRuns();
    return () => {
      active = false;
      dispose?.();
    };
  }, [initListeners, fetchSavedRuns]);

  const isRunning = phase === 'running';
  const canRun = variables.length > 0 && !estimateError && (estimate ?? 0) > 0;
  const progress = total > 0 ? Math.round((rows.length / total) * 100) : 0;

  const handleSplitPointerDown = (e: React.PointerEvent) => {
    const panel = configRef.current;
    if (!panel) return;
    const top = panel.getBoundingClientRect().top;
    let height = configHeight;
    startDragResize(e, {
      cursor: 'row-resize',
      onMove: (ev) => {
        height = Math.round(clamp(ev.clientY - top, CONFIG_H.min, CONFIG_H.max));
        panel.style.height = `${height}px`;
      },
      onEnd: () => setConfigHeight(height),
    });
  };

  const runMenuItems = (run: FuzzRunMeta): ContextMenuItem[] => [
    { label: 'Open', icon: 'external_link_line', action: () => openSavedRun(run.id) },
    { label: 'Delete', icon: 'delete_2_line', danger: true, action: () => removeSavedRun(run.id) },
  ];

  const startRun = (name: string, save: boolean) => {
    useFuzzerStore.setState({ runName: name });
    start(save);
  };

  const tabs: { value: ConfigTab; label: string; count?: number }[] = [
    { value: 'request', label: 'Request' },
    { value: 'payloads', label: 'Payloads', count: variables.length },
    { value: 'match', label: 'Match', count: config.matchRules.length },
    { value: 'options', label: 'Options' },
  ];

  return (
    <div className="h-full flex bg-background overflow-hidden text-xs">
      {sidebarOpen && (
        <div className="w-56 shrink-0 bg-surface border-r border-border flex flex-col">
          <div className="h-10 px-3 flex items-center border-b border-border">
            <span className="flex-1 text-3xs font-semibold uppercase tracking-wider text-muted-foreground">
              Saved runs <span className="font-mono">{savedRuns.length}</span>
            </span>
            <button onClick={() => fetchSavedRuns()} className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer" title="Refresh">
              <MingCuteIcon name="refresh_line" size={13} />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto p-1.5 space-y-0.5">
            {savedRuns.map((run) => (
              <button
                key={run.id}
                onClick={() => openSavedRun(run.id)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setContextMenu({ x: e.clientX, y: e.clientY, run });
                }}
                className="w-full text-left px-2 py-1.5 rounded hover:bg-neutral-subtle cursor-pointer group"
              >
                <div className="flex items-center gap-1.5">
                  <span className="flex-1 truncate text-xs text-foreground font-medium">{run.name}</span>
                  <span className="text-3xs font-mono text-muted-foreground">{run.total}</span>
                </div>
                <div className="text-3xs text-muted-foreground font-mono">
                  {run.attackType} · {new Date(run.createdAtMs).toLocaleString()}
                </div>
              </button>
            ))}
            {savedRuns.length === 0 && <div className="px-2 py-4 text-center text-2xs text-muted-foreground italic">No saved runs</div>}
          </div>
        </div>
      )}

      <div className="flex-1 min-w-0 flex flex-col">
        <div className="h-10 px-2 bg-header border-b border-border flex items-center gap-1 shrink-0">
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className={`p-1.5 rounded cursor-pointer ${sidebarOpen ? 'bg-primary/10 text-primary' : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'}`}
            title="Saved runs"
          >
            <MingCuteIcon name="history_line" size={15} />
          </button>

          <div className="w-px h-5 bg-border mx-1" />

          <div className="flex items-center gap-0.5">
            {tabs.map((t) => (
              <button
                key={t.value}
                onClick={() => setTab(t.value)}
                className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-medium cursor-pointer transition-colors ${
                  tab === t.value ? 'bg-background text-foreground' : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                {t.label}
                {t.count !== undefined && t.count > 0 && (
                  <span className="text-3xs font-mono text-muted-foreground tabular-nums">{t.count}</span>
                )}
              </button>
            ))}
          </div>

          <div className="ml-auto flex items-center gap-2">
            {estimateError ? (
              <span className="text-2xs text-rose-500 font-mono max-w-xs truncate" title={estimateError}>{estimateError}</span>
            ) : (
              <span className="text-2xs text-muted-foreground font-mono">
                {estimate !== null ? `${estimate.toLocaleString()} requests` : ''}
              </span>
            )}

            {isRunning ? (
              <>
                <div className="w-32 h-1.5 rounded-full bg-neutral-subtle overflow-hidden">
                  <div className="h-full bg-primary transition-all" style={{ width: `${progress}%` }} />
                </div>
                <span className="text-2xs font-mono text-muted-foreground tabular-nums">{rows.length}/{total}</span>
                <button onClick={() => stop()} className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-rose-600 text-white font-semibold cursor-pointer hover:bg-rose-500">
                  <MingCuteIcon name="pause_line" size={13} />
                  <span>Stop</span>
                </button>
              </>
            ) : (
              <button
                onClick={() => setRunDialog('start')}
                disabled={!canRun}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-primary text-primary-foreground font-semibold cursor-pointer hover:bg-primary-hover disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <MingCuteIcon name="play_line" size={13} />
                <span>Start</span>
              </button>
            )}
          </div>
        </div>

        {startError && (
          <div className="px-3 py-1.5 bg-rose-500/10 border-b border-rose-500/30 text-rose-500 text-2xs font-mono">{startError}</div>
        )}
        {(phase === 'done' || phase === 'stopped') && !savedToDb && rows.length > 0 && (
          <div className="px-3 py-1.5 bg-amber-500/10 border-b border-amber-500/30 text-2xs flex items-center gap-2">
            <MingCuteIcon name="alert_line" size={13} className="text-amber-500 shrink-0" />
            <span className="text-foreground">This run is only in memory and will be lost on restart.</span>
            <button onClick={() => setRunDialog('save')} className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded bg-amber-500 text-black font-semibold cursor-pointer hover:bg-amber-400">
              <MingCuteIcon name="download_line" size={12} />
              Save run
            </button>
          </div>
        )}

        <div
          ref={configRef}
          className="shrink-0 overflow-auto p-3"
          style={{ height: `${configHeight}px` }}
        >
          {tab === 'request' && <FuzzerRequestEditor />}
          {tab === 'payloads' && <PayloadsConfig />}
          {tab === 'match' && <MatchRulesConfig />}
          {tab === 'options' && <OptionsConfig />}
        </div>

        <div
          onPointerDown={handleSplitPointerDown}
          className="h-1 cursor-row-resize -my-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
          title="Drag to resize"
        >
          <div className="w-8 h-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
        </div>

        <div className="flex-1 min-h-0 border-t border-border">
          <FuzzerResultsTable />
        </div>
      </div>

      <FuzzerRunDialog
        mode={runDialog}
        estimate={estimate}
        variableCount={variables.length}
        attackType={attackType}
        onClose={() => setRunDialog(null)}
        onStart={startRun}
        onSave={saveRun}
      />

      {contextMenu && (
        <ContextMenu x={contextMenu.x} y={contextMenu.y} items={runMenuItems(contextMenu.run)} onClose={() => setContextMenu(null)} />
      )}
    </div>
  );
};
