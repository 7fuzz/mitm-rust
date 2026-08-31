import { create } from 'zustand';
import type { WebSocketConn, WebSocketMessage, WebSocketMessageCapturedPayload, WebSocketConnectionPayload } from '../types';
import {
  getWsConnections,
  getWsMessages,
  connectWsClient,
  disconnectWsClient,
  sendWsMessage,
  clearWsMessages,
  deleteWsConnection,
  isTauriAvailable,
} from '../services/tauri/ipc';
import { listenWsMessageCaptured, listenWsConnectionEvent } from '../services/tauri/events';

interface WebSocketState {
  isInitialized: boolean;
  isLoading: boolean;
  connections: WebSocketConn[];
  selectedConnectionId: string | null;
  messages: Record<string, WebSocketMessage[]>;
  directionFilter: 'all' | 'in' | 'out';
  searchQuery: string;
  messageBuilderType: 'text' | 'json' | 'binary';
  messageBuilderContent: string;
  selectedMessageId: string | null;
  isConnecting: boolean;

  initialize: () => Promise<void>;
  fetchConnections: () => Promise<void>;
  selectConnection: (id: string | null) => Promise<void>;
  createClientConnection: (url: string, headers?: [string, string][]) => Promise<WebSocketConn | undefined>;
  disconnectConnection: (id: string) => Promise<void>;
  deleteConnection: (id: string) => Promise<void>;
  sendMessage: (direction: 'to_server' | 'to_client') => Promise<void>;
  clearMessages: (connId?: string) => Promise<void>;
  setDirectionFilter: (filter: 'all' | 'in' | 'out') => void;
  setSearchQuery: (query: string) => void;
  setMessageBuilderType: (type: 'text' | 'json' | 'binary') => void;
  setMessageBuilderContent: (content: string) => void;
  selectMessage: (id: string | null) => void;
}

export const useWebSocketStore = create<WebSocketState>((set, get) => ({
  isInitialized: false,
  isLoading: false,
  connections: [],
  selectedConnectionId: null,
  messages: {},
  directionFilter: 'all',
  searchQuery: '',
  messageBuilderType: 'json',
  messageBuilderContent: JSON.stringify({ event: 'ping', timestamp: Date.now() }, null, 2),
  selectedMessageId: null,
  isConnecting: false,

  initialize: async () => {
    if (get().isInitialized) return;
    set({ isInitialized: true, isLoading: true });

    try {
      if (isTauriAvailable()) {
        await listenWsMessageCaptured((payload: WebSocketMessageCapturedPayload) => {
          const msg = payload.message;
          const connId = msg.connection_id;

          set((state) => {
            const currentMsgs = state.messages[connId] || [];
            const updatedMsgs = [...currentMsgs.filter((m) => m.id !== msg.id), msg];

            const updatedConns = state.connections.map((c) => {
              if (c.connectionId === connId) {
                return { ...c, messageCount: payload.messageCount ?? ((c.messageCount || 0) + 1) };
              }
              return c;
            });

            return {
              messages: { ...state.messages, [connId]: updatedMsgs },
              connections: updatedConns,
              selectedMessageId: state.selectedConnectionId === connId ? (state.selectedMessageId || msg.id) : state.selectedMessageId,
            };
          });
        });

        await listenWsConnectionEvent((payload: WebSocketConnectionPayload) => {
          const { connection, eventType } = payload;
          set((state) => {
            if (eventType === 'opened') {
              const exists = state.connections.some((c) => c.connectionId === connection.connectionId);
              const updated = exists
                ? state.connections.map((c) => (c.connectionId === connection.connectionId ? connection : c))
                : [connection, ...state.connections];
              return {
                connections: updated,
                selectedConnectionId: state.selectedConnectionId || connection.connectionId,
              };
            } else {
              const updated = state.connections.map((c) =>
                c.connectionId === connection.connectionId ? { ...c, status: 'disconnected' as const, closedAt: connection.closedAt } : c
              );
              return { connections: updated };
            }
          });
        });
      }

      await get().fetchConnections();
    } catch (err) {
      console.error('Failed to initialize WebSocket store:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  fetchConnections: async () => {
    try {
      const conns = await getWsConnections();
      set({
        connections: conns || [],
        selectedConnectionId: get().selectedConnectionId || (conns && conns.length > 0 ? conns[0].connectionId : null),
      });

      const selected = get().selectedConnectionId || (conns && conns.length > 0 ? conns[0].connectionId : null);
      if (selected) {
        await get().selectConnection(selected);
      }
    } catch (err) {
      console.error('Failed to fetch WS connections:', err);
    }
  },

  selectConnection: async (id) => {
    set({ selectedConnectionId: id, selectedMessageId: null });
    if (id) {
      try {
        const msgs = await getWsMessages(id);
        set((state) => ({
          messages: { ...state.messages, [id]: msgs || [] },
          selectedMessageId: msgs && msgs.length > 0 ? msgs[msgs.length - 1].id : null,
        }));
      } catch (err) {
        console.error('Failed to fetch messages for WS connection:', id, err);
      }
    }
  },

  createClientConnection: async (url, headers) => {
    set({ isConnecting: true });
    try {
      const conn = await connectWsClient(url, headers);
      set((state) => ({
        connections: [conn, ...state.connections.filter((c) => c.connectionId !== conn.connectionId)],
        selectedConnectionId: conn.connectionId,
        selectedMessageId: null,
      }));
      return conn;
    } catch (err) {
      console.error('Failed to create WebSocket client connection:', err);
      return undefined;
    } finally {
      set({ isConnecting: false });
    }
  },

  disconnectConnection: async (id) => {
    try {
      await disconnectWsClient(id);
      set((state) => ({
        connections: state.connections.map((c) =>
          c.connectionId === id ? { ...c, status: 'disconnected' as const, closedAt: Date.now() } : c
        ),
      }));
    } catch (err) {
      console.error('Failed to disconnect WS session:', id, err);
    }
  },

  deleteConnection: async (id) => {
    try {
      await deleteWsConnection(id);
      set((state) => {
        const remaining = state.connections.filter((c) => c.connectionId !== id);
        const nextSelected = state.selectedConnectionId === id ? (remaining.length > 0 ? remaining[0].connectionId : null) : state.selectedConnectionId;
        const newMsgs = { ...state.messages };
        delete newMsgs[id];
        return {
          connections: remaining,
          selectedConnectionId: nextSelected,
          messages: newMsgs,
          selectedMessageId: null,
        };
      });
    } catch (err) {
      console.error('Failed to delete WS connection:', id, err);
    }
  },

  sendMessage: async (direction) => {
    const { selectedConnectionId, messageBuilderContent, messageBuilderType } = get();
    if (!selectedConnectionId || !messageBuilderContent.trim()) return;

    try {
      const sentMsg = await sendWsMessage(
        selectedConnectionId,
        direction,
        messageBuilderType,
        messageBuilderContent,
      );

      set((state) => {
        const current = state.messages[selectedConnectionId] || [];
        return {
          messages: {
            ...state.messages,
            [selectedConnectionId]: [...current, sentMsg],
          },
          selectedMessageId: sentMsg.id,
        };
      });
    } catch (err) {
      console.error('Failed to send WebSocket message:', err);
    }
  },

  clearMessages: async (connId) => {
    try {
      await clearWsMessages(connId);
      set((state) => {
        if (connId) {
          return {
            messages: { ...state.messages, [connId]: [] },
            selectedMessageId: null,
            connections: state.connections.map((c) => (c.connectionId === connId ? { ...c, messageCount: 0 } : c)),
          };
        } else {
          return {
            messages: {},
            selectedMessageId: null,
            connections: state.connections.map((c) => ({ ...c, messageCount: 0 })),
          };
        }
      });
    } catch (err) {
      console.error('Failed to clear WS messages:', err);
    }
  },

  setDirectionFilter: (filter) => set({ directionFilter: filter }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setMessageBuilderType: (type) => set({ messageBuilderType: type }),
  setMessageBuilderContent: (content) => set({ messageBuilderContent: content }),
  selectMessage: (id) => set({ selectedMessageId: id }),
}));
