import React, { useEffect, useRef, useState } from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { clamp, startDragResize } from '../../../utils/dragResize';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';
import { FuzzerRequestEditor } from './FuzzerRequestEditor';
import { FuzzerPayloadPanel } from './FuzzerPayloadPanel';
import { FuzzerResultsTable } from './FuzzerResultsTable';
import type { FuzzRunMeta } from '../../../services/tauri/bridge';

const COL = { min: 360, max: 760 };

export const FuzzerView: React.FC = () => {
  const {
    runName,
    phase,
    estimate,
    estimateError,
    variables,
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
  const setRunName = (name: string) => useFuzzerStore.setState({ runName: name });

  const [startMenuOpen, setStartMenuOpen] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; run: FuzzRunMeta } | null>(null);
  const [configWidth, setConfigWidth] = useUiPref('fuzzer.configWidth');
  const configRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const startMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    initListeners();
    fetchSavedRuns();
  }, [initListeners, fetchSavedRuns]);

  useEffect(() => {
    if (!startMenuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (startMenuRef.current && !startMenuRef.current.contains(e.target as Node)) setStartMenuOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [startMenuOpen]);

  const isRunning = phase === 'running';
  const canRun = variables.length > 0 && !estimateError && (estimate ?? 0) > 0;
  const progress = total > 0 ? Math.round((rows.length / total) * 100) : 0;

  const handleSplitPointerDown = (e: React.PointerEvent) => {
    let width = configWidth;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!containerRef.current || !configRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        width = Math.round(clamp(ev.clientX - rect.left - (sidebarOpen ? 224 : 0), COL.min, COL.max));
        configRef.current.style.width = `${width}px`;
      },
      onEnd: () => setConfigWidth(width),
    });
  };

  const runMenuItems = (run: FuzzRunMeta): ContextMenuItem[] => [
    { label: 'Open', icon: 'external_link_line', action: () => openSavedRun(run.id) },
    { label: 'Delete', icon: 'delete_2_line', danger: true, action: () => removeSavedRun(run.id) },
  ];

  const promptSave = async () => {
    const name = window.prompt('Save this run as:', runName || 'Fuzz run');
    if (name !== null) await saveRun(name);
  };

  return (
    <div ref={containerRef} className="h-full flex bg-background overflow-hidden text-xs">
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
        <div className="h-12 px-3 bg-header border-b border-border flex items-center gap-2 shrink-0">
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className={`p-1.5 rounded border cursor-pointer ${sidebarOpen ? 'bg-primary/10 border-primary/40 text-primary' : 'bg-background border-border text-muted-foreground hover:text-foreground'}`}
            title="Saved runs"
          >
            <MingCuteIcon name="history_line" size={15} />
          </button>
          <input
            type="text"
            value={runName}
            onChange={(e) => setRunName(e.target.value)}
            placeholder="Fuzz run name"
            className="w-48 bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
          />

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
              <div className="relative" ref={startMenuRef}>
                <div className="flex items-center">
                  <button
                    onClick={() => start(true)}
                    disabled={!canRun}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-l bg-primary text-primary-foreground font-semibold cursor-pointer hover:bg-primary-hover disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Start and save this run to the database"
                  >
                    <MingCuteIcon name="play_line" size={13} />
                    <span>Start &amp; save</span>
                  </button>
                  <button
                    onClick={() => setStartMenuOpen(!startMenuOpen)}
                    disabled={!canRun}
                    className="px-1 py-1.5 rounded-r bg-primary text-primary-foreground border-l border-primary-foreground/20 cursor-pointer hover:bg-primary-hover disabled:opacity-50"
                  >
                    <MingCuteIcon name="down_line" size={13} />
                  </button>
                </div>
                {startMenuOpen && (
                  <div className="absolute right-0 top-full mt-1 w-48 bg-surface border border-border rounded-lg shadow-xl py-1 z-50">
                    <button
                      onClick={() => {
                        setStartMenuOpen(false);
                        start(false);
                      }}
                      className="w-full px-3 py-1.5 text-left text-xs text-foreground hover:bg-neutral-subtle cursor-pointer flex items-center gap-2"
                    >
                      <MingCuteIcon name="play_line" size={13} />
                      <span>Start temporary</span>
                    </button>
                  </div>
                )}
              </div>
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
            <button onClick={promptSave} className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded bg-amber-500 text-black font-semibold cursor-pointer hover:bg-amber-400">
              <MingCuteIcon name="download_line" size={12} />
              Save run
            </button>
          </div>
        )}

        <div className="flex-1 min-h-0 flex">
          <div ref={configRef} className="shrink-0 border-r border-border overflow-y-auto p-3 space-y-3" style={{ width: `${configWidth}px` }}>
            <FuzzerRequestEditor />
            <FuzzerPayloadPanel />
          </div>

          <div
            onPointerDown={handleSplitPointerDown}
            className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
            title="Drag to resize"
          >
            <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
          </div>

          <div className="flex-1 min-w-0">
            <FuzzerResultsTable />
          </div>
        </div>
      </div>

      {contextMenu && (
        <ContextMenu x={contextMenu.x} y={contextMenu.y} items={runMenuItems(contextMenu.run)} onClose={() => setContextMenu(null)} />
      )}
    </div>
  );
};
