import React, { useState } from 'react';
import { useWebSocketStore } from '../../../stores/useWebSocketStore';
import { CodeEditor } from '../../common/CodeEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const WebSocketsView: React.FC = () => {
  const {
    connections,
    selectedConnectionId,
    selectConnection,
    messages,
    directionFilter,
    setDirectionFilter,
    messageBuilderType,
    setMessageBuilderType,
    messageBuilderContent,
    setMessageBuilderContent,
    selectedMessageId,
    selectMessage,
    sendLiveMessage,
  } = useWebSocketStore();

  const [payloadFmt, setPayloadFmt] = useState<'pretty' | 'raw' | 'hex'>('pretty');

  const activeConnection = connections.find((c) => c.connectionId === selectedConnectionId) || null;
  const connMessages = selectedConnectionId ? messages[selectedConnectionId] || [] : [];

  const filteredMessages = connMessages.filter((msg) => {
    if (directionFilter === 'in' && msg.direction !== 'to_client') return false;
    if (directionFilter === 'out' && msg.direction !== 'to_server') return false;
    return true;
  });

  const selectedMsg = connMessages.find((m) => m.id === selectedMessageId) || (filteredMessages.length > 0 ? filteredMessages[0] : null);

  const handleSendToServer = async () => {
    await sendLiveMessage('to_server');
  };

  const handleSendToClient = async () => {
    await sendLiveMessage('to_client');
  };

  return (
    <div className="h-full flex bg-background overflow-hidden text-xs">
      {/* Left Panel: Connections List */}
      <div className="w-64 border-r border-border bg-surface flex flex-col overflow-hidden shrink-0 select-none">
        <div className="p-2.5 bg-header border-b border-border font-semibold flex items-center gap-2">
          <MingCuteIcon name="websocket_line" size={16} className="text-cyan-500" />
          <span>WebSocket Streams</span>
        </div>

        <div className="flex-1 overflow-y-auto divide-y divide-border">
          {connections.length === 0 ? (
            <div className="p-4 text-center text-muted-foreground italic">No WebSocket streams logged</div>
          ) : (
            connections.map((conn) => {
              const isSelected = conn.connectionId === selectedConnectionId;
              return (
                <div
                  key={conn.connectionId}
                  onClick={() => selectConnection(conn.connectionId)}
                  className={`p-2.5 cursor-pointer font-mono text-xs transition-colors space-y-1 ${
                    isSelected ? 'bg-primary/15 font-semibold text-foreground border-l-2 border-primary' : 'hover:bg-neutral-subtle'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span
                      className={`px-1.5 py-0.2 rounded text-[10px] uppercase font-bold ${
                        conn.status === 'connected' ? 'bg-emerald-500/10 text-emerald-500' : 'bg-slate-500/10 text-slate-400'
                      }`}
                    >
                      {conn.status}
                    </span>
                    <span className="text-[10px] text-muted-foreground">
                      {new Date(conn.handshakeTime).toLocaleTimeString()}
                    </span>
                  </div>
                  <div className="text-foreground truncate">{conn.url}</div>
                </div>
              );
            })
          )}
        </div>
      </div>

      {/* Right Panel: Live Stream Table + Payload Inspector + Bottom Dock */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {activeConnection ? (
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Connection Header & Filter Bar */}
            <div className="p-2.5 bg-header border-b border-border flex items-center justify-between shrink-0">
              <div className="flex items-center gap-2 font-mono">
                <span className="font-bold text-foreground text-xs">{activeConnection.url}</span>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-muted-foreground text-[11px]">Filter Stream:</span>
                {(['all', 'in', 'out'] as const).map((dir) => (
                  <button
                    key={dir}
                    onClick={() => setDirectionFilter(dir)}
                    className={`px-2 py-0.5 rounded text-[10px] uppercase font-mono ${
                      directionFilter === dir ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground bg-neutral-subtle'
                    }`}
                  >
                    {dir === 'in' ? 'IN (Client)' : dir === 'out' ? 'OUT (Server)' : 'ALL'}
                  </button>
                ))}
              </div>
            </div>

            {/* Split Stream Table (Top 50%) & Payload Inspector (Bottom 50%) */}
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* Message Stream Table */}
              <div className="h-1/2 border-b border-border bg-surface flex flex-col overflow-hidden">
                <div className="bg-header border-b border-border flex items-center text-[11px] font-medium text-muted-foreground select-none shrink-0 font-mono px-2 py-1">
                  <span className="w-16 text-center">Dir</span>
                  <span className="w-16">Type</span>
                  <span className="w-24">Time</span>
                  <span className="w-20 text-right px-2">Length</span>
                  <span className="flex-1 truncate px-2">Payload Preview</span>
                </div>

                <div className="flex-1 overflow-auto divide-y divide-border/50 font-mono">
                  {filteredMessages.length === 0 ? (
                    <div className="py-8 text-center text-muted-foreground italic font-sans">
                      No frame messages in stream
                    </div>
                  ) : (
                    filteredMessages.map((msg) => {
                      const isSelected = selectedMsg?.id === msg.id;
                      const isIncoming = msg.direction === 'to_client';

                      return (
                        <div
                          key={msg.id}
                          onClick={() => selectMessage(msg.id)}
                          className={`flex items-center px-2 py-1 text-xs cursor-pointer transition-colors ${
                            isSelected ? 'bg-primary/15 font-semibold text-foreground' : 'hover:bg-neutral-subtle'
                          }`}
                        >
                          <span className="w-16 flex items-center justify-center">
                            <span
                              className={`flex items-center gap-0.5 px-1 py-0.2 rounded text-[10px] font-bold ${
                                isIncoming ? 'text-emerald-500 bg-emerald-500/10' : 'text-sky-500 bg-sky-500/10'
                              }`}
                            >
                              <MingCuteIcon name={isIncoming ? 'arrow_down_line' : 'arrow_up_line'} size={12} />
                              {isIncoming ? 'IN' : 'OUT'}
                            </span>
                          </span>
                          <span className="w-16 uppercase text-[10px] font-semibold text-muted-foreground">{msg.msg_type}</span>
                          <span className="w-24 text-muted-foreground text-[10px]">
                            {new Date(msg.timestamp).toLocaleTimeString()}
                          </span>
                          <span className="w-20 text-right px-2 text-muted-foreground text-[11px]">{msg.payload.length} B</span>
                          <span className="flex-1 truncate px-2 text-foreground">{msg.payload.replace(/\s+/g, ' ')}</span>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>

              {/* Selected Frame Payload Inspector */}
              <div className="h-1/2 flex flex-col bg-surface overflow-hidden">
                <div className="p-2 bg-header border-b border-border flex items-center justify-between shrink-0 font-mono">
                  <span className="font-semibold text-foreground text-[11px] uppercase">Frame Payload Viewer</span>
                  <div className="flex items-center gap-1">
                    {(['pretty', 'raw', 'hex'] as const).map((fmt) => (
                      <button
                        key={fmt}
                        onClick={() => setPayloadFmt(fmt)}
                        className={`px-2 py-0.5 rounded text-[10px] uppercase ${
                          payloadFmt === fmt ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground bg-neutral-subtle'
                        }`}
                      >
                        {fmt}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="flex-1 p-2 overflow-hidden">
                  {selectedMsg ? (
                    payloadFmt === 'hex' ? (
                      <HexViewer content={selectedMsg.payload} />
                    ) : payloadFmt === 'pretty' ? (
                      <CodeEditor value={selectedMsg.payload} language="json" readOnly />
                    ) : (
                      <CodeEditor value={selectedMsg.payload} language="plaintext" readOnly />
                    )
                  ) : (
                    <div className="h-full flex items-center justify-center text-muted-foreground italic">
                      Select a frame row above
                    </div>
                  )}
                </div>
              </div>
            </div>

            {/* Bottom Dock: Live Message Builder */}
            <div className="p-3 bg-header border-t border-border flex flex-col gap-2 shrink-0">
              <div className="flex items-center justify-between font-mono">
                <div className="flex items-center gap-2">
                  <span className="font-bold text-foreground">WebSocket Message Injector:</span>
                  {(['json', 'text', 'binary'] as const).map((t) => (
                    <label key={t} className="flex items-center gap-1 cursor-pointer text-muted-foreground">
                      <input
                        type="radio"
                        name="wsMsgType"
                        checked={messageBuilderType === t}
                        onChange={() => setMessageBuilderType(t)}
                        className="text-primary"
                      />
                      <span className="uppercase text-[10px] font-bold">{t}</span>
                    </label>
                  ))}
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={handleSendToServer}
                    className="flex items-center gap-1 px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-white font-semibold shadow-xs"
                  >
                    <MingCuteIcon name="arrow_up_line" size={14} />
                    <span>Send to Server</span>
                  </button>
                  <button
                    onClick={handleSendToClient}
                    className="flex items-center gap-1 px-3 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold shadow-xs"
                  >
                    <MingCuteIcon name="arrow_down_line" size={14} />
                    <span>Inject to Client</span>
                  </button>
                </div>
              </div>

              <textarea
                rows={2}
                value={messageBuilderContent}
                onChange={(e) => setMessageBuilderContent(e.target.value)}
                placeholder="Type raw text, JSON frame payload..."
                className="w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
              />
            </div>
          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center text-muted-foreground italic text-xs p-6">
            <MingCuteIcon name="websocket_line" size={40} className="mb-2 opacity-30" />
            Select a WebSocket stream from the left panel.
          </div>
        )}
      </div>
    </div>
  );
};
