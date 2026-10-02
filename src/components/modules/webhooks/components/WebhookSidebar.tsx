import React, { useEffect, useState } from 'react';
import { useWebhookStore } from '../../../../stores/useWebhookStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Switch } from '../../../common/ui';
import { ContextMenu, type ContextMenuItem } from '../../../common/ContextMenu';
import type { WebhookEndpoint } from '../../../../types';

export const ALL_ENDPOINTS = 'all';
export const UNMATCHED_ENDPOINT = 'unmatched';

interface WebhookSidebarProps {
  selectedFilter: string;
  onSelectFilter: (filter: string) => void;
  onAddEndpoint: () => void;
  onOpenHmacTool: () => void;
  onDeleteEndpoint: (endpoint: WebhookEndpoint) => void;
  onCopy: (text: string, label: string) => void;
}

export const WebhookSidebar: React.FC<WebhookSidebarProps> = ({
  selectedFilter,
  onSelectFilter,
  onAddEndpoint,
  onOpenHmacTool,
  onDeleteEndpoint,
  onCopy,
}) => {
  const { listenerConfig, toggleServer, savePort, endpoints, deliveries } = useWebhookStore();
  const [portDraft, setPortDraft] = useState(String(listenerConfig.port));
  const [portStatus, setPortStatus] = useState<{ type: 'saved' | 'error'; message: string } | null>(null);
  const [toggleError, setToggleError] = useState<string | null>(null);
  const [isToggling, setIsToggling] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; endpoint: WebhookEndpoint } | null>(null);

  const baseUrl = `http://127.0.0.1:${listenerConfig.port}`;
  const unmatchedCount = deliveries.filter((d) => d.endpointId === UNMATCHED_ENDPOINT).length;

  useEffect(() => {
    setPortDraft(String(listenerConfig.port));
  }, [listenerConfig.port]);

  const commitPort = async (): Promise<boolean> => {
    if (portDraft === String(listenerConfig.port)) {
      setPortStatus(null);
      return true;
    }
    const port = Number(portDraft);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      setPortStatus({ type: 'error', message: 'Port must be between 1 and 65535' });
      return false;
    }
    try {
      await savePort(port);
      setPortStatus({ type: 'saved', message: 'Saved, used on next start' });
      setTimeout(() => setPortStatus((s) => (s?.type === 'saved' ? null : s)), 2500);
      return true;
    } catch (err) {
      setPortStatus({ type: 'error', message: String(err) });
      return false;
    }
  };

  const handleToggle = async () => {
    if (isToggling) return;
    if (!listenerConfig.is_running && !(await commitPort())) return;
    setIsToggling(true);
    setToggleError(null);
    try {
      await toggleServer();
    } catch (err) {
      setToggleError(String(err));
    } finally {
      setIsToggling(false);
    }
  };

  const menuItems = (ep: WebhookEndpoint): ContextMenuItem[] => [
    { label: 'Copy URL', icon: 'link_line', action: () => onCopy(`${baseUrl}${ep.path}`, 'URL') },
    { label: 'Copy secret', icon: 'key_line', disabled: !ep.secretKey, action: () => onCopy(ep.secretKey, 'Secret') },
    { label: 'Delete', icon: 'delete_2_line', danger: true, action: () => onDeleteEndpoint(ep) },
  ];

  const filterRow = (
    key: string,
    label: React.ReactNode,
    count: number,
    extra?: { sub?: string; onContextMenu?: (e: React.MouseEvent) => void }
  ) => {
    const isActive = selectedFilter === key;
    return (
      <button
        key={key}
        onClick={() => onSelectFilter(key)}
        onContextMenu={extra?.onContextMenu}
        className={`w-full flex items-center gap-2 px-2 py-1.5 rounded text-left cursor-pointer transition-colors ${
          isActive ? 'bg-primary/10 text-foreground' : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
        }`}
      >
        <span className="min-w-0 flex-1">
          <span className={`block truncate font-mono text-xs ${isActive ? 'font-semibold' : ''}`}>{label}</span>
          {extra?.sub && <span className="block truncate text-3xs text-muted-foreground">{extra.sub}</span>}
        </span>
        <span className="text-3xs font-mono text-muted-foreground tabular-nums">{count}</span>
      </button>
    );
  };

  return (
    <div className="h-full flex flex-col bg-surface border-r border-border select-none">
      <div className="p-3 border-b border-border space-y-2">
        <div className="flex items-center gap-2">
          <span
            className={`w-2 h-2 rounded-full shrink-0 ${
              listenerConfig.is_running ? 'bg-emerald-400 animate-pulse' : 'bg-muted-foreground/40'
            }`}
          />
          <span className="flex-1 font-semibold text-foreground">
            {listenerConfig.is_running ? 'Receiving' : 'Receiver stopped'}
          </span>
          <Switch
            checked={listenerConfig.is_running}
            onChange={handleToggle}
            title={listenerConfig.is_running ? 'Stop the receiver' : 'Start the receiver'}
          />
        </div>

        <div className="flex items-center gap-1.5 font-mono text-2xs">
          <span className="text-muted-foreground">127.0.0.1:</span>
          <input
            type="text"
            inputMode="numeric"
            value={portDraft}
            onChange={(e) => {
              setPortDraft(e.target.value.replace(/\D/g, '').slice(0, 5));
              setPortStatus(null);
            }}
            onBlur={commitPort}
            onKeyDown={(e) => {
              if (e.key === 'Enter') e.currentTarget.blur();
              if (e.key === 'Escape') {
                setPortDraft(String(listenerConfig.port));
                setPortStatus(null);
              }
            }}
            disabled={listenerConfig.is_running}
            title={listenerConfig.is_running ? 'Stop the receiver to change the port' : 'Port to listen on. Enter to save'}
            className={`w-16 bg-background border rounded px-1.5 py-0.5 text-foreground focus:outline-none disabled:opacity-60 ${
              portStatus?.type === 'error' ? 'border-rose-500' : 'border-border focus:border-primary'
            }`}
          />
          <button
            onClick={() => onCopy(baseUrl, 'Base URL')}
            className="ml-auto p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
            title={`Copy ${baseUrl}`}
          >
            <MingCuteIcon name="copy_line" size={12} />
          </button>
        </div>

        {portStatus && (
          <div className={`flex items-center gap-1 text-3xs ${portStatus.type === 'error' ? 'text-rose-500' : 'text-emerald-500'}`}>
            <MingCuteIcon name={portStatus.type === 'error' ? 'alert_line' : 'check_line'} size={11} className="shrink-0" />
            <span className="break-all">{portStatus.message}</span>
          </div>
        )}
        {listenerConfig.is_running && (
          <div className="text-3xs text-muted-foreground">Stop the receiver to change the port</div>
        )}
        {toggleError && <div className="text-rose-500 font-mono text-3xs break-all">{toggleError}</div>}
      </div>

      <div className="px-3 pt-3 pb-1 flex items-center gap-2">
        <span className="flex-1 text-3xs font-semibold uppercase tracking-wider text-muted-foreground">
          Endpoints <span className="font-mono">{endpoints.length}</span>
        </span>
        <button
          onClick={onAddEndpoint}
          className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
          title="Add endpoint"
        >
          <MingCuteIcon name="plus_line" size={14} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-2 space-y-0.5">
        {filterRow(ALL_ENDPOINTS, <span className="font-sans">All deliveries</span>, deliveries.length)}
        {endpoints.map((ep) =>
          filterRow(ep.id, ep.path, ep.hitCount, {
            sub: ep.name,
            onContextMenu: (e) => {
              e.preventDefault();
              setContextMenu({ x: e.clientX, y: e.clientY, endpoint: ep });
            },
          })
        )}
        {unmatchedCount > 0 &&
          filterRow(UNMATCHED_ENDPOINT, <span className="font-sans italic">Unmatched paths</span>, unmatchedCount, {
            sub: 'Requests to paths with no endpoint',
          })}
        {endpoints.length === 0 && (
          <button
            onClick={onAddEndpoint}
            className="w-full mt-1 px-2 py-3 rounded border border-dashed border-border text-2xs text-muted-foreground hover:text-foreground cursor-pointer"
          >
            Add your first endpoint
          </button>
        )}
      </div>

      <div className="p-2 border-t border-border">
        <button
          onClick={onOpenHmacTool}
          className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded border border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle text-xs cursor-pointer transition-colors"
          title="Compute a signature header for a payload and secret"
        >
          <MingCuteIcon name="key_line" size={14} />
          <span>HMAC signature tool</span>
        </button>
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={menuItems(contextMenu.endpoint)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};
