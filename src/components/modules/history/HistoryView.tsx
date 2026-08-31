import React, { useState, useRef } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { TrafficFilterBar } from './TrafficFilterBar';
import { RequestResponseInspector } from './RequestResponseInspector';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { ContextMenu, ContextMenuItem } from '../../common/ContextMenu';
import type { TrafficItem } from '../../../types';
import { formatReqAndRes } from '../../../utils/reqResFormatter';

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
    onlyRewritten,
    onlyFailed,
    addInterceptRule,
    initStore,
  } = useProxyStore();
  const { sendToRepeater } = useRepeaterStore();
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

      // Flag filters
      if (onlyIntercepted && !item.isIntercepted) return false;
      if (onlyRewritten && !item.isRewritten) return false;
      if (onlyFailed && !item.isFailed) return false;

      return true;
    });
  }, [traffic, searchQuery, selectedMethods, methodFilters, statusCodeRange, onlyIntercepted, onlyRewritten, onlyFailed]);

  const selectedItem = traffic.find((t) => t.id === selectedTrafficId) || null;

  const handleContextMenu = (e: React.MouseEvent, item: TrafficItem) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, item });
  };

  const handleSendToRepeater = async (item: TrafficItem) => {
    await sendToRepeater(item);
    setActiveModule('repeater');
  };

  const handleAddToCollection = async (item: TrafficItem) => {
    await sendToRepeater(item);
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

  const handleCopyReqAndRes = (item: TrafficItem) => {
    const formatted = formatReqAndRes(item);
    navigator.clipboard.writeText(formatted);
  };

  const getContextMenuItems = (item: TrafficItem): ContextMenuItem[] => [
    { label: 'Send to Repeater', icon: 'send_plane_line', action: () => handleSendToRepeater(item) },
    { label: 'Add to Collection', icon: 'folder_line', action: () => handleAddToCollection(item) },
    { label: 'Add to Intercept Rules', icon: 'shield_line', action: () => handleAddInterceptRule(item) },
    { label: 'Copy as cURL', icon: 'copy_line', action: () => handleCopyAsCurl(item) },
    { label: 'Copy req and res', icon: 'transfer_line', action: () => handleCopyReqAndRes(item) },
    { label: 'Delete Item', icon: 'delete_2_line', action: () => deleteTrafficItem(item.id), danger: true },
  ];

  return (
    <div className="h-full flex flex-col bg-background overflow-hidden">
      {/* Filter Bar */}
      <TrafficFilterBar />

      {/* Main Resizable Split Area: Table Top vs Inspector Bottom */}
      <div ref={mainSplitRef} className="flex-1 flex flex-col overflow-hidden">
        {/* Traffic Table (Resizable Height) */}
        <div
          className="flex flex-col border-b border-border bg-surface overflow-hidden min-h-[100px]"
          style={{ height: `${topHeightPercent}%` }}
        >
          {/* Table Container */}
          <div className="flex-1 overflow-auto">
            <table className="w-full text-left text-xs border-collapse table-fixed font-mono">
              <thead className="bg-header sticky top-0 border-b border-border text-[11px] font-medium text-muted-foreground select-none z-10 shadow-sm">
                <tr>
                  <th className="py-2 px-2 w-10 text-center text-muted-foreground">#</th>
                  <th className="py-2 px-2 w-24">Method</th>
                  <th className="py-2 px-2 w-48">Host</th>
                  <th className="py-2 px-2">Path</th>
                  <th className="py-2 px-2 w-16 text-center">Status</th>
                  <th className="py-2 px-2 w-32">Content-Type</th>
                  <th className="py-2 px-2 w-20 text-right">Size</th>
                  <th className="py-2 px-2 w-20 text-right">Time</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/50">
                {filteredTraffic.length === 0 ? (
                  <tr>
                    <td colSpan={8} className="py-12 text-center text-muted-foreground italic text-xs">
                      No traffic items match the current filters
                    </td>
                  </tr>
                ) : (
                  filteredTraffic.map((item, index) => {
                    const isSelected = selectedTrafficId === item.id;
                    const isFailed = item.isFailed;
                    const isIntercepted = item.isIntercepted;
                    const isRewritten = item.isRewritten;

                    const rowClass = isSelected
                      ? 'bg-primary/15 text-foreground font-semibold border-l-2 border-primary'
                      : isFailed
                      ? 'bg-rose-500/5 hover:bg-rose-500/10 text-foreground border-l-2 border-rose-500'
                      : isIntercepted
                      ? 'bg-amber-500/5 hover:bg-amber-500/10 text-foreground border-l-2 border-amber-500'
                      : isRewritten
                      ? 'bg-sky-500/5 hover:bg-sky-500/10 text-foreground border-l-2 border-sky-500'
                      : 'text-foreground hover:bg-neutral-subtle border-l-2 border-transparent';

                    return (
                      <tr
                        key={item.id}
                        onClick={() => selectTrafficItem(item.id)}
                        onContextMenu={(e) => handleContextMenu(e, item)}
                        className={`cursor-pointer transition-colors ${rowClass}`}
                      >
                        <td className="py-1.5 px-2 text-center text-muted-foreground text-[10px]">{filteredTraffic.length - index}</td>
                        <td className="py-1.5 px-2">
                          <div className="flex items-center gap-1">
                            <MethodBadge method={item.method} />
                            {item.isIntercepted && (
                              <span
                                className="px-1 py-0.2 rounded text-[9px] font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30 shrink-0 select-none"
                                title="Intercepted manually"
                              >
                                INT
                              </span>
                            )}
                            {item.isRewritten && (
                              <span
                                className="px-1 py-0.2 rounded text-[9px] font-bold bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30 shrink-0 select-none"
                                title="Rewritten automatically"
                              >
                                RW
                              </span>
                            )}
                          </div>
                        </td>
                        <td className="py-1.5 px-2 text-foreground font-medium overflow-hidden">
                          <div className="truncate" title={item.host}>{item.host}</div>
                        </td>
                        <td className="py-1.5 px-2 text-muted-foreground overflow-hidden">
                          <div className="truncate" title={item.path}>{item.path}</div>
                        </td>
                        <td className="py-1.5 px-2 text-center">
                          <StatusBadge code={item.statusCode} isFailed={item.isFailed} />
                        </td>
                        <td className="py-1.5 px-2 text-muted-foreground text-[11px] overflow-hidden">
                          <div className="truncate" title={item.contentType}>{item.contentType || '-'}</div>
                        </td>
                        <td className="py-1.5 px-2 text-right text-muted-foreground text-[11px]">{item.size}</td>
                        <td className="py-1.5 px-2 text-right text-muted-foreground text-[11px]">{item.durationMs != null ? `${item.durationMs}ms` : '-'}</td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
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
