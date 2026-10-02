import React, { useEffect, useRef } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { CollectionTreeSidebar } from './CollectionTreeSidebar';
import { CollectionRequestTab } from './CollectionRequestTab';
import { CollectionResponseViewer } from './CollectionResponseViewer';
import { CollectionHistoryDrawer } from './CollectionHistoryDrawer';
import { MethodBadge } from '../../common/MethodBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { COLLECTIONS_HISTORY_DRAWER_WIDTH, COLLECTIONS_SIDEBAR_WIDTH } from '../../../stores/uiPrefs/registry';
import { clamp, clampPercent, startDragResize } from '../../../utils/dragResize';

export const CollectionsView: React.FC = () => {
  const { activeWorkspaceId } = useWorkspaceStore();
  const {
    openRequests,
    activeRequestId,
    setActiveRequestId,
    closeRequestTab,
    fetchCollections,
    fetchRunHistory,
    isHistoryDrawerOpen,
  } = useCollectionStore();

  // Resizable panel dimensions, persisted as UI preferences
  const [sidebarWidthPx, setSidebarWidthPx, resetSidebarWidthPx] = useUiPref('collections.sidebarWidth');
  const [reqWidthPercent, setReqWidthPercent, resetReqWidthPercent] = useUiPref('collections.requestSplitPercent');
  const [historyDrawerWidthPx, setHistoryDrawerWidthPx, resetHistoryDrawerWidthPx] = useUiPref('collections.historyDrawerWidth');

  const containerRef = useRef<HTMLDivElement>(null);
  const mainAreaRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLDivElement>(null);
  const requestPaneRef = useRef<HTMLDivElement>(null);
  const responsePaneRef = useRef<HTMLDivElement>(null);
  const historyDrawerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (activeWorkspaceId) {
      fetchCollections(activeWorkspaceId);
    }
  }, [activeWorkspaceId, fetchCollections]);

  // The response pane shows the latest recorded run, so load runs for whichever tab is active
  useEffect(() => {
    if (activeRequestId) {
      fetchRunHistory(activeRequestId);
    }
  }, [activeRequestId, fetchRunHistory]);

  const activeRequest = openRequests.find((r) => r.id === activeRequestId) || null;

  const handleSidebarPointerDown = (e: React.PointerEvent) => {
    let width = sidebarWidthPx;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!containerRef.current || !sidebarRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        width = Math.round(clamp(ev.clientX - rect.left, COLLECTIONS_SIDEBAR_WIDTH.min, COLLECTIONS_SIDEBAR_WIDTH.max));
        sidebarRef.current.style.width = `${width}px`;
      },
      onEnd: () => setSidebarWidthPx(width),
    });
  };

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

  const handleHistoryDrawerPointerDown = (e: React.PointerEvent) => {
    let width = historyDrawerWidthPx;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!containerRef.current || !historyDrawerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        width = Math.round(
          clamp(rect.right - ev.clientX, COLLECTIONS_HISTORY_DRAWER_WIDTH.min, COLLECTIONS_HISTORY_DRAWER_WIDTH.max)
        );
        historyDrawerRef.current.style.width = `${width}px`;
      },
      onEnd: () => setHistoryDrawerWidthPx(width),
    });
  };

  return (
    <div ref={containerRef} className="h-full flex bg-background overflow-hidden text-xs">
      {/* Left Collection Tree Sidebar */}
      <div ref={sidebarRef} className="h-full shrink-0" style={{ width: `${sidebarWidthPx}px` }}>
        <CollectionTreeSidebar />
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

      {/* Main Request Tab & Editor Workspace */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Top Request Tabs Header */}
        <div className="bg-header border-b border-border flex items-center gap-1 px-2 pt-1 overflow-x-auto no-scrollbar shrink-0">
          {openRequests.map((req) => {
            const isActive = req.id === activeRequestId;
            return (
              <div
                key={req.id}
                onClick={() => setActiveRequestId(req.id)}
                onAuxClick={(e) => {
                  if (e.button === 1) {
                    e.preventDefault();
                    e.stopPropagation();
                    closeRequestTab(req.id);
                  }
                }}
                onMouseDown={(e) => {
                  if (e.button === 1) {
                    e.preventDefault();
                  }
                }}
                className={`group flex items-center gap-1.5 px-3 py-1 rounded-t-lg border-t border-x text-xs cursor-pointer font-mono transition-colors min-w-[120px] max-w-[220px] ${
                  isActive
                    ? 'bg-surface border-border text-foreground font-semibold shadow-xs'
                    : 'bg-surface/60 border-border/50 text-foreground/80 hover:bg-neutral-subtle hover:text-foreground'
                }`}
              >
                <MethodBadge method={req.method} />
                <span className="truncate flex-1 font-sans font-medium">{req.name}</span>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    closeRequestTab(req.id);
                  }}
                  className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-opacity cursor-pointer shrink-0"
                  title="Close Tab (×)"
                >
                  <MingCuteIcon name="close_line" size={12} />
                </button>
              </div>
            );
          })}

          {openRequests.length === 0 && (
            <div className="text-muted-foreground text-xs italic py-1 px-2">
              Select or create a collection request from the sidebar tree to open tabs.
            </div>
          )}
        </div>

        {/* Editor & Response Split View */}
        <div ref={mainAreaRef} className="flex-1 flex overflow-hidden">
          {activeRequest ? (
            <>
              {/* Request Editor */}
              <div
                ref={requestPaneRef}
                className="border-r border-border flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${reqWidthPercent}%` }}
              >
                <CollectionRequestTab request={activeRequest} />
              </div>

              {/* Resizer Handle */}
              <div
                onPointerDown={handleMainSplitPointerDown}
                onDoubleClick={resetReqWidthPercent}
                className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                title="Drag to resize, double-click to reset"
              >
                <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
              </div>

              {/* Response Inspector */}
              <div
                ref={responsePaneRef}
                className="flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${100 - reqWidthPercent}%` }}
              >
                <CollectionResponseViewer requestId={activeRequest.id} />
              </div>

              {/* Draggable Resizer Handle for Run History Drawer */}
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
                  <CollectionHistoryDrawer requestId={activeRequest.id} />
                </div>
              )}
            </>
          ) : (
            <div className="h-full flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground italic text-xs p-6">
              <MingCuteIcon name="folder_open_line" size={40} className="mb-2 opacity-30" />
              <span>No collection request tab open.</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
