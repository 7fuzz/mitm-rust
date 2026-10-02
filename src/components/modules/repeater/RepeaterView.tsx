import React, { useState, useRef, useEffect } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { RequestBuilder } from './RequestBuilder';
import { ResponsePanel } from './ResponsePanel';
import { ExecutionHistoryDrawer } from './ExecutionHistoryDrawer';
import { RepeaterSidebar } from './RepeaterSidebar';
import { MingCuteIcon } from '../../common/MingCuteIcon';

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
  const [reqWidthPercent, setReqWidthPercent] = useState<number>(50);
  const [sidebarWidthPx, setSidebarWidthPx] = useState<number>(290);
  const [historyDrawerWidthPx, setHistoryDrawerWidthPx] = useState<number>(288);

  const containerRef = useRef<HTMLDivElement>(null);
  const mainAreaRef = useRef<HTMLDivElement>(null);

  const isResizingMain = useRef(false);
  const isResizingSidebar = useRef(false);
  const isResizingHistoryDrawer = useRef(false);

  useEffect(() => {
    initStore();
  }, [initStore]);

  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0] || null;
  const activeResponse = activeTab ? lastExecutionResult[activeTab.id] || null : null;

  const handleMouseDownMainSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingMain.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingMain.current || !mainAreaRef.current) return;
      const rect = mainAreaRef.current.getBoundingClientRect();
      const relativeX = moveEvent.clientX - rect.left;
      const newPercent = (relativeX / rect.width) * 100;
      setReqWidthPercent(Math.min(Math.max(newPercent, 15), 85));
    };

    const handleMouseUp = () => {
      isResizingMain.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const handleMouseDownSidebarSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingSidebar.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingSidebar.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = moveEvent.clientX - rect.left;
      setSidebarWidthPx(Math.min(Math.max(newWidth, 220), 450));
    };

    const handleMouseUp = () => {
      isResizingSidebar.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const handleMouseDownHistoryDrawerSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingHistoryDrawer.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingHistoryDrawer.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = rect.right - moveEvent.clientX;
      setHistoryDrawerWidthPx(Math.min(Math.max(newWidth, 200), 550));
    };

    const handleMouseUp = () => {
      isResizingHistoryDrawer.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden text-xs">
      {/* Main Body Area: Sidebar (Left) + Editor / Response (Right) */}
      <div className="flex-1 flex overflow-hidden">
        <RepeaterSidebar widthPx={sidebarWidthPx} />

        {/* Draggable Resizer Handle for Sidebar */}
        <div
          onMouseDown={handleMouseDownSidebarSplit}
          className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
          title="Drag to adjust Sidebar width"
        >
          <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
        </div>

        {/* Main Work Area: Request Builder & Response Inspector */}
        <div ref={mainAreaRef} className="flex-1 flex overflow-hidden">
          {activeTab ? (
            <div className="flex-1 flex overflow-hidden">
              {/* Request Builder Panel */}
              <div
                className="border-r border-border flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${reqWidthPercent}%` }}
              >
                <RequestBuilder request={activeTab} />
              </div>

              {/* Draggable Resizer Handle for Request vs Response */}
              <div
                onMouseDown={handleMouseDownMainSplit}
                className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                title="Drag to adjust Request vs Response width"
              >
                <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
              </div>

              {/* Response Panel */}
              <div
                className="flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${100 - reqWidthPercent}%` }}
              >
                <ResponsePanel response={activeResponse} />
              </div>

              {/* Draggable Resizer Handle for Execution History Drawer */}
              {isHistoryDrawerOpen && (
                <div
                  onMouseDown={handleMouseDownHistoryDrawerSplit}
                  className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                  title="Drag to adjust Execution History width"
                >
                  <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
                </div>
              )}

              <ExecutionHistoryDrawer requestId={activeTab.id} widthPx={historyDrawerWidthPx} />
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
