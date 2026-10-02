import React, { useRef, useEffect } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { RequestBuilder } from './RequestBuilder';
import { ResponsePanel } from './ResponsePanel';
import { ExecutionHistoryDrawer } from './ExecutionHistoryDrawer';
import { RepeaterSidebar } from './RepeaterSidebar';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { REPEATER_HISTORY_DRAWER_WIDTH, REPEATER_SIDEBAR_WIDTH } from '../../../stores/uiPrefs/registry';
import { clamp, clampPercent, startDragResize } from '../../../utils/dragResize';

export const RepeaterView: React.FC = () => {
  const {
    tabs,
    activeTabId,
    createNewRequest,
    lastExecutionResult,
    initStore,
    isHistoryDrawerOpen,
  } = useRepeaterStore();

  // Resizable panel dimensions
  // Resizable panel dimensions, persisted as UI preferences
  const [reqWidthPercent, setReqWidthPercent, resetReqWidthPercent] = useUiPref('repeater.requestSplitPercent');
  const [sidebarWidthPx, setSidebarWidthPx, resetSidebarWidthPx] = useUiPref('repeater.sidebarWidth');
  const [historyDrawerWidthPx, setHistoryDrawerWidthPx, resetHistoryDrawerWidthPx] = useUiPref('repeater.historyDrawerWidth');

  const containerRef = useRef<HTMLDivElement>(null);
  const mainAreaRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLDivElement>(null);
  const requestPaneRef = useRef<HTMLDivElement>(null);
  const responsePaneRef = useRef<HTMLDivElement>(null);
  const historyDrawerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    initStore();
  }, [initStore]);

  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0] || null;
  const activeResponse = activeTab ? lastExecutionResult[activeTab.id] || null : null;

  const handleMainSplitPointerDown = (e: React.PointerEvent) => {
    let percent = reqWidthPercent;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!mainAreaRef.current || !requestPaneRef.current || !responsePaneRef.current) return;
        const rect = mainAreaRef.current.getBoundingClientRect();
        percent = clampPercent(((ev.clientX - rect.left) / rect.width) * 100);
        requestPaneRef.current.style.width = `${percent}%`;
        responsePaneRef.current.style.width = `${100 - percent}%`;
      },
      onEnd: () => setReqWidthPercent(percent),
    });
  };

  const handleSidebarPointerDown = (e: React.PointerEvent) => {
    let width = sidebarWidthPx;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!containerRef.current || !sidebarRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        width = Math.round(clamp(ev.clientX - rect.left, REPEATER_SIDEBAR_WIDTH.min, REPEATER_SIDEBAR_WIDTH.max));
        sidebarRef.current.style.width = `${width}px`;
      },
      onEnd: () => setSidebarWidthPx(width),
    });
  };

  const handleHistoryDrawerPointerDown = (e: React.PointerEvent) => {
    let width = historyDrawerWidthPx;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!containerRef.current || !historyDrawerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        width = Math.round(clamp(rect.right - ev.clientX, REPEATER_HISTORY_DRAWER_WIDTH.min, REPEATER_HISTORY_DRAWER_WIDTH.max));
        historyDrawerRef.current.style.width = `${width}px`;
      },
      onEnd: () => setHistoryDrawerWidthPx(width),
    });
  };

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden text-xs">
      {/* Main Body Area: Sidebar (Left) + Editor / Response (Right) */}
      <div className="flex-1 flex overflow-hidden">
        <div ref={sidebarRef} className="h-full shrink-0" style={{ width: `${sidebarWidthPx}px` }}>
          <RepeaterSidebar />
        </div>

        {/* Draggable Resizer Handle for Sidebar */}
        <div
          onPointerDown={handleSidebarPointerDown}
          onDoubleClick={resetSidebarWidthPx}
          className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
          title="Drag to resize, double-click to reset"
        >
          <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
        </div>

        {/* Main Work Area: Request Builder & Response Inspector */}
        <div ref={mainAreaRef} className="flex-1 flex overflow-hidden">
          {activeTab ? (
            <div className="flex-1 flex overflow-hidden">
              {/* Request Builder Panel */}
              <div
                ref={requestPaneRef}
                className="border-r border-border flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${reqWidthPercent}%` }}
              >
                <RequestBuilder request={activeTab} />
              </div>

              {/* Draggable Resizer Handle for Request vs Response */}
              <div
                onPointerDown={handleMainSplitPointerDown}
                onDoubleClick={resetReqWidthPercent}
                className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                title="Drag to resize, double-click to reset"
              >
                <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
              </div>

              {/* Response Panel */}
              <div
                ref={responsePaneRef}
                className="flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${100 - reqWidthPercent}%` }}
              >
                <ResponsePanel response={activeResponse} />
              </div>

              {/* Draggable Resizer Handle for Execution History Drawer */}
              {isHistoryDrawerOpen && (
                <div
                  onPointerDown={handleHistoryDrawerPointerDown}
                  onDoubleClick={resetHistoryDrawerWidthPx}
                  className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                  title="Drag to resize, double-click to reset"
                >
                  <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
                </div>
              )}

              {isHistoryDrawerOpen && (
                <div ref={historyDrawerRef} className="h-full shrink-0" style={{ width: `${historyDrawerWidthPx}px` }}>
                  <ExecutionHistoryDrawer requestId={activeTab.id} />
                </div>
              )}
            </div>
          ) : (
            <div className="h-full flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground italic text-xs p-6">
              <MingCuteIcon name="send_plane_line" size={40} className="mb-2 opacity-30" />
              <span>No repeater request selected.</span>
              <button
                onClick={() => createNewRequest()}
                className="mt-3 px-3 py-1.5 bg-primary hover:bg-primary/90 text-primary-foreground rounded font-sans not-italic text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer shadow-2xs"
              >
                <MingCuteIcon name="plus_line" size={14} />
                <span>Create New Request</span>
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
