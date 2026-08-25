import React, { useState, useRef, useEffect } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MethodBadge } from '../../common/MethodBadge';
import { RequestBuilder } from './RequestBuilder';
import { ResponsePanel } from './ResponsePanel';
import { ExecutionHistoryDrawer } from './ExecutionHistoryDrawer';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const RepeaterView: React.FC = () => {
  const {
    tabs,
    activeTabId,
    setActiveTab,
    createNewRequest,
    updateTab,
    deleteTab,
    lastExecutionResult,
    initStore,
    isHistoryDrawerOpen,
  } = useRepeaterStore();

  const [editingTabId, setEditingTabId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');

  // Resizable panel dimensions
  const [reqWidthPercent, setReqWidthPercent] = useState<number>(50);
  const [historyDrawerWidthPx, setHistoryDrawerWidthPx] = useState<number>(288);

  const containerRef = useRef<HTMLDivElement>(null);
  const mainAreaRef = useRef<HTMLDivElement>(null);

  const isResizingMain = useRef(false);
  const isResizingHistoryDrawer = useRef(false);

  useEffect(() => {
    initStore();
  }, [initStore]);

  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0] || null;
  const activeResponse = activeTab ? lastExecutionResult[activeTab.id] || null : null;

  const handleStartRename = (id: string, name: string) => {
    setEditingTabId(id);
    setEditingName(name);
  };

  const handleSaveRename = (tab: any) => {
    if (editingName.trim() && editingName !== tab.name) {
      updateTab({ ...tab, name: editingName.trim() });
    }
    setEditingTabId(null);
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

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden select-none text-xs">
      {/* Top Burp-Suite Style TabBar */}
      <div className="bg-header border-b border-border flex items-center gap-1 px-2 pt-1.5 overflow-x-auto shrink-0 select-none">
        {tabs.map((tab, idx) => {
          const isActive = tab.id === activeTabId;
          const isEditing = editingTabId === tab.id;

          return (
            <div
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              onDoubleClick={() => handleStartRename(tab.id, tab.name)}
              className={`group flex items-center gap-2 px-3 py-1.5 rounded-t-lg border-t border-x text-xs cursor-pointer font-mono transition-colors min-w-[120px] max-w-[200px] ${
                isActive
                  ? 'bg-surface border-border text-foreground font-semibold shadow-xs'
                  : 'bg-zinc-900/60 border-transparent text-muted-foreground hover:bg-neutral-subtle hover:text-foreground'
              }`}
            >
              <MethodBadge method={tab.method} />

              {isEditing ? (
                <input
                  type="text"
                  autoFocus
                  value={editingName}
                  onChange={(e) => setEditingName(e.target.value)}
                  onBlur={() => handleSaveRename(tab)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSaveRename(tab);
                    if (e.key === 'Escape') setEditingTabId(null);
                  }}
                  className="bg-background border border-border rounded px-1 text-xs text-foreground font-sans w-full focus:outline-none"
                />
              ) : (
                <span className="truncate flex-1 font-sans text-xs font-medium" title={tab.name}>
                  {tab.name || `Tab ${idx + 1}`}
                </span>
              )}

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  deleteTab(tab.id);
                }}
                className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-opacity cursor-pointer"
                title="Close Tab (×)"
              >
                <MingCuteIcon name="close_line" size={12} />
              </button>
            </div>
          );
        })}

        {/* Plus (+) Add New Tab Button */}
        <button
          onClick={() => createNewRequest()}
          className="p-1.5 mb-0.5 rounded hover:bg-neutral-subtle text-muted-foreground hover:text-primary transition-colors cursor-pointer"
          title="Create New Repeater Tab (+)"
        >
          <MingCuteIcon name="plus_line" size={16} />
        </button>
      </div>

      {/* Main Area: Split Request Editor (Left) & Response Inspector (Right) */}
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

            <ExecutionHistoryDrawer requestId={activeTab.id} widthPx={historyDrawerWidthPx} />
          </div>
        ) : (
          <div className="h-full flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground italic text-xs p-6">
            <MingCuteIcon name="repeat_line" size={40} className="mb-2 opacity-30" />
            No repeater tab open. Click "+" in the top bar to create a new tab.
          </div>
        )}
      </div>
    </div>
  );
};
