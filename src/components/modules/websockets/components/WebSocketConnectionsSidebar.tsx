import React, { useMemo, useState } from 'react';
import { useWebSocketStore } from '../../../../stores/useWebSocketStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button } from '../../../common/ui';

interface WebSocketConnectionsSidebarProps {
  widthPx: number;
  onOpenConnectModal: () => void;
}

export const WebSocketConnectionsSidebar: React.FC<WebSocketConnectionsSidebarProps> = ({
  widthPx,
  onOpenConnectModal,
}) => {
  const {
    connections,
    selectedConnectionId,
    selectConnection,
    disconnectConnection,
    deleteConnection,
    wsMitmEnabled,
    toggleWsMitm,
  } = useWebSocketStore();

  const [streamScope, setStreamScope] = useState<'all' | 'proxy' | 'client'>('all');
  const [sourceFilter, setSourceFilter] = useState('');

  // Labels seen on captured streams, so older/renamed sources stay filterable
  const sourceLabels = useMemo(
    () =>
      Array.from(new Set(connections.map((c) => c.listenerLabel).filter((l): l is string => !!l))).sort(),
    [connections]
  );

  const filteredConnections = connections.filter((conn) => {
    if (streamScope === 'proxy' && conn.isClientSession) return false;
    if (streamScope === 'client' && !conn.isClientSession) return false;
    if (sourceFilter && conn.listenerLabel !== sourceFilter) return false;
    return true;
  });

  return (
    <div
      style={{ width: `${widthPx}px` }}
      className="border-r border-border bg-surface flex flex-col overflow-hidden shrink-0 select-none min-w-[200px]"
    >
      {/* Header with Title & + Connect button */}
      <div className="p-3 bg-header border-b border-border flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded-md bg-cyan-500/10 flex items-center justify-center text-cyan-500">
            <MingCuteIcon name="websocket_line" size={14} />
          </div>
          <div>
            <span className="font-semibold text-foreground text-xs block">WebSocket Streams</span>
            <span className="text-[10px] text-muted-foreground">{connections.length} active sessions</span>
          </div>
        </div>

        <Button
          variant="primary"
          size="xs"
          icon="add_line"
          onClick={onOpenConnectModal}
        >
          Connect
        </Button>
      </div>

      {/* Scope Filters: All / MITM Proxy / Client Tester */}
      <div className="p-2 border-b border-border/60 bg-surface flex items-center gap-1">
        {(['all', 'proxy', 'client'] as const).map((s) => (
          <button
            key={s}
            onClick={() => setStreamScope(s)}
            className={`flex-1 py-1 rounded text-[11px] font-medium transition-colors text-center ${
              streamScope === s
                ? 'bg-primary text-primary-foreground font-semibold'
                : 'text-muted-foreground hover:text-foreground bg-neutral-subtle/50'
            }`}
          >
            {s === 'all' ? 'All' : s === 'proxy' ? 'Proxy (MITM)' : 'Client Sessions'}
          </button>
        ))}
      </div>

      {/* Source (listener) filter */}
      {sourceLabels.length > 0 && (
        <div className="px-2 py-1.5 border-b border-border/60 bg-surface flex items-center gap-1.5 text-[11px] font-mono">
          <MingCuteIcon name="route_line" size={13} className="text-primary shrink-0" />
          <select
            value={sourceFilter}
            onChange={(e) => setSourceFilter(e.target.value)}
            className="flex-1 bg-transparent text-foreground focus:outline-none cursor-pointer"
            title="Filter streams by source listener"
          >
            <option value="">All Sources</option>
            {sourceLabels.map((label) => (
              <option key={label} value={label}>
                {label}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* WS Proxy MITM Interception Toggle Banner */}
      <div className="px-3 py-2 bg-neutral-subtle/40 border-b border-border/70 flex items-center justify-between">
        <div className="flex items-center gap-1.5 overflow-hidden">
          <span
            className={`w-2 h-2 rounded-full shrink-0 ${
              wsMitmEnabled ? 'bg-cyan-500 animate-pulse' : 'bg-slate-400'
            }`}
          />
          <span className="text-[11px] font-medium text-foreground">Proxy MITM:</span>
          <span
            className={`px-1.5 py-0.2 rounded text-[9px] font-bold uppercase border ${
              wsMitmEnabled
                ? 'bg-cyan-500/15 text-cyan-600 dark:text-cyan-400 border-cyan-500/30'
                : 'bg-neutral-subtle text-muted-foreground border-border'
            }`}
          >
            {wsMitmEnabled ? 'Active' : 'Disabled'}
          </span>
        </div>

        <Button
          variant={wsMitmEnabled ? 'destructive' : 'primary'}
          size="xs"
          onClick={() => toggleWsMitm()}
        >
          {wsMitmEnabled ? 'Disable' : 'Enable'}
        </Button>
      </div>

      {/* Stream List */}
      <div className="flex-1 overflow-y-auto divide-y divide-border/60">
        {filteredConnections.length === 0 ? (
          <div className="p-6 text-center text-muted-foreground italic font-sans space-y-2">
            <MingCuteIcon name="websocket_line" size={24} className="opacity-40 mx-auto" />
            <p>No WebSocket streams available.</p>
            <p className="text-[10px]">
              {wsMitmEnabled
                ? 'Proxy WebSocket traffic through MITM or click "+ Connect" to establish a direct client connection.'
                : 'Enable Proxy MITM above to intercept proxied WebSocket frames, or click "+ Connect" for client studio.'}
            </p>
          </div>
        ) : (
          filteredConnections.map((conn) => {
            const isSelected = conn.connectionId === selectedConnectionId;
            const isConnected = conn.status === 'connected';

            return (
              <div
                key={conn.connectionId}
                onClick={() => selectConnection(conn.connectionId)}
                className={`p-2.5 cursor-pointer font-mono text-xs transition-colors space-y-1.5 ${
                  isSelected
                    ? 'bg-primary/15 font-semibold text-foreground ring-1 ring-inset ring-primary'
                    : 'hover:bg-neutral-subtle/60'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        isConnected ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'
                      }`}
                    />
                    <span
                      className={`px-1.5 py-0.2 rounded text-[10px] uppercase font-bold border ${
                        isConnected
                          ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                          : 'bg-neutral-subtle text-muted-foreground border-border'
                      }`}
                    >
                      {conn.status}
                    </span>
                    <span
                      className={`px-1.5 py-0.2 rounded text-[9px] font-bold ${
                        conn.isClientSession
                          ? 'bg-purple-500/15 text-purple-600 dark:text-purple-400 border border-purple-500/30'
                          : 'bg-cyan-500/15 text-cyan-600 dark:text-cyan-400 border border-cyan-500/30'
                      }`}
                    >
                      {conn.isClientSession ? 'CLIENT' : 'PROXY'}
                    </span>
                    {conn.listenerLabel && (
                      <span
                        className="px-1.5 py-0.2 rounded text-[9px] font-bold bg-primary/10 text-primary border border-primary/20 truncate max-w-[90px]"
                        title={`Source: ${conn.listenerLabel}`}
                      >
                        {conn.listenerLabel}
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-1">
                    <span className="px-1.5 py-0.2 rounded text-[10px] font-bold bg-neutral-subtle text-muted-foreground border border-border">
                      {conn.messageCount || 0} msgs
                    </span>
                  </div>
                </div>

                <div className="text-foreground text-[11px] truncate font-bold" title={conn.url}>
                  {conn.url}
                </div>

                <div className="flex items-center justify-between text-[10px] text-muted-foreground font-sans pt-0.5">
                  <span>{new Date(conn.handshakeTime).toLocaleTimeString()}</span>
                  <div className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
                    {isConnected ? (
                      <button
                        onClick={() => disconnectConnection(conn.connectionId)}
                        className="hover:text-amber-500 p-0.5"
                        title="Disconnect Session"
                      >
                        <MingCuteIcon name="power_line" size={13} />
                      </button>
                    ) : null}
                    <button
                      onClick={() => deleteConnection(conn.connectionId)}
                      className="hover:text-rose-500 p-0.5"
                      title="Delete Session Log"
                    >
                      <MingCuteIcon name="delete_2_line" size={13} />
                    </button>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
