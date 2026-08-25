import React, { useState, useRef } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { RequestBuilder } from './RequestBuilder';
import { ResponsePanel } from './ResponsePanel';
import { ExecutionHistoryDrawer } from './ExecutionHistoryDrawer';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const RepeaterView: React.FC = () => {
  const {
    requests,
    createNewRequest,
    deleteRequest,
    lastExecutionResponse,
  } = useRepeaterStore();

  const [selectedScratchId, setSelectedScratchId] = useState<string | null>(
    requests.length > 0 ? requests[0].id : null
  );
  const [searchQuery, setSearchQuery] = useState('');

  // Resizable panel dimensions
  const [sidebarWidthPx, setSidebarWidthPx] = useState<number>(288);
  const [reqWidthPercent, setReqWidthPercent] = useState<number>(50);
  const [historyDrawerWidthPx, setHistoryDrawerWidthPx] = useState<number>(288);

  const containerRef = useRef<HTMLDivElement>(null);
  const mainAreaRef = useRef<HTMLDivElement>(null);

  const isResizingSidebar = useRef(false);
  const isResizingMain = useRef(false);
  const isResizingHistoryDrawer = useRef(false);

  const { isHistoryDrawerOpen } = useRepeaterStore();

  const handleMouseDownSidebarSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingSidebar.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingSidebar.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = moveEvent.clientX - rect.left;
      setSidebarWidthPx(Math.min(Math.max(newWidth, 160), 480));
    };

    const handleMouseUp = () => {
      isResizingSidebar.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

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

  // Filter requests for the left sidebar history-like list
  const scratchRequests = requests.filter(
    (r) =>
      r.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      r.url.toLowerCase().includes(searchQuery.toLowerCase()) ||
      r.method.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const activeRequest = requests.find((r) => r.id === (selectedScratchId || (scratchRequests[0]?.id))) || null;
  const activeResponse = activeRequest ? lastExecutionResponse[activeRequest.id] || null : null;

  const handleNewScratch = async () => {
    await createNewRequest(null);
    const store = useRepeaterStore.getState();
    if (store.activeTabId) {
      setSelectedScratchId(store.activeTabId);
    }
  };

  return (
    <div ref={containerRef} className="h-full flex bg-background overflow-hidden select-none text-xs">
      {/* Left Sidebar: HTTP Viewer-style Request History / Scratch List (Resizable Width) */}
      <aside
        className="bg-surface border-r border-border h-full flex flex-col overflow-hidden shrink-0"
        style={{ width: `${sidebarWidthPx}px` }}
      >
        {/* Sidebar Header */}
        <div className="p-2.5 bg-header border-b border-border flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 font-semibold text-foreground">
            <MingCuteIcon name="repeat_line" size={16} className="text-primary" />
            <span>Repeater History ({requests.length})</span>
          </div>

          <button
            onClick={handleNewScratch}
            className="flex items-center gap-1 px-2 py-1 rounded bg-primary text-primary-foreground font-medium hover:bg-primary-hover transition-colors shadow-xs text-[11px]"
            title="Create New Repeater Scratch Request"
          >
            <MingCuteIcon name="plus_line" size={13} />
            <span>New</span>
          </button>
        </div>

        {/* Search Bar */}
        <div className="p-2 bg-header border-b border-border">
          <div className="relative flex items-center">
            <MingCuteIcon name="search_line" size={13} className="absolute left-2 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Filter repeater items..."
              className="w-full bg-background border border-border rounded pl-7 pr-2 py-1 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary font-mono"
            />
          </div>
        </div>

        {/* Requests List (HTTP Viewer Style) */}
        <div className="flex-1 overflow-y-auto divide-y divide-border/60">
          {scratchRequests.length === 0 ? (
            <div className="p-4 text-center text-muted-foreground italic">
              No repeater scratch requests logged
            </div>
          ) : (
            scratchRequests.map((req, idx) => {
              const isSelected = activeRequest?.id === req.id;
              const lastResp = lastExecutionResponse[req.id];

              return (
                <div
                  key={req.id}
                  onClick={() => setSelectedScratchId(req.id)}
                  className={`p-2.5 cursor-pointer font-mono text-xs transition-colors group ${
                    isSelected
                      ? 'bg-primary/15 font-semibold text-foreground border-l-2 border-primary'
                      : 'hover:bg-neutral-subtle'
                  }`}
                >
                  <div className="flex items-center justify-between gap-1 mb-1">
                    <div className="flex items-center gap-1.5">
                      <span className="text-[10px] text-muted-foreground">#{idx + 1}</span>
                      <MethodBadge method={req.method} />
                    </div>
                    {lastResp ? (
                      <StatusBadge code={lastResp.statusCode} />
                    ) : (
                      <span className="text-[10px] text-muted-foreground font-sans uppercase">Staged</span>
                    )}
                  </div>

                  <div className="flex items-center justify-between gap-1">
                    <div className="text-foreground text-xs font-sans truncate font-medium">{req.name}</div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteRequest(req.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5"
                      title="Delete repeater item"
                    >
                      <MingCuteIcon name="close_line" size={12} />
                    </button>
                  </div>

                  <div className="text-muted-foreground text-[10px] truncate mt-0.5">{req.url}</div>
                </div>
              );
            })
          )}
        </div>
      </aside>

      {/* INVISIBLE DRAGGABLE RESIZER HANDLE FOR SIDEBAR */}
      <div
        onMouseDown={handleMouseDownSidebarSplit}
        className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
        title="Drag to adjust sidebar width"
      >
        <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
      </div>

      {/* Main Area: Split Request Editor (Left) & Response Inspector (Right) */}
      <div ref={mainAreaRef} className="flex-1 flex overflow-hidden">
        {activeRequest ? (
          <div className="flex-1 flex overflow-hidden">
            {/* Request Builder Panel (Resizable Width + Division Border) */}
            <div
              className="border-r border-border flex flex-col overflow-hidden min-w-[200px]"
              style={{ width: `${reqWidthPercent}%` }}
            >
              <RequestBuilder request={activeRequest} />
            </div>

            {/* INVISIBLE DRAGGABLE RESIZER HANDLE FOR REQUEST VS RESPONSE */}
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

            {/* INVISIBLE DRAGGABLE RESIZER HANDLE FOR EXECUTION HISTORY DRAWER */}
            {isHistoryDrawerOpen && (
              <div
                onMouseDown={handleMouseDownHistoryDrawerSplit}
                className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                title="Drag to adjust Execution History width"
              >
                <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
              </div>
            )}

            <ExecutionHistoryDrawer requestId={activeRequest.id} widthPx={historyDrawerWidthPx} />
          </div>
        ) : (
          <div className="h-full flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground italic text-xs p-6">
            <MingCuteIcon name="repeat_line" size={40} className="mb-2 opacity-30" />
            No repeater item selected. Click "+ New" in the sidebar to create a scratch request.
          </div>
        )}
      </div>
    </div>
  );
};
