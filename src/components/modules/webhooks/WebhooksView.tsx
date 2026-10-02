import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { clampPercent, startDragResize } from '../../../utils/dragResize';
import type { WebhookEndpoint } from '../../../types';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button, Dialog } from '../../common/ui';
import { WebhookSidebar, ALL_ENDPOINTS, UNMATCHED_ENDPOINT } from './components/WebhookSidebar';
import { WebhookDeliveriesTable } from './components/WebhookDeliveriesTable';
import { WebhookDeliveryInspector } from './components/WebhookDeliveryInspector';
import { WebhookHmacSandbox } from './components/WebhookHmacSandbox';
import { NewEndpointModal } from './components/NewEndpointModal';

const curlFor = (url: string) =>
  `curl -X POST "${url}" \\\n  -H "Content-Type: application/json" \\\n  -d '{"event":"test_webhook","timestamp":${Date.now()}}'`;

export const WebhooksView: React.FC = () => {
  const { listenerConfig, endpoints, deliveries, clearDeliveries, deleteEndpoint, selectedDeliveryId, selectDelivery, initialize } =
    useWebhookStore();

  const [filter, setFilter] = useState<string>(ALL_ENDPOINTS);
  const [search, setSearch] = useState('');
  const [isAddOpen, setIsAddOpen] = useState(false);
  const [isHmacOpen, setIsHmacOpen] = useState(false);
  const [deletingEndpoint, setDeletingEndpoint] = useState<WebhookEndpoint | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const [showSecret, setShowSecret] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const [listPercent, setListPercent, resetListPercent] = useUiPref('webhooks.listSplitPercent');
  const splitRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    initialize();
  }, [initialize]);

  useEffect(() => {
    if (filter !== ALL_ENDPOINTS && filter !== UNMATCHED_ENDPOINT && !endpoints.some((e) => e.id === filter)) {
      setFilter(ALL_ENDPOINTS);
    }
    setShowSecret(false);
  }, [filter, endpoints]);

  const copy = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setToast(`${label} copied`);
    setTimeout(() => setToast(null), 1500);
  };

  const baseUrl = `http://127.0.0.1:${listenerConfig.port}`;
  const endpoint = endpoints.find((e) => e.id === filter) ?? null;

  const shown = useMemo(() => {
    const q = search.trim().toLowerCase();
    return deliveries.filter((d) => {
      if (filter !== ALL_ENDPOINTS && d.endpointId !== filter) return false;
      return !q || d.endpointPath.toLowerCase().includes(q) || d.payload.toLowerCase().includes(q);
    });
  }, [deliveries, filter, search]);

  const selected = shown.find((d) => d.id === selectedDeliveryId) ?? shown[0] ?? null;

  const handleSplitPointerDown = (e: React.PointerEvent) => {
    let percent = listPercent;
    startDragResize(e, {
      cursor: 'col-resize',
      onMove: (ev) => {
        if (!splitRef.current || !listRef.current) return;
        const rect = splitRef.current.getBoundingClientRect();
        percent = clampPercent(((ev.clientX - rect.left) / rect.width) * 100);
        listRef.current.style.width = `${percent}%`;
      },
      onEnd: () => setListPercent(percent),
    });
  };

  const handleClear = async () => {
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    setConfirmClear(false);
    await clearDeliveries().catch(() => {});
  };

  const handleDeleteEndpoint = async () => {
    if (!deletingEndpoint) return;
    await deleteEndpoint(deletingEndpoint.id);
    setDeletingEndpoint(null);
  };

  const iconButtonClass = 'p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer';

  return (
    <div className="h-full flex bg-background overflow-hidden text-xs">
      <div className="w-64 shrink-0">
        <WebhookSidebar
          selectedFilter={filter}
          onSelectFilter={setFilter}
          onAddEndpoint={() => setIsAddOpen(true)}
          onOpenHmacTool={() => setIsHmacOpen(true)}
          onDeleteEndpoint={setDeletingEndpoint}
          onCopy={copy}
        />
      </div>

      <div className="flex-1 min-w-0 flex flex-col">
        <div className="h-12 px-3 bg-header border-b border-border flex items-center gap-3 shrink-0">
          {endpoint ? (
            <div className="flex-1 min-w-0 flex items-center gap-3">
              <div className="min-w-0">
                <div className="font-semibold text-foreground truncate">{endpoint.name}</div>
                <div className="font-mono text-2xs text-muted-foreground truncate select-text">
                  {baseUrl}
                  {endpoint.path}
                </div>
              </div>
              <button onClick={() => copy(`${baseUrl}${endpoint.path}`, 'URL')} className={iconButtonClass} title="Copy URL">
                <MingCuteIcon name="link_line" size={13} />
              </button>
              <button onClick={() => copy(curlFor(`${baseUrl}${endpoint.path}`), 'cURL')} className={iconButtonClass} title="Copy a test cURL command">
                <MingCuteIcon name="terminal_line" size={13} />
              </button>
              {endpoint.secretKey && (
                <div className="flex items-center gap-1 pl-2 border-l border-border font-mono text-2xs min-w-0">
                  <MingCuteIcon name="key_line" size={12} className="text-amber-500 shrink-0" />
                  <span className="text-amber-500 truncate select-text">{showSecret ? endpoint.secretKey : '••••••••••'}</span>
                  <button onClick={() => setShowSecret(!showSecret)} className={iconButtonClass} title={showSecret ? 'Hide secret' : 'Show secret'}>
                    <MingCuteIcon name={showSecret ? 'eye_close_line' : 'eye_line'} size={12} />
                  </button>
                  <button onClick={() => copy(endpoint.secretKey, 'Secret')} className={iconButtonClass} title="Copy secret">
                    <MingCuteIcon name="copy_line" size={12} />
                  </button>
                </div>
              )}
              <button
                onClick={() => setDeletingEndpoint(endpoint)}
                className="p-1 rounded text-muted-foreground hover:text-rose-500 hover:bg-neutral-subtle cursor-pointer"
                title="Delete endpoint"
              >
                <MingCuteIcon name="delete_2_line" size={13} />
              </button>
            </div>
          ) : (
            <div className="flex-1 min-w-0">
              <div className="font-semibold text-foreground">
                {filter === UNMATCHED_ENDPOINT ? 'Unmatched paths' : 'All deliveries'}
              </div>
              <div className="text-2xs text-muted-foreground">
                {filter === UNMATCHED_ENDPOINT
                  ? 'Requests to paths that no endpoint is registered for'
                  : `Every request received on ${baseUrl}`}
              </div>
            </div>
          )}

          {toast && (
            <span className="flex items-center gap-1 text-2xs text-emerald-500 shrink-0">
              <MingCuteIcon name="check_line" size={12} />
              {toast}
            </span>
          )}

          <div className="relative w-56 shrink-0">
            <MingCuteIcon name="search_line" size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Filter by path or payload..."
              className="w-full bg-background border border-border rounded pl-7 pr-2 py-1 text-2xs font-mono text-foreground focus:outline-none focus:border-primary"
            />
          </div>
          <button
            onClick={handleClear}
            onMouseLeave={() => setConfirmClear(false)}
            disabled={deliveries.length === 0}
            className={`flex items-center gap-1 px-2 py-1 rounded text-2xs cursor-pointer shrink-0 disabled:opacity-40 disabled:cursor-not-allowed ${
              confirmClear ? 'bg-rose-600 text-white' : 'text-muted-foreground hover:text-rose-500 hover:bg-neutral-subtle'
            }`}
            title="Delete every captured delivery"
          >
            <MingCuteIcon name="delete_2_line" size={13} />
            <span>{confirmClear ? 'Click again to clear all' : 'Clear'}</span>
          </button>
        </div>

        <div ref={splitRef} className="flex-1 min-h-0 flex">
          <div ref={listRef} className="min-w-[240px] border-r border-border" style={{ width: `${listPercent}%` }}>
            <WebhookDeliveriesTable
              deliveries={shown}
              selectedDeliveryId={selected?.id ?? null}
              onSelectDelivery={selectDelivery}
              emptyMessage={
                deliveries.length === 0
                  ? listenerConfig.is_running
                    ? `Waiting for requests on ${baseUrl}`
                    : 'Start the receiver in the sidebar to capture requests'
                  : 'No deliveries match'
              }
            />
          </div>

          <div
            onPointerDown={handleSplitPointerDown}
            onDoubleClick={resetListPercent}
            className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
            title="Drag to resize, double-click to reset"
          >
            <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
          </div>

          <div className="flex-1 min-w-[280px]">
            <WebhookDeliveryInspector delivery={selected} />
          </div>
        </div>
      </div>

      <NewEndpointModal isOpen={isAddOpen} onClose={() => setIsAddOpen(false)} listenerPort={listenerConfig.port} />

      <Dialog isOpen={isHmacOpen} onClose={() => setIsHmacOpen(false)} title="HMAC signature tool" size="lg">
        <WebhookHmacSandbox />
      </Dialog>

      <Dialog
        isOpen={!!deletingEndpoint}
        onClose={() => setDeletingEndpoint(null)}
        title="Delete endpoint"
        size="md"
        footer={
          <>
            <Button variant="subtle" onClick={() => setDeletingEndpoint(null)}>
              Cancel
            </Button>
            <Button variant="danger" onClick={handleDeleteEndpoint}>
              Delete endpoint
            </Button>
          </>
        }
      >
        <p className="text-xs text-muted-foreground leading-relaxed">
          Delete <strong className="text-foreground font-mono">{deletingEndpoint?.path}</strong>? Its captured deliveries are
          removed too, and requests to this path will show up as unmatched.
        </p>
      </Dialog>
    </div>
  );
};
