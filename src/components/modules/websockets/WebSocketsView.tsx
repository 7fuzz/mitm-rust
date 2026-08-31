import React, { useState, useEffect } from 'react';
import { useWebSocketStore } from '../../../stores/useWebSocketStore';
import { CodeEditor } from '../../common/CodeEditor';
import { HexViewer } from '../../common/HexViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button, Input, Dialog } from '../../common/ui';

export const WebSocketsView: React.FC = () => {
  const {
    initialize,
    connections,
    selectedConnectionId,
    selectConnection,
    messages,
    directionFilter,
    setDirectionFilter,
    searchQuery,
    setSearchQuery,
    messageBuilderType,
    setMessageBuilderType,
    messageBuilderContent,
    setMessageBuilderContent,
    selectedMessageId,
    selectMessage,
    sendMessage,
    clearMessages,
    createClientConnection,
    disconnectConnection,
    deleteConnection,
    isConnecting,
  } = useWebSocketStore();

  const [activeTab, setActiveTab] = useState<'pretty' | 'raw' | 'hex' | 'info'>('pretty');
  const [streamScope, setStreamScope] = useState<'all' | 'proxy' | 'client'>('all');
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  // Resizable sidebar and stream split states
  const [sidebarWidthPx, setSidebarWidthPx] = useState<number>(288);
  const containerRef = React.useRef<HTMLDivElement>(null);
  const isResizingSidebar = React.useRef(false);

  const [tableHeightPercent, setTableHeightPercent] = useState<number>(50);
  const streamAreaRef = React.useRef<HTMLDivElement>(null);
  const isResizingTable = React.useRef(false);

  const handleMouseDownSidebar = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingSidebar.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingSidebar.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = moveEvent.clientX - rect.left;
      setSidebarWidthPx(Math.min(Math.max(newWidth, 200), 550));
    };

    const handleMouseUp = () => {
      isResizingSidebar.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const handleMouseDownTableSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingTable.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingTable.current || !streamAreaRef.current) return;
      const rect = streamAreaRef.current.getBoundingClientRect();
      const relativeY = moveEvent.clientY - rect.top;
      const newPercent = (relativeY / rect.height) * 100;
      setTableHeightPercent(Math.min(Math.max(newPercent, 15), 85));
    };

    const handleMouseUp = () => {
      isResizingTable.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  // New Client Connection Modal state
  const [isConnectModalOpen, setIsConnectModalOpen] = useState(false);
  const [connectUrl, setConnectUrl] = useState('wss://echo.websocket.events');
  const [customHeaders, setCustomHeaders] = useState<Array<{ key: string; value: string }>>([
    { key: '', value: '' },
  ]);

  useEffect(() => {
    initialize();
  }, [initialize]);

  const activeConnection = connections.find((c) => c.connectionId === selectedConnectionId) || null;
  const connMessages = selectedConnectionId ? messages[selectedConnectionId] || [] : [];

  // Filter connections by search and scope
  const filteredConnections = connections.filter((conn) => {
    if (streamScope === 'proxy' && conn.isClientSession) return false;
    if (streamScope === 'client' && !conn.isClientSession) return false;
    return true;
  });

  // Filter frame messages by direction and search query
  const filteredMessages = connMessages.filter((msg) => {
    if (directionFilter === 'in' && msg.direction !== 'to_client') return false;
    if (directionFilter === 'out' && msg.direction !== 'to_server') return false;
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return msg.payload.toLowerCase().includes(q) || msg.msg_type.toLowerCase().includes(q);
    }
    return true;
  });

  const selectedMsg =
    connMessages.find((m) => m.id === selectedMessageId) ||
    (filteredMessages.length > 0 ? filteredMessages[filteredMessages.length - 1] : null);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleConnectSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!connectUrl.trim()) return;

    const validHeaders = customHeaders
      .filter((h) => h.key.trim() && h.value.trim())
      .map((h) => [h.key.trim(), h.value.trim()] as [string, string]);

    const conn = await createClientConnection(
      connectUrl.trim(),
      validHeaders.length > 0 ? validHeaders : undefined,
    );

    if (conn) {
      setIsConnectModalOpen(false);
      setConnectUrl('wss://echo.websocket.events');
      setCustomHeaders([{ key: '', value: '' }]);
    }
  };

  const addHeaderRow = () => {
    setCustomHeaders([...customHeaders, { key: '', value: '' }]);
  };

  const removeHeaderRow = (idx: number) => {
    setCustomHeaders(customHeaders.filter((_, i) => i !== idx));
  };

  const updateHeaderRow = (idx: number, field: 'key' | 'value', val: string) => {
    const updated = [...customHeaders];
    updated[idx][field] = val;
    setCustomHeaders(updated);
  };

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
    <div ref={containerRef} className="h-full flex bg-background overflow-hidden text-xs font-sans">
      {/* Left Panel: Stream Navigator & Sessions */}
      <div
        style={{ width: `${sidebarWidthPx}px` }}
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
            onClick={() => setIsConnectModalOpen(true)}
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

        {/* Stream List */}
        <div className="flex-1 overflow-y-auto divide-y divide-border/60">
          {filteredConnections.length === 0 ? (
            <div className="p-6 text-center text-muted-foreground italic font-sans space-y-2">
              <MingCuteIcon name="websocket_line" size={24} className="opacity-40 mx-auto" />
              <p>No WebSocket streams available.</p>
              <p className="text-[10px]">
                Proxy WebSocket traffic through MITM or click "+ Connect" to establish a direct client connection.
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

      {/* Draggable Resizer Handle for Sidebar */}
      <div
        onMouseDown={handleMouseDownSidebar}
        className="w-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-col-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
        title="Drag to resize sidebar"
      >
        <div className="w-0.5 h-6 rounded bg-muted-foreground/40 group-hover:bg-primary" />
      </div>

      {/* Right Panel: Live Stream Console & Payload Inspector */}
      <div className="flex-1 flex flex-col overflow-hidden min-w-[300px]">
        {activeConnection ? (
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Stream Top Action & Filter Toolbar */}
            <div className="p-2.5 bg-header border-b border-border flex items-center justify-between shrink-0">
              <div className="flex items-center gap-2 font-mono overflow-hidden">
                <span
                  className={`w-2.5 h-2.5 rounded-full shrink-0 ${
                    activeConnection.status === 'connected' ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'
                  }`}
                />
                <span className="font-bold text-foreground text-xs truncate max-w-md" title={activeConnection.url}>
                  {activeConnection.url}
                </span>
                <span className="text-[10px] text-muted-foreground shrink-0">
                  ({activeConnection.isClientSession ? 'Direct Studio Client' : 'MITM Intercept Stream'})
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
                  onClick={() => clearMessages(activeConnection.connectionId)}
                  disabled={connMessages.length === 0}
                >
                  Clear Frames
                </Button>
              </div>
            </div>

            {/* Split Stream: Top Stream Table, Bottom Payload Inspector */}
            <div ref={streamAreaRef} className="flex-1 flex flex-col overflow-hidden">
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
                      {connMessages.length === 0
                        ? 'No WebSocket frames captured yet in this stream.'
                        : 'No frames match your search or direction filter.'}
                    </div>
                  ) : (
                    filteredMessages.map((msg) => {
                      const isSelected = selectedMsg?.id === msg.id;
                      const isIncoming = msg.direction === 'to_client';

                      return (
                        <div
                          key={msg.id}
                          onClick={() => selectMessage(msg.id)}
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

              {/* Draggable Resizer Bar between Table and Inspector */}
              <div
                onMouseDown={handleMouseDownTableSplit}
                className="h-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-row-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
                title="Drag to resize message table / payload inspector"
              >
                <div className="w-8 h-0.5 rounded bg-muted-foreground/40 group-hover:bg-primary" />
              </div>

              {/* Selected Frame Payload Inspector */}
              <div
                style={{ height: `${100 - tableHeightPercent}%` }}
                className="flex flex-col bg-background overflow-hidden min-h-[100px]"
              >
                <div className="p-2 bg-header border-b border-border flex items-center justify-between shrink-0 font-mono">
                  <div className="flex items-center gap-1 bg-surface border border-border rounded p-0.5">
                    {(['pretty', 'raw', 'hex', 'info'] as const).map((tab) => (
                      <button
                        key={tab}
                        onClick={() => setActiveTab(tab)}
                        className={`px-2.5 py-0.5 rounded text-xs font-sans font-medium transition-colors ${
                          activeTab === tab
                            ? 'bg-primary text-primary-foreground font-semibold'
                            : 'text-muted-foreground hover:text-foreground'
                        }`}
                      >
                        {tab === 'pretty'
                          ? 'Pretty JSON'
                          : tab === 'raw'
                          ? 'Raw Text'
                          : tab === 'hex'
                          ? 'Hex Viewer'
                          : 'Frame Info'}
                      </button>
                    ))}
                  </div>

                  {selectedMsg && (
                    <div className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        size="xs"
                        icon={copiedKey === 'payload-copy' ? 'check_line' : 'copy_line'}
                        onClick={() => handleCopy(selectedMsg.payload, 'payload-copy')}
                      >
                        {copiedKey === 'payload-copy' ? 'Copied' : 'Copy Payload'}
                      </Button>
                    </div>
                  )}
                </div>

                <div className="flex-1 p-2 overflow-hidden bg-surface">
                  {selectedMsg ? (
                    activeTab === 'hex' ? (
                      <HexViewer content={selectedMsg.payload} />
                    ) : activeTab === 'pretty' ? (
                      <CodeEditor
                        value={selectedMsg.payload}
                        language={
                          selectedMsg.payload.startsWith('{') || selectedMsg.payload.startsWith('[')
                            ? 'json'
                            : 'plaintext'
                        }
                        readOnly
                      />
                    ) : activeTab === 'raw' ? (
                      <CodeEditor value={selectedMsg.payload} language="plaintext" readOnly />
                    ) : (
                      <div className="p-3 space-y-2 font-mono text-xs">
                        <div className="grid grid-cols-2 gap-2 pb-2 border-b border-border">
                          <div>
                            <span className="text-muted-foreground text-[11px] block font-sans">Message ID:</span>
                            <span className="text-foreground">{selectedMsg.id}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground text-[11px] block font-sans">Direction:</span>
                            <span className="font-bold text-primary">{selectedMsg.direction}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground text-[11px] block font-sans">Format:</span>
                            <span className="uppercase text-foreground">{selectedMsg.msg_type}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground text-[11px] block font-sans">Length:</span>
                            <span className="text-foreground">{selectedMsg.length || selectedMsg.payload.length} Bytes</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground text-[11px] block font-sans">Timestamp:</span>
                            <span className="text-foreground">{new Date(selectedMsg.timestamp).toLocaleString()}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground text-[11px] block font-sans">Injected:</span>
                            <span className="text-foreground">{selectedMsg.is_injected ? 'Yes (User Injected)' : 'No (Wire Captured)'}</span>
                          </div>
                        </div>
                      </div>
                    )
                  ) : (
                    <div className="h-full flex items-center justify-center text-muted-foreground italic font-sans">
                      Select a frame row above to inspect its contents
                    </div>
                  )}
                </div>
              </div>
            </div>

            {/* Bottom Dock: Interactive Frame Injector */}
            <div className="p-3 bg-header border-t border-border flex flex-col gap-2 shrink-0">
              <div className="flex items-center justify-between font-mono">
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
                    disabled={activeConnection.status !== 'connected' || !messageBuilderContent.trim()}
                  >
                    Send to Server
                  </Button>
                  <Button
                    variant="secondary"
                    size="xs"
                    icon="arrow_down_line"
                    onClick={() => sendMessage('to_client')}
                    disabled={activeConnection.status !== 'connected' || !messageBuilderContent.trim()}
                  >
                    Inject to Client
                  </Button>
                </div>
              </div>

              <textarea
                rows={2}
                value={messageBuilderContent}
                onChange={(e) => setMessageBuilderContent(e.target.value)}
                placeholder="Type raw text, JSON payload, or hex string for binary frames..."
                className="w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
              />
            </div>
          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center text-muted-foreground italic text-xs p-6 gap-3">
            <MingCuteIcon name="websocket_line" size={48} className="opacity-30" />
            <div className="text-center space-y-1">
              <p className="font-semibold text-foreground text-sm">No Active WebSocket Stream Selected</p>
              <p>Select a stream from the left panel or start a new client connection.</p>
            </div>
            <Button
              variant="primary"
              size="sm"
              icon="add_line"
              onClick={() => setIsConnectModalOpen(true)}
            >
              Establish New Connection
            </Button>
          </div>
        )}
      </div>

      {/* New Client Connection Modal Dialog */}
      <Dialog
        isOpen={isConnectModalOpen}
        onClose={() => setIsConnectModalOpen(false)}
        title="Connect WebSocket Client"
        footer={
          <div className="flex items-center justify-end gap-2">
            <Button variant="secondary" onClick={() => setIsConnectModalOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleConnectSubmit}
              disabled={isConnecting || !connectUrl.trim()}
            >
              {isConnecting ? 'Connecting...' : 'Connect to Server'}
            </Button>
          </div>
        }
      >
        <form onSubmit={handleConnectSubmit} className="space-y-4 font-sans text-xs">
          <div>
            <label className="block text-foreground font-medium mb-1">WebSocket URL (ws:// or wss://)</label>
            <Input
              value={connectUrl}
              onChange={(e) => setConnectUrl(e.target.value)}
              placeholder="wss://echo.websocket.events"
              className="font-mono"
              required
            />
            <p className="text-[11px] text-muted-foreground mt-1">
              Connects directly to the specified endpoint with live frame bidirectional streaming.
            </p>
          </div>

          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="block text-foreground font-medium">Custom Handshake Headers (Optional)</label>
              <button
                type="button"
                onClick={addHeaderRow}
                className="text-primary hover:underline font-bold text-[11px]"
              >
                + Add Header
              </button>
            </div>

            <div className="space-y-2 max-h-36 overflow-y-auto">
              {customHeaders.map((header, idx) => (
                <div key={idx} className="flex items-center gap-2">
                  <Input
                    placeholder="Header Name (e.g. Authorization)"
                    value={header.key}
                    onChange={(e) => updateHeaderRow(idx, 'key', e.target.value)}
                    className="flex-1 font-mono text-xs"
                  />
                  <Input
                    placeholder="Header Value"
                    value={header.value}
                    onChange={(e) => updateHeaderRow(idx, 'value', e.target.value)}
                    className="flex-1 font-mono text-xs"
                  />
                  {customHeaders.length > 1 && (
                    <button
                      type="button"
                      onClick={() => removeHeaderRow(idx)}
                      className="text-muted-foreground hover:text-rose-500 p-1"
                    >
                      <MingCuteIcon name="delete_2_line" size={14} />
                    </button>
                  )}
                </div>
              ))}
            </div>
          </div>
        </form>
      </Dialog>
    </div>
  );
};
