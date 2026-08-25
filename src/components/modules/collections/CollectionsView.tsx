import React from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { CollectionSidebar } from '../repeater/CollectionSidebar';
import { RequestBuilder } from '../repeater/RequestBuilder';
import { ResponsePanel } from '../repeater/ResponsePanel';
import { ExecutionHistoryDrawer } from '../repeater/ExecutionHistoryDrawer';
import { CurlImportModal } from '../repeater/CurlImportModal';
import { MethodBadge } from '../../common/MethodBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const CollectionsView: React.FC = () => {
  const {
    requests,
    openTabIds,
    activeTabId,
    setActiveTab,
    closeTab,
    createNewRequest,
    lastExecutionResponse,
  } = useRepeaterStore();

  const activeRequest = requests.find((r) => r.id === activeTabId) || null;
  const activeResponse = activeTabId ? lastExecutionResponse[activeTabId] || null : null;

  const tabContainerRef = React.useRef<HTMLDivElement>(null);

  const handleTabWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (tabContainerRef.current) {
      tabContainerRef.current.scrollLeft += e.deltaY || e.deltaX;
    }
  };

  return (
    <div className="h-full flex bg-background overflow-hidden">
      {/* Left Sidebar: Collections & Request Tree */}
      <CollectionSidebar />

      {/* Main Workspace Area */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Workspace Tab Bar */}
        <div
          ref={tabContainerRef}
          onWheel={handleTabWheel}
          className="h-9 bg-header border-b border-border flex items-center px-2 gap-1 overflow-x-auto no-scrollbar shrink-0 select-none text-xs"
        >
          {openTabIds.map((tId) => {
            const req = requests.find((r) => r.id === tId);
            if (!req) return null;
            const isActive = activeTabId === tId;

            return (
              <div
                key={tId}
                onClick={() => setActiveTab(tId)}
                className={`flex items-center gap-2 px-3 py-1 rounded-t border-t border-x cursor-pointer font-mono text-xs max-w-[180px] shrink-0 ${
                  isActive
                    ? 'bg-surface border-border text-foreground font-semibold shadow-2xs'
                    : 'border-transparent text-muted-foreground hover:bg-neutral-subtle'
                }`}
              >
                <MethodBadge method={req.method} />
                <span className="truncate font-sans text-xs">{req.name}</span>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTab(tId);
                  }}
                  className="p-0.5 rounded text-muted-foreground hover:text-rose-500 hover:bg-rose-500/10 shrink-0"
                >
                  <MingCuteIcon name="close_line" size={12} />
                </button>
              </div>
            );
          })}

          <button
            onClick={() => createNewRequest()}
            className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle shrink-0 ml-1"
            title="New Collection Request Tab"
          >
            <MingCuteIcon name="plus_line" size={14} />
          </button>
        </div>

        {/* Tab Content Split View: Request (Left) vs Response (Right) */}
        {activeRequest ? (
          <div className="flex-1 flex overflow-hidden">
            <RequestBuilder request={activeRequest} />
            <ResponsePanel response={activeResponse} />
            <ExecutionHistoryDrawer requestId={activeRequest.id} />
          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center bg-surface text-muted-foreground italic text-xs p-6">
            <MingCuteIcon name="folder_line" size={40} className="mb-2 opacity-30" />
            No collection request open. Click "+" or select a request from the sidebar collections tree.
          </div>
        )}
      </div>

      {/* cURL Import Modal */}
      <CurlImportModal />
    </div>
  );
};
