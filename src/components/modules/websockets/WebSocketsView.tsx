import React, { useState, useEffect, useRef } from 'react';
import { useWebSocketStore } from '../../../stores/useWebSocketStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button } from '../../common/ui';
import { WebSocketConnectionsSidebar } from './components/WebSocketConnectionsSidebar';
import { WebSocketMessageList } from './components/WebSocketMessageList';
import { WebSocketMessageInspector } from './components/WebSocketMessageInspector';
import { WebSocketMessageComposer } from './components/WebSocketMessageComposer';
import { NewConnectionModal } from './components/NewConnectionModal';

export const WebSocketsView: React.FC = () => {
  const {
    initialize,
    connections,
    selectedConnectionId,
    messages,
    directionFilter,
    searchQuery,
    selectedMessageId,
    selectMessage,
  } = useWebSocketStore();

  // Resizable sidebar and stream split states
  const [sidebarWidthPx, setSidebarWidthPx] = useState<number>(288);
  const containerRef = useRef<HTMLDivElement>(null);
  const isResizingSidebar = useRef(false);

  const [tableHeightPercent, setTableHeightPercent] = useState<number>(50);
  const streamAreaRef = useRef<HTMLDivElement>(null);
  const isResizingTable = useRef(false);

  // Resizable Frame Injector Dock
  const [injectorHeightPx, setInjectorHeightPx] = useState<number>(140);
  const mainConsoleRef = useRef<HTMLDivElement>(null);
  const isResizingInjector = useRef(false);

  // New Client Connection Modal state
  const [isConnectModalOpen, setIsConnectModalOpen] = useState(false);

  useEffect(() => {
    initialize();
  }, [initialize]);

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

  const handleMouseDownInjectorSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingInjector.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingInjector.current || !mainConsoleRef.current) return;
      const rect = mainConsoleRef.current.getBoundingClientRect();
      const newHeight = rect.bottom - moveEvent.clientY;
      setInjectorHeightPx(Math.min(Math.max(newHeight, 80), 500));
    };

    const handleMouseUp = () => {
      isResizingInjector.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const activeConnection = connections.find((c) => c.connectionId === selectedConnectionId) || null;
  const connMessages = selectedConnectionId ? messages[selectedConnectionId] || [] : [];

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

  return (
    <div ref={containerRef} className="h-full flex bg-background overflow-hidden text-xs font-sans">
      {/* Left Panel: Stream Navigator & Sessions */}
      <WebSocketConnectionsSidebar
        widthPx={sidebarWidthPx}
        onOpenConnectModal={() => setIsConnectModalOpen(true)}
      />

      {/* Draggable Resizer Handle for Sidebar */}
      <div
        onMouseDown={handleMouseDownSidebar}
        className="w-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-col-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
        title="Drag to resize sidebar"
      >
        <div className="w-0.5 h-6 rounded bg-muted-foreground/40 group-hover:bg-primary" />
      </div>

      {/* Right Panel: Live Stream Console & Payload Inspector */}
      <div ref={mainConsoleRef} className="flex-1 flex flex-col overflow-hidden min-w-[300px]">
        {activeConnection ? (
          <div className="flex-1 flex flex-col overflow-hidden">
            <WebSocketMessageList
              connection={activeConnection}
              tableHeightPercent={tableHeightPercent}
              filteredMessages={filteredMessages}
              totalMessageCount={connMessages.length}
              selectedMessageId={selectedMsg?.id || null}
              onSelectMessage={selectMessage}
            />

            {/* Draggable Resizer Bar between Table and Inspector */}
            <div
              onMouseDown={handleMouseDownTableSplit}
              className="h-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-row-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
              title="Drag to resize message table / payload inspector"
            >
              <div className="w-8 h-0.5 rounded bg-muted-foreground/40 group-hover:bg-primary" />
            </div>

            {/* Selected Frame Payload Inspector */}
            <WebSocketMessageInspector
              selectedMsg={selectedMsg}
              heightPercent={100 - tableHeightPercent}
            />

            {/* Draggable Resizer Bar between Stream Inspector and Frame Injector */}
            <div
              onMouseDown={handleMouseDownInjectorSplit}
              className="h-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-row-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
              title="Drag to resize frame injector dock"
            >
              <div className="w-8 h-0.5 rounded bg-muted-foreground/40 group-hover:bg-primary" />
            </div>

            {/* Bottom Dock: Interactive Frame Injector */}
            <WebSocketMessageComposer
              connection={activeConnection}
              heightPx={injectorHeightPx}
            />
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
      <NewConnectionModal
        isOpen={isConnectModalOpen}
        onClose={() => setIsConnectModalOpen(false)}
      />
    </div>
  );
};
