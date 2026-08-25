import React, { useState, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { TrafficFilterBar } from './TrafficFilterBar';
import { RequestResponseInspector } from './RequestResponseInspector';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { ContextMenu, ContextMenuItem } from '../../common/ContextMenu';
import type { TrafficItem } from '../../../types';

export const HistoryView: React.FC = () => {
  const {
    traffic,
    selectedTrafficId,
    selectTrafficItem,
    deleteTrafficItem,
    searchQuery,
    selectedMethods,
    methodFilters,
    statusCodeRange,
    onlyIntercepted,
    addInterceptRule,
    initStore,
  } = useProxyStore();
  const { createNewRequest } = useRepeaterStore();
  const { layoutMode, setLayoutMode, setActiveModule } = useSettingsStore();

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: TrafficItem } | null>(null);

  // Initialize store with backend history and live event listeners
  React.useEffect(() => {
    initStore();
  }, [initStore]);

  // Resizable top vs bottom split height percentage
  const [topHeightPercent, setTopHeightPercent] = useState<number>(50);
  const mainSplitRef = useRef<HTMLDivElement>(null);
  const isResizingVertical = useRef(false);

  const handleMouseDownVerticalSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingVertical.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingVertical.current || !mainSplitRef.current) return;
      const rect = mainSplitRef.current.getBoundingClientRect();
      const relativeY = moveEvent.clientY - rect.top;
      const newPercent = (relativeY / rect.height) * 100;
      setTopHeightPercent(Math.min(Math.max(newPercent, 15), 85));
    };

    const handleMouseUp = () => {
      isResizingVertical.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  // Filter traffic items cleanly
  const filteredTraffic = React.useMemo(() => {
    return traffic.filter((item) => {
      // Keyword search
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        const matchesQuery =
          item.host.toLowerCase().includes(q) ||
          item.path.toLowerCase().includes(q) ||
          item.url.toLowerCase().includes(q) ||
          item.statusCode.toString().includes(q) ||
          item.method.toLowerCase().includes(q) ||
          item.requestBody.toLowerCase().includes(q) ||
          item.responseBody.toLowerCase().includes(q);
        if (!matchesQuery) return false;
      }

      // Tri-state Method filter (include / exclude / neutral)
      const methodKey = item.method.toUpperCase();
      const methodState = methodFilters[methodKey] || 'neutral';
      if (methodState === 'exclude') return false;

      const hasAnyInclude = Object.values(methodFilters).some((st) => st === 'include');
      if (hasAnyInclude && methodState !== 'include') return false;

      // Fallback for selectedMethods array
      if (!hasAnyInclude && selectedMethods.length > 0 && !selectedMethods.includes(methodKey)) {
        return false;
      }

      // Status code range filter
      if (statusCodeRange !== 'all') {
        const code = item.statusCode;
        if (statusCodeRange === '2xx' && (code < 200 || code >= 300)) return false;
        if (statusCodeRange === '3xx' && (code < 300 || code >= 400)) return false;
        if (statusCodeRange === '4xx' && (code < 400 || code >= 500)) return false;
        if (statusCodeRange === '5xx' && code < 500) return false;
      }

      // Intercepted only filter
      if (onlyIntercepted && !item.isIntercepted) return false;

      return true;
    });
  }, [traffic, searchQuery, selectedMethods, methodFilters, statusCodeRange, onlyIntercepted]);

  const selectedItem = traffic.find((t) => t.id === selectedTrafficId) || null;

  // Virtualizer parent container
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: filteredTraffic.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 32, // compact 32px height
    overscan: 20,
  });

  const handleContextMenu = (e: React.MouseEvent, item: TrafficItem) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, item });
  };

  const handleSendToRepeater = async (item: TrafficItem) => {
    await createNewRequest();
    const store = useRepeaterStore.getState();
    const activeReqId = store.activeTabId;
    if (activeReqId) {
      const activeReq = store.requests.find((r) => r.id === activeReqId);
      if (activeReq) {
        store.updateRequest({
          ...activeReq,
          name: `${item.method} ${item.path}`,
          method: item.method,
          url: item.url,
          headers: item.requestHeaders.map((h, i) => ({ id: `h-${i}`, key: h.key, value: h.value, enabled: true })),
          body: item.requestBody,
          bodyType: item.requestBody ? 'json' : 'none',
        });
      }
    }
    setActiveModule('repeater');
  };

  const handleAddToCollection = async (item: TrafficItem) => {
    await createNewRequest();
    const store = useRepeaterStore.getState();
    const activeReqId = store.activeTabId;
    if (activeReqId) {
      const activeReq = store.requests.find((r) => r.id === activeReqId);
      if (activeReq) {
        store.updateRequest({
          ...activeReq,
          name: `${item.method} ${item.path}`,
          method: item.method,
          url: item.url,
          headers: item.requestHeaders.map((h, i) => ({ id: `h-${i}`, key: h.key, value: h.value, enabled: true })),
          body: item.requestBody,
          bodyType: item.requestBody ? 'json' : 'none',
        });
      }
    }
    setActiveModule('collections');
  };

  const handleAddInterceptRule = (item: TrafficItem) => {
    addInterceptRule({
      target: 'domain',
      pattern: item.host,
      action: 'intercept',
      enabled: true,
    });
    setActiveModule('intercept');
  };

  const handleCopyAsCurl = (item: TrafficItem) => {
    let curl = `curl -X ${item.method} "${item.url}"`;
    item.requestHeaders.forEach((h) => {
      curl += ` \\\n  -H "${h.key}: ${h.value}"`;
    });
    if (item.requestBody) {
      curl += ` \\\n  --data '${item.requestBody.replace(/'/g, "'\\''")}'`;
    }
    navigator.clipboard.writeText(curl);
  };

  const getContextMenuItems = (item: TrafficItem): ContextMenuItem[] => [
    { label: 'Send to Repeater', icon: 'send_plane_line', action: () => handleSendToRepeater(item) },
    { label: 'Add to Collection', icon: 'folder_line', action: () => handleAddToCollection(item) },
    { label: 'Add to Intercept Rules', icon: 'shield_line', action: () => handleAddInterceptRule(item) },
    { label: 'Copy as cURL', icon: 'copy_line', action: () => handleCopyAsCurl(item) },
    { label: 'Delete Item', icon: 'delete_2_line', action: () => deleteTrafficItem(item.id), danger: true },
  ];

  return (
    <div className="h-full flex flex-col bg-background overflow-hidden">
      {/* Filter Bar */}
      <TrafficFilterBar />

      {/* Main Resizable Split Area: Table Top vs Inspector Bottom */}
      <div ref={mainSplitRef} className="flex-1 flex flex-col overflow-hidden">
        {/* Virtualized Traffic Table (Resizable Height) */}
        <div
          className="flex flex-col border-b border-border bg-surface overflow-hidden min-h-[100px]"
          style={{ height: `${topHeightPercent}%` }}
        >
          {/* Table Header */}
          <div className="bg-header border-b border-border flex items-center text-[11px] font-medium text-muted-foreground select-none shrink-0 font-mono px-2 py-1">
            <span className="w-10 text-center">#</span>
            <span className="w-16">Method</span>
            <span className="w-48 truncate px-2">Host</span>
            <span className="flex-1 truncate px-2">Path</span>
            <span className="w-16 text-center">Status</span>
            <span className="w-32 truncate px-2">Content-Type</span>
            <span className="w-20 text-right px-2">Size</span>
            <span className="w-20 text-right px-2">Time (ms)</span>
          </div>

          {/* Table Body (Virtualized) */}
          <div ref={parentRef} className="flex-1 overflow-auto">
            {filteredTraffic.length === 0 ? (
              <div className="py-12 text-center text-muted-foreground italic text-xs">
                No traffic items match the current filters
              </div>
            ) : (
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  width: '100%',
                  position: 'relative',
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const item = filteredTraffic[virtualRow.index];
                  const isSelected = selectedTrafficId === item.id;

                  return (
                    <div
                      key={item.id}
                      onClick={() => selectTrafficItem(item.id)}
                      onContextMenu={(e) => handleContextMenu(e, item)}
                      className={`absolute top-0 left-0 w-full flex items-center text-xs border-b border-border/50 cursor-pointer font-mono transition-colors ${
                        isSelected
                          ? 'bg-primary/15 text-foreground font-semibold border-primary/30'
                          : 'hover:bg-neutral-subtle text-foreground'
                      }`}
                      style={{
                        height: `${virtualRow.size}px`,
                        transform: `translateY(${virtualRow.start}px)`,
                      }}
                    >
                      <span className="w-10 text-center text-muted-foreground text-[10px]">{virtualRow.index + 1}</span>
                      <span className="w-16">
                        <MethodBadge method={item.method} />
                      </span>
                      <span className="w-48 truncate px-2 text-foreground font-medium">{item.host}</span>
                      <span className="flex-1 truncate px-2 text-muted-foreground">{item.path}</span>
                      <span className="w-16 text-center">
                        <StatusBadge code={item.statusCode} />
                      </span>
                      <span className="w-32 truncate px-2 text-muted-foreground text-[11px]">{item.contentType}</span>
                      <span className="w-20 text-right px-2 text-muted-foreground text-[11px]">{item.size}</span>
                      <span className="w-20 text-right px-2 text-muted-foreground text-[11px]">{item.durationMs}</span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* INVISIBLE DRAGGABLE RESIZER HANDLE (Visible on hover/drag) */}
        <div
          onMouseDown={handleMouseDownVerticalSplit}
          className="h-1 bg-transparent hover:bg-primary/50 active:bg-primary cursor-row-resize shrink-0 transition-colors flex items-center justify-center group z-10 relative -my-0.5"
          title="Drag to adjust height of top table and bottom inspector"
        >
          <div className="w-10 h-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
        </div>

        {/* Bottom Request / Response Inspector (Resizable Height) */}
        <div
          className="flex flex-col overflow-hidden min-h-[100px]"
          style={{ height: `${100 - topHeightPercent}%` }}
        >
          <RequestResponseInspector
            item={selectedItem}
            layoutMode={layoutMode}
            onToggleLayoutMode={() => setLayoutMode(layoutMode === 'vertical' ? 'horizontal' : 'vertical')}
          />
        </div>
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getContextMenuItems(contextMenu.item)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};
