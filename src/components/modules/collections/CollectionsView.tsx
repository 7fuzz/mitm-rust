import React, { useEffect, useState, useRef } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { CollectionTreeSidebar } from './CollectionTreeSidebar';
import { CollectionRequestTab } from './CollectionRequestTab';
import { CollectionResponseViewer } from './CollectionResponseViewer';
import { MethodBadge } from '../../common/MethodBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const CollectionsView: React.FC = () => {
  const { activeWorkspaceId } = useWorkspaceStore();
  const {
    openRequests,
    activeRequestId,
    setActiveRequestId,
    closeRequestTab,
    fetchCollections,
  } = useCollectionStore();

  const [sidebarWidthPx, setSidebarWidthPx] = useState(290);
  const [reqWidthPercent, setReqWidthPercent] = useState(50);

  const containerRef = useRef<HTMLDivElement>(null);
  const mainAreaRef = useRef<HTMLDivElement>(null);
  const isResizingSidebar = useRef(false);
  const isResizingMain = useRef(false);

  useEffect(() => {
    if (activeWorkspaceId) {
      fetchCollections(activeWorkspaceId);
    }
  }, [activeWorkspaceId, fetchCollections]);

  const activeRequest = openRequests.find((r) => r.id === activeRequestId) || null;

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

  return (
    <div ref={containerRef} className="h-full flex bg-background overflow-hidden select-none text-xs">
      {/* Left Collection Tree Sidebar */}
      <CollectionTreeSidebar widthPx={sidebarWidthPx} />

      {/* Draggable Resizer Handle for Sidebar */}
      <div
        onMouseDown={handleMouseDownSidebarSplit}
        className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
        title="Drag to adjust Sidebar width"
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
                className="border-r border-border flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${reqWidthPercent}%` }}
              >
                <CollectionRequestTab request={activeRequest} />
              </div>

              {/* Resizer Handle */}
              <div
                onMouseDown={handleMouseDownMainSplit}
                className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
                title="Drag to adjust Request vs Response width"
              >
                <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
              </div>

              {/* Response Inspector */}
              <div
                className="flex flex-col overflow-hidden min-w-[200px]"
                style={{ width: `${100 - reqWidthPercent}%` }}
              >
                <CollectionResponseViewer requestId={activeRequest.id} />
              </div>
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
