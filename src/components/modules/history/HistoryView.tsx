import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { TrafficFilterBar } from './TrafficFilterBar';
import { RequestResponseInspector } from './RequestResponseInspector';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { ContextMenu, ContextMenuItem } from '../../common/ContextMenu';
import type { TrafficItem } from '../../../types';
import { formatReqAndRes, formatRawCurl, formatUrlBodyAndRes } from '../../../utils/reqResFormatter';
import { CopyCustomModal } from './CopyCustomModal';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { HISTORY_COLUMN_DEFAULTS } from '../../../stores/uiPrefs/registry';
import { moveColumn } from '../../../utils/columnLayout';
import { clampPercent, startDragResize } from '../../../utils/dragResize';
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core';
import { SortableContext, horizontalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

/** Inspector visibility for this app session; HistoryView unmounts when switching modules */
let inspectorOpenThisSession = false;

const ROW_HEIGHT = 29;
const OVERSCAN = 10;
const MIN_COLUMN_WIDTH = 40;

interface RowState {
  isPendingResponse: boolean;
  isPendingIntercept: boolean;
  isFailed: boolean;
  isIntercepted: boolean;
  isRewritten: boolean;
}

interface HistoryColumn {
  id: string;
  label: string;
  /** Label in the show/hide menu when the header label is too terse */
  menuLabel?: string;
  align?: 'left' | 'center' | 'right';
  cellClass?: string;
  render: (item: TrafficItem, row: RowState) => React.ReactNode;
}

const ALIGN_CLASS = { left: 'text-left', center: 'text-center', right: 'text-right' } as const;

const pad = (n: number, len = 2) => String(n).padStart(len, '0');

/** Local wall-clock time with milliseconds, e.g. 14:03:27.512 */
const formatClock = (ms: number) => {
  const d = new Date(ms);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`;
};

const ClockCell: React.FC<{ ms: number | null | undefined }> = ({ ms }) =>
  ms != null ? <span title={new Date(ms).toLocaleString()}>{formatClock(ms)}</span> : <>-</>;

const PendingDots: React.FC<{ row: RowState }> = ({ row }) => (
  <span
    className={`font-mono animate-pulse ${
      row.isPendingIntercept ? 'text-amber-500 dark:text-amber-400' : 'text-rose-500 dark:text-rose-400'
    }`}
  >
    ...
  </span>
);

const HISTORY_COLUMNS: HistoryColumn[] = [
  {
    id: 'id',
    label: '#',
    menuLabel: '# (ID)',
    align: 'center',
    cellClass: 'text-muted-foreground text-3xs',
    render: (item) => item.id,
  },
  {
    id: 'method',
    label: 'Method',
    render: (item, row) => (
      <div className="flex items-center gap-1">
        <MethodBadge method={item.method} />
        {(row.isIntercepted || row.isPendingIntercept) && (
          <span
            className={`px-1 py-0.2 rounded text-3xs font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30 shrink-0 select-none ${
              row.isPendingIntercept ? 'animate-pulse' : ''
            }`}
            title={row.isPendingIntercept ? 'Paused in Interceptor' : 'Intercepted manually'}
          >
            INT
          </span>
        )}
        {row.isRewritten && (
          <span
            className="px-1 py-0.2 rounded text-3xs font-bold bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30 shrink-0 select-none"
            title="Rewritten automatically"
          >
            RW
          </span>
        )}
      </div>
    ),
  },
  {
    id: 'source',
    label: 'Source',
    cellClass: 'text-muted-foreground text-3xs',
    render: (item) =>
      item.listenerLabel && item.listenerLabel !== 'Default' ? (
        <span
          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-3xs font-semibold bg-primary/10 text-primary border border-primary/20 truncate max-w-full select-none"
          title={`Source: ${item.listenerLabel}`}
        >
          <span className="w-1 h-1 rounded-full bg-primary shrink-0" />
          <span className="truncate">{item.listenerLabel}</span>
        </span>
      ) : (
        <span className="text-muted-foreground/50 text-3xs font-mono" title="Default proxy listener">
          {item.listenerLabel || 'Default'}
        </span>
      ),
  },
  {
    id: 'host',
    label: 'Host',
    cellClass: 'text-foreground font-medium',
    render: (item) => <div className="truncate" title={item.host}>{item.host}</div>,
  },
  {
    id: 'path',
    label: 'Path',
    cellClass: 'text-muted-foreground',
    render: (item) => <div className="truncate" title={item.path}>{item.path}</div>,
  },
  {
    id: 'status',
    label: 'Status',
    align: 'center',
    render: (item, row) => (
      <StatusBadge
        code={item.statusCode}
        isFailed={row.isFailed}
        isPending={row.isPendingResponse}
        isInterceptedPending={row.isPendingIntercept}
      />
    ),
  },
  {
    id: 'contentType',
    label: 'Content-Type',
    cellClass: 'text-muted-foreground text-2xs',
    render: (item) => <div className="truncate" title={item.contentType}>{item.contentType || '-'}</div>,
  },
  {
    id: 'size',
    label: 'Size',
    align: 'right',
    cellClass: 'text-muted-foreground text-2xs',
    render: (item, row) => (row.isPendingResponse || row.isPendingIntercept ? '-' : item.size),
  },
  {
    id: 'duration',
    label: 'Duration',
    align: 'right',
    cellClass: 'text-muted-foreground text-2xs',
    render: (item, row) =>
      row.isPendingResponse || row.isPendingIntercept ? (
        <PendingDots row={row} />
      ) : item.durationMs != null ? (
        `${item.durationMs}ms`
      ) : (
        '-'
      ),
  },
  {
    id: 'requestAt',
    label: 'Request Time',
    align: 'right',
    cellClass: 'text-muted-foreground text-2xs',
    render: (item) => <ClockCell ms={item.requestAt} />,
  },
  {
    id: 'responseAt',
    label: 'Response Time',
    align: 'right',
    cellClass: 'text-muted-foreground text-2xs',
    render: (item, row) =>
      row.isPendingResponse || row.isPendingIntercept ? <PendingDots row={row} /> : <ClockCell ms={item.responseAt} />,
  },
];

const COLUMNS_BY_ID: Record<string, HistoryColumn> = Object.fromEntries(HISTORY_COLUMNS.map((c) => [c.id, c]));
const HISTORY_COLUMN_DEFAULT_WIDTHS: Record<string, number> = Object.fromEntries(
  HISTORY_COLUMN_DEFAULTS.map((c) => [c.id, c.width])
);

const SortableHeaderCell: React.FC<{
  column: HistoryColumn;
  onResizeStart: (e: React.PointerEvent) => void;
  onResetWidth: () => void;
}> = ({ column, onResizeStart, onResetWidth }) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: column.id });

  return (
    <th
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      style={{
        // Horizontal only; the default sortable transform also scales, which distorts table cells
        transform: transform ? CSS.Translate.toString({ ...transform, y: 0 }) : undefined,
        transition,
      }}
      title="Drag to reorder, right-click to show or hide columns"
      className={`relative py-2 px-2 truncate cursor-grab active:cursor-grabbing outline-none ${ALIGN_CLASS[column.align ?? 'left']} ${
        isDragging ? 'z-20 bg-neutral-subtle text-foreground shadow-md' : ''
      }`}
    >
      {column.label}
      <div
        onPointerDown={(e) => {
          e.stopPropagation();
          onResizeStart(e);
        }}
        onDoubleClick={onResetWidth}
        className="absolute top-0 right-0 h-full w-1.5 cursor-col-resize hover:bg-primary/50 active:bg-primary"
        title="Drag to resize, double-click to reset width"
      />
    </th>
  );
};

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
    listenerFilter,
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
  const loadFuzzerFromTraffic = useFuzzerStore((s) => s.loadFromTraffic);
  const { setActiveModule } = useSettingsStore();

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: TrafficItem } | null>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);

  // Initialize store with backend history and live event listeners
  React.useEffect(() => {
    initStore();
  }, [initStore]);

  // Inspector starts hidden at launch and stays as the user left it while switching modules;
  // selecting a row shows it again
  const [inspectorOpen, setInspectorOpenState] = useState(inspectorOpenThisSession);
  const setInspectorOpen = (open: boolean) => {
    inspectorOpenThisSession = open;
    setInspectorOpenState(open);
  };
  const handleSelectRow = (id: string) => {
    selectTrafficItem(id);
    setInspectorOpen(true);
  };

  // Resizable top vs bottom split height percentage
  const [topHeightPercent, setTopHeightPercent, resetTopHeightPercent] = useUiPref('history.tableHeightPercent');
  const mainSplitRef = useRef<HTMLDivElement>(null);
  const tablePaneRef = useRef<HTMLDivElement>(null);
  const inspectorPaneRef = useRef<HTMLDivElement>(null);

  const handleVerticalSplitPointerDown = (e: React.PointerEvent) => {
    let percent = topHeightPercent;
    startDragResize(e, {
      cursor: 'row-resize',
      onMove: (ev) => {
        if (!mainSplitRef.current || !tablePaneRef.current || !inspectorPaneRef.current) return;
        const rect = mainSplitRef.current.getBoundingClientRect();
        percent = clampPercent(((ev.clientY - rect.top) / rect.height) * 100);
        tablePaneRef.current.style.height = `${percent}%`;
        inspectorPaneRef.current.style.height = `${100 - percent}%`;
      },
      onEnd: () => setTopHeightPercent(percent),
    });
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
          (item.listenerLabel && item.listenerLabel.toLowerCase().includes(q)) ||
          item.requestBody.toLowerCase().includes(q) ||
          item.responseBody.toLowerCase().includes(q);
        if (!matchesQuery) return false;
      }

      // Listener Source filter
      if (listenerFilter) {
        const itemLabel = item.listenerLabel || 'Default';
        if (itemLabel.toLowerCase() !== listenerFilter.toLowerCase()) {
          return false;
        }
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
  }, [traffic, searchQuery, selectedMethods, methodFilters, statusFilters, statusCodeRange, flagFilters, onlyIntercepted, onlyRewritten, onlyFailed, listenerFilter]);

  // Column layout (order, widths, visibility), persisted as a UI preference
  const [columnLayout, setColumnLayout, resetColumnLayout] = useUiPref('history.columns');
  const visibleColumns = useMemo(
    () => columnLayout.order.filter((id) => !columnLayout.hidden.includes(id)).map((id) => COLUMNS_BY_ID[id]),
    [columnLayout]
  );
  const tableWidth = visibleColumns.reduce((sum, col) => sum + columnLayout.widths[col.id], 0);

  const layoutRef = useRef(columnLayout);
  layoutRef.current = columnLayout;

  const tableRef = useRef<HTMLTableElement>(null);
  const colRefs = useRef<Record<string, HTMLTableColElement | null>>({});

  const startColumnResize = (id: string) => (e: React.PointerEvent) => {
    e.stopPropagation();
    const startX = e.clientX;
    const startWidth = layoutRef.current.widths[id];
    const otherColumnsWidth = tableWidth - startWidth;
    let width = startWidth;

    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        width = Math.max(MIN_COLUMN_WIDTH, Math.round(startWidth + ev.clientX - startX));
        const col = colRefs.current[id];
        if (col) col.style.width = `${width}px`;
        if (tableRef.current) {
          tableRef.current.style.width = `${otherColumnsWidth + width}px`;
          tableRef.current.style.minWidth = `${otherColumnsWidth + width}px`;
        }
      },
      onEnd: () => {
        const current = layoutRef.current;
        if (width !== current.widths[id]) setColumnLayout({ ...current, widths: { ...current.widths, [id]: width } });
      },
    });
  };

  // Small activation distance so clicks and the resize handle don't start a drag
  const dndSensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  const handleColumnDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const order = columnLayout.order;
    const after = order.indexOf(String(active.id)) < order.indexOf(String(over.id));
    setColumnLayout({ ...columnLayout, order: moveColumn(order, String(active.id), String(over.id), after) });
  };

  const toggleColumn = (id: string) => {
    const hidden = columnLayout.hidden.includes(id)
      ? columnLayout.hidden.filter((h) => h !== id)
      : [...columnLayout.hidden, id];
    setColumnLayout({ ...columnLayout, hidden });
  };

  const [columnMenu, setColumnMenu] = useState<{ x: number; y: number } | null>(null);
  const columnMenuRef = useRef<HTMLDivElement>(null);

  const openColumnMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setColumnMenu({ x: e.clientX, y: e.clientY });
  };

  useEffect(() => {
    if (!columnMenu) return;
    const handleMouseDown = (e: MouseEvent) => {
      if (columnMenuRef.current && !columnMenuRef.current.contains(e.target as Node)) setColumnMenu(null);
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setColumnMenu(null);
    };
    document.addEventListener('mousedown', handleMouseDown);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleMouseDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [columnMenu]);

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
  const showInspector = inspectorOpen && selectedBaseItem !== null;

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

  const handleSendToFuzzer = async (item: TrafficItem) => {
    const full = await ensureFullItem(item);
    loadFuzzerFromTraffic(full);
    setActiveModule('fuzzer');
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
    { label: 'Send to Fuzzer', icon: 'fast_forward_line', action: () => handleSendToFuzzer(item) },
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
          ref={tablePaneRef}
          className="flex flex-col border-b border-border bg-surface overflow-hidden min-h-[100px]"
          style={{ height: showInspector ? `${topHeightPercent}%` : '100%' }}
        >
          {/* Virtualized Table Container */}
          <div ref={tableContainerRef} onScroll={handleScroll} className="flex-1 overflow-auto">
            <table
              ref={tableRef}
              className="text-left text-xs border-collapse table-fixed font-mono"
              style={{ width: tableWidth, minWidth: tableWidth }}
            >
              <colgroup>
                {visibleColumns.map((col) => (
                  <col
                    key={col.id}
                    ref={(el) => {
                      colRefs.current[col.id] = el;
                    }}
                    style={{ width: columnLayout.widths[col.id] }}
                  />
                ))}
              </colgroup>
              <thead className="bg-header sticky top-0 border-b border-border text-2xs font-medium text-muted-foreground select-none z-10 shadow-sm">
                <DndContext sensors={dndSensors} collisionDetection={closestCenter} onDragEnd={handleColumnDragEnd}>
                  <SortableContext items={visibleColumns.map((c) => c.id)} strategy={horizontalListSortingStrategy}>
                    <tr onContextMenu={openColumnMenu}>
                      {visibleColumns.map((col) => (
                        <SortableHeaderCell
                          key={col.id}
                          column={col}
                          onResizeStart={startColumnResize(col.id)}
                          onResetWidth={() => setColumnLayout({
                            ...columnLayout,
                            widths: { ...columnLayout.widths, [col.id]: HISTORY_COLUMN_DEFAULT_WIDTHS[col.id] },
                          })}
                        />
                      ))}
                    </tr>
                  </SortableContext>
                </DndContext>
              </thead>
              <tbody className="divide-y divide-border/50">
                {totalRows === 0 ? (
                  <tr>
                    <td colSpan={visibleColumns.length} className="py-12 text-center text-muted-foreground italic text-xs">
                      {isLoadingMore ? 'Loading traffic logs...' : 'No traffic items match the current filters'}
                    </td>
                  </tr>
                ) : (
                  <>
                    {/* Top Spacer Row */}
                    {topSpacer > 0 && (
                      <tr style={{ height: `${topSpacer}px`, pointerEvents: 'none' }}>
                        <td colSpan={visibleColumns.length} className="p-0 border-0" />
                      </tr>
                    )}

                    {/* Rendered Visible Rows */}
                    {visibleRows.map((item) => {
                      const isSelected = selectedTrafficId === item.id;
                      const isPendingResponse = item.phase === 'request';
                      const isPendingIntercept = item.phase === 'intercepted_request' || item.phase === 'intercepted_response';
                      const isFailed = Boolean(item.isFailed && !isPendingResponse && !isPendingIntercept);
                      const isIntercepted = Boolean(item.isIntercepted && !isPendingIntercept);
                      const isRewritten = Boolean(item.isRewritten);
                      const row: RowState = { isPendingResponse, isPendingIntercept, isFailed, isIntercepted, isRewritten };

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
                          onClick={() => handleSelectRow(item.id)}
                          onContextMenu={(e) => handleContextMenu(e, item)}
                          className={`cursor-pointer transition-colors ${rowClass}`}
                        >
                          {visibleColumns.map((col) => (
                            <td key={col.id} className={`py-1 px-2 overflow-hidden ${ALIGN_CLASS[col.align ?? 'left']} ${col.cellClass ?? ''}`}>
                              {col.render(item, row)}
                            </td>
                          ))}
                        </tr>
                      );
                    })}

                    {/* Bottom Spacer Row */}
                    {bottomSpacer > 0 && (
                      <tr style={{ height: `${bottomSpacer}px`, pointerEvents: 'none' }}>
                        <td colSpan={visibleColumns.length} className="p-0 border-0" />
                      </tr>
                    )}

                    {/* Infinite Loading Indicator */}
                    {isLoadingMore && (
                      <tr>
                        <td colSpan={visibleColumns.length} className="py-2 text-center text-muted-foreground text-2xs italic bg-surface/50 animate-pulse">
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

        {showInspector && (
          <>
          {/* INVISIBLE DRAGGABLE RESIZER HANDLE (Visible on hover/drag) */}
          <div
            onPointerDown={handleVerticalSplitPointerDown}
            onDoubleClick={resetTopHeightPercent}
            className="h-1 bg-transparent hover:bg-primary/50 active:bg-primary cursor-row-resize shrink-0 transition-colors flex items-center justify-center group z-10 relative -my-0.5"
            title="Drag to resize, double-click to reset"
          >
            <div className="w-10 h-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
          </div>

          {/* Bottom Request / Response Inspector (Resizable Height) */}
          <div
            ref={inspectorPaneRef}
            className="flex flex-col overflow-hidden min-h-[100px]"
            style={{ height: `${100 - topHeightPercent}%` }}
          >
            <RequestResponseInspector
              item={selectedItemWithDetail}
              onHide={() => setInspectorOpen(false)}
            />
          </div>
          </>
        )}
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

      {/* Column visibility menu (header right-click) */}
      {columnMenu && (
        <div
          ref={columnMenuRef}
          style={{ left: columnMenu.x, top: columnMenu.y }}
          className="fixed z-50 min-w-[180px] py-1 bg-surface border border-border rounded-lg shadow-lg text-xs select-none"
        >
          <div className="px-3 py-1 text-3xs uppercase tracking-wider font-semibold text-muted-foreground">Columns</div>
          {columnLayout.order.map((id) => {
            const col = COLUMNS_BY_ID[id];
            const isVisible = !columnLayout.hidden.includes(id);
            const isLastVisible = isVisible && visibleColumns.length === 1;
            return (
              <label
                key={id}
                className={`flex items-center gap-2 px-3 py-1 hover:bg-neutral-subtle ${isLastVisible ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
              >
                <input
                  type="checkbox"
                  checked={isVisible}
                  disabled={isLastVisible}
                  onChange={() => toggleColumn(id)}
                  className="rounded accent-primary cursor-pointer w-3.5 h-3.5"
                />
                <span className="text-foreground">{col.menuLabel ?? col.label}</span>
              </label>
            );
          })}
          <div className="my-1 border-t border-border" />
          <button
            onClick={() => {
              resetColumnLayout();
              setColumnMenu(null);
            }}
            className="w-full text-left px-3 py-1 text-foreground hover:bg-neutral-subtle cursor-pointer"
          >
            Reset columns
          </button>
        </div>
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
