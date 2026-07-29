import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useTraffic } from '@/hooks/traffic';
import { Button } from '@/components/ui/Button';

interface WsMessage {
  id: string;
  connectionId: string;
  direction: string;
  msgType: string;
  payload: string;
  timestamp: number;
  isIntercepted: boolean;
}

export function WebSocketView() {
  const { traffic } = useTraffic();
  const [selectedConnectionId, setSelectedConnectionId] = useState<string | null>(null);
  const [activeConnectionIds, setActiveConnectionIds] = useState<Set<string>>(new Set());
  const [connectionMessages, setConnectionMessages] = useState<WsMessage[]>([]);
  const [selectedMessageId, setSelectedMessageId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  
  // Custom injection state
  const [injectPayload, setInjectPayload] = useState('');
  const [injectDirection, setInjectDirection] = useState<'to_server' | 'to_client'>('to_server');
  const [isInjecting, setIsInjecting] = useState(false);
  const [injectSuccess, setInjectSuccess] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Filter connections (only WS 101 switching protocols)
  const wsConnections = traffic.filter(item => 
    item.status_code === 101 && 
    (item.url.toLowerCase().includes(searchQuery.toLowerCase()) || 
     item.host.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  // Load message history from DB when a connection is selected
  useEffect(() => {
    if (selectedConnectionId) {
      invoke<WsMessage[]>('get_websocket_messages', { connectionId: selectedConnectionId })
        .then(msgs => {
          setConnectionMessages(msgs);
          setSelectedMessageId(null);
          // Auto scroll to bottom
          setTimeout(() => {
            messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
          }, 100);
        })
        .catch(err => console.error('Error loading WS messages:', err));
    } else {
      setConnectionMessages([]);
    }
  }, [selectedConnectionId]);

  // Listen to live capture and closure events
  useEffect(() => {
    const unlistens: (() => void)[] = [];

    listen<WsMessage>('ws_message_captured', event => {
      const msg = event.payload;
      // Mark connection as active
      setActiveConnectionIds(prev => {
        const next = new Set(prev);
        next.add(msg.connectionId);
        return next;
      });

      // Append to list if it belongs to selected connection
      if (selectedConnectionId === msg.connectionId) {
        setConnectionMessages(prev => {
          // Prevent duplicates
          if (prev.some(m => m.id === msg.id)) {
            return prev.map(m => m.id === msg.id ? msg : m);
          }
          return [...prev, msg];
        });
        setTimeout(() => {
          messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
        }, 100);
      }
    }).then(unsub => unlistens.push(unsub));

    listen<string>('ws_connection_closed', event => {
      const connId = event.payload;
      setActiveConnectionIds(prev => {
        const next = new Set(prev);
        next.delete(connId);
        return next;
      });
    }).then(unsub => unlistens.push(unsub));

    const initialActive = new Set<string>();
    traffic.forEach(item => {
      if (item.status_code === 101) {
        initialActive.add(item.id);
      }
    });
    setActiveConnectionIds(initialActive);

    return () => {
      unlistens.forEach(unsub => unsub());
    };
  }, [selectedConnectionId, traffic]);

  const selectedConnection = traffic.find(item => item.id === selectedConnectionId);
  const selectedMessage = connectionMessages.find(msg => msg.id === selectedMessageId);

  const handleInjectFrame = async () => {
    if (!selectedConnectionId || !injectPayload.trim()) return;
    setIsInjecting(true);
    try {
      await invoke('send_websocket_message', {
        connectionId: selectedConnectionId,
        direction: injectDirection,
        payload: injectPayload,
      });
      setInjectSuccess(true);
      setInjectPayload('');
      setTimeout(() => setInjectSuccess(false), 2000);
    } catch (e) {
      console.error('Failed to inject frame:', e);
    } finally {
      setIsInjecting(false);
    }
  };

  const getFormatPayload = (payload: string) => {
    try {
      const parsed = JSON.parse(payload);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return payload;
    }
  };

  return (
    <div className="flex-1 flex overflow-hidden bg-zinc-950 text-zinc-100 font-sans">
      {/* Connections List Sidebar */}
      <div className="w-80 border-r border-zinc-800 flex flex-col bg-zinc-900/20">
        <div className="p-4 border-b border-zinc-800">
          <input
            type="text"
            placeholder="Search connections..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            className="w-full bg-zinc-900/60 border border-zinc-800 rounded px-3 py-2 text-xs text-zinc-300 placeholder-zinc-500 focus:outline-none focus:border-cyan-500/40 transition-colors"
          />
        </div>
        
        <div className="flex-1 overflow-y-auto divide-y divide-zinc-900">
          {wsConnections.length === 0 ? (
            <div className="p-8 text-center text-zinc-600 text-xs font-mono">
              NO ACTIVE OR HISTORIC WEBSOCKETS FOUND
            </div>
          ) : (
            wsConnections.map(conn => {
              const isActive = activeConnectionIds.has(conn.id);
              const isSelected = selectedConnectionId === conn.id;
              
              let pathname = '/';
              try {
                pathname = new URL(conn.url).pathname;
              } catch {
                const parts = conn.url.split('/');
                if (parts.length > 3) {
                  pathname = '/' + parts.slice(3).join('/');
                }
              }

              return (
                <button
                  key={conn.id}
                  onClick={() => setSelectedConnectionId(conn.id)}
                  className={`w-full text-left p-4 hover:bg-zinc-900/40 transition-all flex flex-col gap-1.5 relative ${
                    isSelected ? 'bg-cyan-500/5 border-l-2 border-cyan-500' : 'border-l-2 border-transparent'
                  }`}
                >
                  <div className="flex items-center justify-between w-full">
                    <span className="text-xs font-bold font-mono tracking-tight text-zinc-300 truncate max-w-[180px]">
                      {pathname}
                    </span>
                    <span className={`text-[8px] font-black uppercase tracking-wider px-2 py-0.5 rounded flex items-center gap-1.5 ${
                      isActive ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' : 'bg-zinc-800 text-zinc-500 border border-zinc-700/30'
                    }`}>
                      <span className={`h-1.5 w-1.5 rounded-full ${isActive ? 'bg-emerald-400 animate-pulse' : 'bg-zinc-500'}`}></span>
                      {isActive ? 'Live' : 'Closed'}
                    </span>
                  </div>
                  <span className="text-[10px] text-zinc-500 font-medium truncate w-full">{conn.host}</span>
                </button>
              );
            })
          )}
        </div>
      </div>

      {/* Main Content Area */}
      {selectedConnectionId && selectedConnection ? (
        <div className="flex-1 flex overflow-hidden">
          {/* Messages list */}
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Header info */}
            <div className="p-4 border-b border-zinc-800 bg-zinc-900/20 flex items-center justify-between">
              <div>
                <h3 className="text-xs font-bold font-mono text-cyan-400 truncate max-w-lg">{selectedConnection.url}</h3>
                <p className="text-[10px] text-zinc-500 mt-1 font-medium">Connection ID: {selectedConnectionId}</p>
              </div>
              <span className={`text-[10px] font-mono px-3 py-1 rounded bg-zinc-900 border border-zinc-800 text-zinc-400`}>
                Frames: <strong className="text-zinc-200">{connectionMessages.length}</strong>
              </span>
            </div>

            {/* Scrolling frames list */}
            <div className="flex-1 overflow-y-auto p-4 space-y-2 bg-zinc-950/40">
              {connectionMessages.length === 0 ? (
                <div className="h-full flex items-center justify-center text-zinc-600 text-xs font-mono">
                  WAITING FOR WEBSOCKET MESSAGES...
                </div>
              ) : (
                connectionMessages.map(msg => {
                  const isClient = msg.direction === 'client_to_server';
                  const isSelected = selectedMessageId === msg.id;
                  
                  return (
                    <button
                      key={msg.id}
                      onClick={() => setSelectedMessageId(msg.id)}
                      className={`w-full text-left p-2.5 rounded border transition-all flex items-start gap-4 ${
                        isSelected 
                          ? 'bg-cyan-500/5 border-cyan-500/30' 
                          : msg.isIntercepted
                          ? 'bg-rose-500/10 border-rose-500/40 animate-pulse'
                          : 'bg-zinc-900/30 border-zinc-800/40 hover:border-zinc-700/50'
                      }`}
                    >
                      {/* Direction arrow */}
                      <span className={`flex-shrink-0 text-[9px] font-black px-1.5 py-0.5 rounded ${
                        isClient ? 'bg-emerald-500/10 text-emerald-400' : 'bg-sky-500/10 text-sky-400'
                      }`}>
                        {isClient ? '→ OUT' : '← IN'}
                      </span>
                      
                      {/* Frame type */}
                      <span className="text-[9px] uppercase font-bold text-zinc-500 font-mono mt-0.5">{msg.msgType}</span>
                      
                      {/* Payload preview */}
                      <span className="flex-1 text-xs font-mono text-zinc-300 truncate mt-0.5">{msg.payload}</span>
                      
                      {/* Timestamp */}
                      <span className="text-[10px] font-mono text-zinc-600 mt-0.5">
                        {new Date(msg.timestamp).toLocaleTimeString()}
                      </span>
                    </button>
                  );
                })
              )}
              <div ref={messagesEndRef} />
            </div>

            {/* Injected custom frame control */}
            {activeConnectionIds.has(selectedConnectionId) && (
              <div className="p-4 border-t border-zinc-800 bg-zinc-900/10 flex flex-col gap-3">
                <div className="flex items-center justify-between">
                  <h4 className="text-[10px] font-bold uppercase tracking-wider text-cyan-400 flex items-center gap-2">
                    <span>⚡</span> Inject Custom WebSocket Message
                  </h4>
                  <div className="flex items-center gap-2">
                    <select
                      value={injectDirection}
                      onChange={e => setInjectDirection(e.target.value as any)}
                      className="bg-zinc-950 border border-zinc-805 rounded text-[10px] font-bold text-zinc-400 px-2.5 py-1 focus:outline-none focus:border-cyan-500/50 transition-colors"
                    >
                      <option value="to_server">Send to Server</option>
                      <option value="to_client">Send to Client</option>
                    </select>
                  </div>
                </div>

                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="Type custom text frame payload to inject..."
                    value={injectPayload}
                    onChange={e => setInjectPayload(e.target.value)}
                    onKeyDown={e => e.key === 'Enter' && handleInjectFrame()}
                    className="flex-1 bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-xs text-zinc-300 placeholder-zinc-600 focus:outline-none focus:border-cyan-500/50 font-mono transition-colors"
                  />
                  <Button
                    onClick={handleInjectFrame}
                    disabled={isInjecting || !injectPayload.trim()}
                    variant="sky"
                    size="sm"
                    className="px-6 font-bold"
                  >
                    {isInjecting ? 'Sending...' : 'Send'}
                  </Button>
                </div>
                
                {injectSuccess && (
                  <span className="text-[9px] text-emerald-400 font-bold uppercase tracking-widest transition-all">
                    ✓ Custom frame injected successfully
                  </span>
                )}
              </div>
            )}
          </div>

          {/* Details split inspector */}
          {selectedMessage && (
            <div className="w-[450px] border-l border-zinc-800 flex flex-col overflow-hidden bg-zinc-900/10">
              <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
                <span className="text-xs font-bold font-mono text-zinc-400">Frame Message Inspector</span>
                <button
                  onClick={() => setSelectedMessageId(null)}
                  className="text-zinc-500 hover:text-zinc-300 text-xs"
                >
                  ✕ Close
                </button>
              </div>

              <div className="p-4 space-y-4 overflow-y-auto flex-1 font-mono">
                <div className="grid grid-cols-2 gap-4 text-[10px] text-zinc-500 font-bold uppercase border-b border-zinc-800/50 pb-4">
                  <div>
                    <span>Direction</span>
                    <p className={`text-xs mt-1 font-bold font-sans ${selectedMessage.direction === 'client_to_server' ? 'text-emerald-400' : 'text-sky-400'}`}>
                      {selectedMessage.direction === 'client_to_server' ? 'Client → Server (Outbound)' : 'Server → Client (Inbound)'}
                    </p>
                  </div>
                  <div>
                    <span>Type</span>
                    <p className="text-xs text-zinc-200 mt-1 uppercase">{selectedMessage.msgType}</p>
                  </div>
                  <div>
                    <span>Timestamp</span>
                    <p className="text-xs text-zinc-200 mt-1">{new Date(selectedMessage.timestamp).toLocaleString()}</p>
                  </div>
                  <div>
                    <span>Length</span>
                    <p className="text-xs text-zinc-200 mt-1">{selectedMessage.payload.length} Bytes</p>
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-[10px] text-zinc-500 font-bold uppercase">Payload Data</span>
                    <button
                      onClick={() => navigator.clipboard.writeText(selectedMessage.payload)}
                      className="text-[9px] text-cyan-400 hover:text-cyan-300 font-bold uppercase tracking-wider"
                    >
                      Copy Payload
                    </button>
                  </div>
                  <pre className="bg-zinc-950 p-4 border border-zinc-800 rounded text-xs text-zinc-300 overflow-x-auto whitespace-pre-wrap leading-relaxed max-h-[350px]">
                    {getFormatPayload(selectedMessage.payload)}
                  </pre>
                </div>
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center text-zinc-500 text-xs font-mono bg-zinc-950/20">
          SELECT A WEBSOCKET CONNECTION FROM THE SIDEBAR TO VIEW FRAMES
        </div>
      )}
    </div>
  );
}
