import React from 'react';
import type { WebSocketConn, WebSocketMessage } from '../../../../types';
import { useWebSocketStore } from '../../../../stores/useWebSocketStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button, Input } from '../../../common/ui';

interface WebSocketMessageListProps {
  connection: WebSocketConn;
  tableHeightPercent: number;
  filteredMessages: WebSocketMessage[];
  totalMessageCount: number;
  selectedMessageId: string | null;
  onSelectMessage: (id: string) => void;
}

export const WebSocketMessageList: React.FC<WebSocketMessageListProps> = ({
  connection,
  tableHeightPercent,
  filteredMessages,
  totalMessageCount,
  selectedMessageId,
  onSelectMessage,
}) => {
  const {
    directionFilter,
    setDirectionFilter,
    searchQuery,
    setSearchQuery,
    clearMessages,
  } = useWebSocketStore();

  return (
    <>
      {/* Stream Top Action & Filter Toolbar */}
      <div className="p-2.5 bg-header border-b border-border flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2 font-mono overflow-hidden">
          <span
            className={`w-2.5 h-2.5 rounded-full shrink-0 ${
              connection.status === 'connected' ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'
            }`}
          />
          <span className="font-bold text-foreground text-xs truncate max-w-md" title={connection.url}>
            {connection.url}
          </span>
          <span className="text-[10px] text-muted-foreground shrink-0">
            ({connection.isClientSession ? 'Direct Studio Client' : 'MITM Intercept Stream'})
          </span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {/* Search Filter */}
          <div className="w-48">
            <Input
              placeholder="Search payload frames..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              leftIcon="search_line"
            />
          </div>

          {/* Direction Filter */}
          <div className="flex items-center gap-0.5 bg-surface border border-border rounded p-0.5 font-mono text-[10px]">
            {(['all', 'in', 'out'] as const).map((dir) => (
              <button
                key={dir}
                onClick={() => setDirectionFilter(dir)}
                className={`px-2 py-0.5 rounded font-bold transition-colors ${
                  directionFilter === dir
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                {dir === 'in' ? 'IN (Client)' : dir === 'out' ? 'OUT (Server)' : 'ALL'}
              </button>
            ))}
          </div>

          <Button
            variant="destructive"
            size="xs"
            icon="delete_2_line"
            onClick={() => clearMessages(connection.connectionId)}
            disabled={totalMessageCount === 0}
          >
            Clear Frames
          </Button>
        </div>
      </div>

      {/* Message Frames Stream Table */}
      <div
        style={{ height: `${tableHeightPercent}%` }}
        className="border-b border-border bg-surface flex flex-col overflow-hidden min-h-[100px]"
      >
        <div className="bg-header border-b border-border flex items-center text-[11px] font-medium text-muted-foreground select-none shrink-0 font-mono px-3 py-1.5">
          <span className="w-16 text-center font-sans">Direction</span>
          <span className="w-16 font-sans">Type</span>
          <span className="w-24 font-sans">Time</span>
          <span className="w-20 text-right px-2 font-sans">Length</span>
          <span className="flex-1 truncate px-2 font-sans">Payload Preview</span>
        </div>

        <div className="flex-1 overflow-y-auto divide-y divide-border/40 font-mono">
          {filteredMessages.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground italic font-sans">
              {totalMessageCount === 0
                ? 'No WebSocket frames captured yet in this stream.'
                : 'No frames match your search or direction filter.'}
            </div>
          ) : (
            filteredMessages.map((msg) => {
              const isSelected = selectedMessageId === msg.id;
              const isIncoming = msg.direction === 'to_client';

              return (
                <div
                  key={msg.id}
                  onClick={() => onSelectMessage(msg.id)}
                  className={`flex items-center px-3 py-1.5 text-xs cursor-pointer transition-colors ${
                    isSelected
                      ? 'bg-primary/15 font-semibold text-foreground ring-1 ring-inset ring-primary'
                      : 'hover:bg-neutral-subtle/60'
                  }`}
                >
                  <span className="w-16 flex items-center justify-center">
                    <span
                      className={`flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-bold border ${
                        isIncoming
                          ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-500/10 border-emerald-500/30'
                          : 'text-sky-600 dark:text-sky-400 bg-sky-500/10 border-sky-500/30'
                      }`}
                    >
                      <MingCuteIcon name={isIncoming ? 'arrow_down_line' : 'arrow_up_line'} size={12} />
                      {isIncoming ? 'IN' : 'OUT'}
                    </span>
                  </span>

                  <span className="w-16">
                    <span className="px-1.5 py-0.5 rounded text-[9px] uppercase font-bold bg-neutral-subtle text-muted-foreground border border-border">
                      {msg.msg_type}
                    </span>
                  </span>

                  <span className="w-24 text-muted-foreground text-[10px]">
                    {new Date(msg.timestamp).toLocaleTimeString()}
                  </span>

                  <span className="w-20 text-right px-2 text-muted-foreground text-[11px]">
                    {msg.length || msg.payload.length} B
                  </span>

                  <div className="flex-1 flex items-center gap-2 truncate px-2">
                    {msg.is_injected && (
                      <span className="px-1 py-0.2 rounded text-[9px] font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30">
                        INJECTED
                      </span>
                    )}
                    <span className="truncate text-foreground text-xs">{msg.payload.replace(/\s+/g, ' ')}</span>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </>
  );
};
