import React from 'react';
import type { WebSocketConn } from '../../../../types';
import { useWebSocketStore } from '../../../../stores/useWebSocketStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button } from '../../../common/ui';

interface WebSocketMessageComposerProps {
  connection: WebSocketConn;
  heightPx: number;
}

export const WebSocketMessageComposer: React.FC<WebSocketMessageComposerProps> = ({
  connection,
  heightPx,
}) => {
  const {
    messageBuilderType,
    setMessageBuilderType,
    messageBuilderContent,
    setMessageBuilderContent,
    sendMessage,
  } = useWebSocketStore();

  const handleQuickTemplate = (type: 'ping' | 'subscribe' | 'echo') => {
    if (type === 'ping') {
      setMessageBuilderType('json');
      setMessageBuilderContent(JSON.stringify({ event: 'ping', timestamp: Date.now() }, null, 2));
    } else if (type === 'subscribe') {
      setMessageBuilderType('json');
      setMessageBuilderContent(
        JSON.stringify({ method: 'SUBSCRIBE', params: ['btcusdt@trade'], id: 1 }, null, 2),
      );
    } else {
      setMessageBuilderType('text');
      setMessageBuilderContent(`Hello WebSocket Server! Timestamp: ${new Date().toISOString()}`);
    }
  };

  return (
    <div
      style={{ height: `${heightPx}px` }}
      className="p-3 bg-header border-t border-border flex flex-col gap-2 shrink-0 overflow-hidden min-h-[80px]"
    >
      <div className="flex items-center justify-between font-mono shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1 font-semibold text-foreground font-sans">
            <MingCuteIcon name="send_plane_line" size={14} className="text-primary" />
            <span>Frame Injector:</span>
          </div>

          {/* Message Format */}
          <div className="flex items-center gap-1 bg-surface border border-border rounded p-0.5 text-xs">
            {(['json', 'text', 'binary'] as const).map((t) => (
              <button
                key={t}
                onClick={() => setMessageBuilderType(t)}
                className={`px-2 py-0.5 rounded uppercase font-bold text-[10px] transition-colors ${
                  messageBuilderType === t
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                {t}
              </button>
            ))}
          </div>

          {/* Quick Templates */}
          <div className="flex items-center gap-1">
            <span className="text-[11px] text-muted-foreground font-sans">Templates:</span>
            <button
              onClick={() => handleQuickTemplate('ping')}
              className="px-1.5 py-0.5 rounded text-[10px] bg-neutral-subtle text-foreground border border-border hover:border-primary transition-colors"
            >
              Ping
            </button>
            <button
              onClick={() => handleQuickTemplate('subscribe')}
              className="px-1.5 py-0.5 rounded text-[10px] bg-neutral-subtle text-foreground border border-border hover:border-primary transition-colors"
            >
              Subscribe
            </button>
            <button
              onClick={() => handleQuickTemplate('echo')}
              className="px-1.5 py-0.5 rounded text-[10px] bg-neutral-subtle text-foreground border border-border hover:border-primary transition-colors"
            >
              Echo
            </button>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            size="xs"
            icon="arrow_up_line"
            onClick={() => sendMessage('to_server')}
            disabled={connection.status !== 'connected' || !messageBuilderContent.trim()}
          >
            Send to Server
          </Button>
          <Button
            variant="secondary"
            size="xs"
            icon="arrow_down_line"
            onClick={() => sendMessage('to_client')}
            disabled={connection.status !== 'connected' || !messageBuilderContent.trim()}
          >
            Inject to Client
          </Button>
        </div>
      </div>

      <textarea
        value={messageBuilderContent}
        onChange={(e) => setMessageBuilderContent(e.target.value)}
        placeholder="Type raw text, JSON payload, or hex string for binary frames..."
        className="flex-1 w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary resize-none overflow-y-auto"
      />
    </div>
  );
};
