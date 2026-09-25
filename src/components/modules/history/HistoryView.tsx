import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { TrafficFilterBar } from './TrafficFilterBar';
import { RequestResponseInspector } from './RequestResponseInspector';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { ContextMenu, ContextMenuItem } from '../../common/ContextMenu';
import type { TrafficItem } from '../../../types';
import { formatReqAndRes, formatRawCurl, formatUrlBodyAndRes } from '../../../utils/reqResFormatter';
import { CopyCustomModal } from './CopyCustomModal';

const ROW_HEIGHT = 29;
const OVERSCAN = 10;

export const HistoryView: React.FC = () => {
  const {
    traffic,
    selectedTrafficId,
    selectTrafficItem,
    deleteTrafficItem,
    searchQuery,
    selectedMethods,
    methodFilters,
    flagFilters,
    statusFilters,
    statusCodeRange,
    onlyIntercepted,
    onlyRewritten,
    onlyFailed,
    addInterceptRule,
    initStore,
    loadNextPage,
    hasMore,
    isLoadingMore,
    fetchTrafficDetail,
    trafficDetails,
    deloadInactiveTraffic,
  } = useProxyStore();
  const { sendToRepeater } = useRepeaterStore();
  const { layoutMode, setLayoutMode, setActiveModule } = useSettingsStore();

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: TrafficItem } | null>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);

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

      // Tri-state Status Code filter (2xx, 3xx, 4xx, 5xx)
      const code = item.statusCode;
      const statusRange =
        code >= 200 && code < 300
          ? '2xx'
          : code >= 300 && code < 400
          ? '3xx'
          : code >= 400 && code < 500
          ? '4xx'
          : code >= 500
          ? '5xx'
          : null;

      if (statusRange && statusFilters[statusRange] === 'exclude') {
        return false;
      }

      const hasAnyIncludeStatus = Object.values(statusFilters || {}).some((st) => st === 'include');
      if (hasAnyIncludeStatus) {
        if (!statusRange || statusFilters[statusRange] !== 'include') return false;
      } else if (statusCodeRange !== 'all') {
        // Fallback for legacy statusCodeRange
        if (statusRange !== statusCodeRange) return false;
      }

      // Tri-state Flag filters (waiting, intercepted, rewritten, failed)
      const isPendingResponse = item.phase === 'request';
      const isPendingIntercept = item.phase === 'intercepted_request' || item.phase === 'intercepted_response';
      const isWaiting = isPendingResponse;
      const isIntercepted = Boolean(item.isIntercepted || isPendingIntercept);
      const isRewritten = Boolean(item.isRewritten);
      const isFailed = Boolean(item.isFailed && !isPendingResponse && !isPendingIntercept);

      const flagMap: Record<string, boolean> = {
        waiting: isWaiting,
        intercepted: isIntercepted,
        rewritten: isRewritten,
        failed: isFailed,
      };

      // If any active flag is set to exclude and the item matches it, exclude it
      for (const [flag, state] of Object.entries(flagFilters || {})) {
        if (state === 'exclude' && flagMap[flag]) {
          return false;
        }
      }

      // If any flags are set to include, the item must match at least one included flag
      const hasAnyIncludeFlag = Object.values(flagFilters || {}).some((st) => st === 'include');
      if (hasAnyIncludeFlag) {
        const matchesAnyInclude = Object.entries(flagFilters || {}).some(
          ([flag, state]) => state === 'include' && flagMap[flag]
        );
        if (!matchesAnyInclude) return false;
      } else {
        // Fallback for boolean flags
        if (onlyIntercepted && !flagMap.intercepted) return false;
        if (onlyRewritten && !flagMap.rewritten) return false;
        if (onlyFailed && !flagMap.failed) return false;
      }

      return true;
    });
  }, [traffic, searchQuery, selectedMethods, methodFilters, statusFilters, statusCodeRange, flagFilters, onlyIntercepted, onlyRewritten, onlyFailed]);

  // Virtualization state & refs
  const tableContainerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(400);

  // ResizeObserver for table viewport height
  useEffect(() => {
    const el = tableContainerRef.current;
    if (!el) return;

    const updateHeight = () => {
      if (el.clientHeight > 0) {
        setViewportHeight(el.clientHeight);
      }
    };
    updateHeight();

    const observer = new ResizeObserver(updateHeight);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Optimized smooth scroll handler with requestAnimationFrame & infinite scroll trigger
  const scrollRaf = useRef<number | null>(null);
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const target = e.currentTarget;
    const currentScrollTop = target.scrollTop;

    if (scrollRaf.current) {
      cancelAnimationFrame(scrollRaf.current);
    }

    scrollRaf.current = requestAnimationFrame(() => {
      setScrollTop(currentScrollTop);

      // Infinite scroll trigger: when user scrolls within 15 rows of the end
      const totalVirtualHeight = filteredTraffic.length * ROW_HEIGHT;
      const scrollBottom = currentScrollTop + target.clientHeight;
      if (totalVirtualHeight - scrollBottom < 15 * ROW_HEIGHT) {
        if (hasMore && !isLoadingMore) {
          loadNextPage();
        }
      }
    });
  }, [filteredTraffic.length, hasMore, isLoadingMore, loadNextPage]);

  // Inactivity Deloader: prune in-memory buffer after 60s of idle if user is at top of table or window is hidden
  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout>;

    const resetInactivityTimer = () => {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        const currentScroll = tableContainerRef.current?.scrollTop || 0;
        if (currentScroll < 100 || document.visibilityState === 'hidden') {
          deloadInactiveTraffic();
        }
      }, 60000);
    };

    const events = ['mousedown', 'keydown', 'scroll', 'touchstart'];
    events.forEach((ev) => window.addEventListener(ev, resetInactivityTimer, { passive: true }));
    document.addEventListener('visibilitychange', resetInactivityTimer);
    resetInactivityTimer();

    return () => {
      clearTimeout(timeoutId);
      events.forEach((ev) => window.removeEventListener(ev, resetInactivityTimer));
      document.removeEventListener('visibilitychange', resetInactivityTimer);
    };
  }, [deloadInactiveTraffic]);

  // Compute virtual slice
  const totalRows = filteredTraffic.length;
  const startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const endIndex = Math.min(totalRows, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN);

  const topSpacer = startIndex * ROW_HEIGHT;
  const bottomSpacer = Math.max(0, (totalRows - endIndex) * ROW_HEIGHT);
  const visibleRows = useMemo(() => {
    return filteredTraffic.slice(startIndex, endIndex);
  }, [filteredTraffic, startIndex, endIndex]);

  // Build full item with lazy detail for Inspector
  const selectedBaseItem = useMemo(() => {
    return traffic.find((i) => i.id === selectedTrafficId) || null;
  }, [traffic, selectedTrafficId]);

  const selectedDetail = selectedTrafficId ? trafficDetails[selectedTrafficId] : null;

  const selectedItemWithDetail = useMemo(() => {
    if (!selectedBaseItem) return null;
    if (!selectedDetail) return selectedBaseItem;
    return {
      ...selectedBaseItem,
      requestHeaders: selectedDetail.requestHeaders.map(([k, v]) => ({ key: k, value: v })),
      responseHeaders: selectedDetail.responseHeaders.map(([k, v]) => ({ key: k, value: v })),
      requestBody: selectedDetail.requestBody,
      responseBody: selectedDetail.responseBody,
    };
  }, [selectedBaseItem, selectedDetail]);

  const handleContextMenu = (e: React.MouseEvent, item: TrafficItem) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, item });
  };

  // Helper to ensure item detail is loaded before copying or sending to repeater
  const ensureFullItem = async (item: TrafficItem): Promise<TrafficItem> => {
    if (item.requestBody || item.responseBody || item.requestHeaders.length > 0) {
      return item;
    }
    const detail = await fetchTrafficDetail(item.id);
    if (!detail) return item;
    return {
      ...item,
      requestHeaders: detail.requestHeaders.map(([k, v]) => ({ key: k, value: v })),
      responseHeaders: detail.responseHeaders.map(([k, v]) => ({ key: k, value: v })),
      requestBody: detail.requestBody,
      responseBody: detail.responseBody,
    };
  };

  const handleSendToRepeater = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    await sendToRepeater(full);
    setActiveModule('repeater');
  };

  const handleAddToCollection = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    await sendToRepeater(full);
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

  const handleCopyAsCurl = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    const curl = formatRawCurl(full);
    navigator.clipboard.writeText(curl);
  };

  const handleCopyReqAndRes = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    const formatted = formatReqAndRes(full);
    navigator.clipboard.writeText(formatted);
  };

  const handleCopyUrlBodyAndRes = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    const formatted = formatUrlBodyAndRes(full);
    navigator.clipboard.writeText(formatted);
  };

  const handleCustomCopy = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    setCustomCopyItem(full);
  };

  const getContextMenuItems = (item: TrafficItem): ContextMenuItem[] => [
    { label: 'Send to Repeater', icon: 'send_plane_line', action: () => handleSendToRepeater(item) },
    { label: 'Add to Collection', icon: 'folder_line', action: () => handleAddToCollection(item) },
    { label: 'Add to Intercept Rules', icon: 'shield_line', action: () => handleAddInterceptRule(item) },
    {
      label: 'Copy',
      icon: 'copy_line',
      children: [
        { label: 'Copy Curl', icon: 'terminal_line', action: () => handleCopyAsCurl(item) },
        { label: 'Copy Curl and Res', icon: 'transfer_line', action: () => handleCopyReqAndRes(item) },
        {
          label: 'Copy URL, Body, and Response',
          icon: 'file_code_line',
          action: () => handleCopyUrlBodyAndRes(item),
        },
        { label: 'Copy custom', icon: 'settings_3_line', action: () => handleCustomCopy(item) },
      ],
    },
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
          {/* Virtualized Table Container */}
          <div ref={tableContainerRef} onScroll={handleScroll} className="flex-1 overflow-auto">
            <table className="w-full text-left text-xs border-collapse table-fixed font-mono">
              <thead className="bg-header sticky top-0 border-b border-border text-[11px] font-medium text-muted-foreground select-none z-10 shadow-sm">
                <tr>
                  <th className="py-2 px-2 w-12 text-center text-muted-foreground">#</th>
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
                {totalRows === 0 ? (
                  <tr>
                    <td colSpan={8} className="py-12 text-center text-muted-foreground italic text-xs">
                      {isLoadingMore ? 'Loading traffic logs...' : 'No traffic items match the current filters'}
                    </td>
                  </tr>
                ) : (
                  <>
                    {/* Top Spacer Row */}
                    {topSpacer > 0 && (
                      <tr style={{ height: `${topSpacer}px`, pointerEvents: 'none' }}>
                        <td colSpan={8} className="p-0 border-0" />
                      </tr>
                    )}

                    {/* Rendered Visible Rows */}
                    {visibleRows.map((item) => {
                      const isSelected = selectedTrafficId === item.id;
                      const isPendingResponse = item.phase === 'request';
                      const isPendingIntercept = item.phase === 'intercepted_request' || item.phase === 'intercepted_response';
                      const isFailed = item.isFailed && !isPendingResponse && !isPendingIntercept;
                      const isIntercepted = item.isIntercepted && !isPendingIntercept;
                      const isRewritten = item.isRewritten;

                      const rowClass = isSelected
                        ? 'bg-primary/15 text-foreground font-semibold ring-1 ring-inset ring-primary'
                        : isPendingIntercept
                        ? 'animate-pulse bg-amber-500/15 hover:bg-amber-500/25 text-foreground'
                        : isPendingResponse
                        ? 'animate-pulse bg-rose-500/10 hover:bg-rose-500/20 text-foreground'
                        : isFailed
                        ? 'bg-rose-500/5 hover:bg-rose-500/10 text-foreground'
                        : isIntercepted
                        ? 'bg-amber-500/5 hover:bg-amber-500/10 text-foreground'
                        : isRewritten
                        ? 'bg-sky-500/5 hover:bg-sky-500/10 text-foreground'
                        : 'text-foreground hover:bg-neutral-subtle';

                      return (
                        <tr
                          key={item.id}
                          style={{ height: `${ROW_HEIGHT}px` }}
                          onClick={() => selectTrafficItem(item.id)}
                          onContextMenu={(e) => handleContextMenu(e, item)}
                          className={`cursor-pointer transition-colors ${rowClass}`}
                        >
                          <td className="py-1 px-2 text-center text-muted-foreground text-[10px]">{item.id}</td>
                          <td className="py-1 px-2">
                            <div className="flex items-center gap-1">
                              <MethodBadge method={item.method} />
                              {(isIntercepted || isPendingIntercept) && (
                                <span
                                  className={`px-1 py-0.2 rounded text-[9px] font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30 shrink-0 select-none ${
                                    isPendingIntercept ? 'animate-pulse' : ''
                                  }`}
                                  title={isPendingIntercept ? 'Paused in Interceptor' : 'Intercepted manually'}
                                >
                                  INT
                                </span>
                              )}
                              {isRewritten && (
                                <span
                                  className="px-1 py-0.2 rounded text-[9px] font-bold bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30 shrink-0 select-none"
                                  title="Rewritten automatically"
                                >
                                  RW
                                </span>
                              )}
                            </div>
                          </td>
                          <td className="py-1 px-2 text-foreground font-medium overflow-hidden">
                            <div className="truncate" title={item.host}>{item.host}</div>
                          </td>
                          <td className="py-1 px-2 text-muted-foreground overflow-hidden">
                            <div className="truncate" title={item.path}>{item.path}</div>
                          </td>
                          <td className="py-1 px-2 text-center">
                            <StatusBadge
                              code={item.statusCode}
                              isFailed={isFailed}
                              isPending={isPendingResponse}
                              isInterceptedPending={isPendingIntercept}
                            />
                          </td>
                          <td className="py-1 px-2 text-muted-foreground text-[11px] overflow-hidden">
                            <div className="truncate" title={item.contentType}>{item.contentType || '-'}</div>
                          </td>
                          <td className="py-1 px-2 text-right text-muted-foreground text-[11px]">
                            {isPendingResponse || isPendingIntercept ? '-' : item.size}
                          </td>
                          <td className="py-1 px-2 text-right text-muted-foreground text-[11px]">
                            {isPendingResponse ? (
                              <span className="text-rose-500 dark:text-rose-400 font-mono animate-pulse">...</span>
                            ) : isPendingIntercept ? (
                              <span className="text-amber-500 dark:text-amber-400 font-mono animate-pulse">...</span>
                            ) : item.durationMs != null ? (
                              `${item.durationMs}ms`
                            ) : (
                              '-'
                            )}
                          </td>
                        </tr>
                      );
                    })}

                    {/* Bottom Spacer Row */}
                    {bottomSpacer > 0 && (
                      <tr style={{ height: `${bottomSpacer}px`, pointerEvents: 'none' }}>
                        <td colSpan={8} className="p-0 border-0" />
                      </tr>
                    )}

                    {/* Infinite Loading Indicator */}
                    {isLoadingMore && (
                      <tr>
                        <td colSpan={8} className="py-2 text-center text-muted-foreground text-[11px] italic bg-surface/50 animate-pulse">
                          Loading more records from SQLite...
                        </td>
                      </tr>
                    )}
                  </>
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
            item={selectedItemWithDetail}
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

      {/* Copy Custom Modal */}
      <CopyCustomModal
        isOpen={!!customCopyItem}
        item={customCopyItem}
        onClose={() => setCustomCopyItem(null)}
      />
    </div>
  );
};
