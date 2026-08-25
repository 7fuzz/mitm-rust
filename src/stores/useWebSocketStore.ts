import { create } from 'zustand';
import type { WebSocketConn, WebSocketMessage } from '../types';
import { getWebsocketMessages, sendWebsocketMessage } from '../services/tauri/ipc';

const SAMPLE_CONNECTIONS: WebSocketConn[] = [
  { connectionId: 'ws-conn-1', url: 'wss://stream.binance.com/ws/btcusdt@trade', status: 'connected', handshakeTime: Date.now() - 300000, protocol: 'websocket' },
  { connectionId: 'ws-conn-2', url: 'wss://echo.websocket.events', status: 'disconnected', handshakeTime: Date.now() - 600000 },
];

const SAMPLE_MESSAGES: Record<string, WebSocketMessage[]> = {
  'ws-conn-1': [
    { id: 'ws-msg-1', connection_id: 'ws-conn-1', direction: 'to_client', msg_type: 'json', payload: JSON.stringify({ e: 'trade', E: 1672531199000, s: 'BTCUSDT', p: '94210.50', q: '0.0421', b: 884192, a: 884193, T: 1672531198998, m: true }, null, 2), timestamp: Date.now() - 20000 },
    { id: 'ws-msg-2', connection_id: 'ws-conn-1', direction: 'to_server', msg_type: 'json', payload: JSON.stringify({ method: 'SUBSCRIBE', params: ['btcusdt@depth'], id: 1 }, null, 2), timestamp: Date.now() - 25000 },
    { id: 'ws-msg-3', connection_id: 'ws-conn-1', direction: 'to_client', msg_type: 'json', payload: JSON.stringify({ result: null, id: 1 }, null, 2), timestamp: Date.now() - 24800 },
  ],
};

interface WebSocketState {
  connections: WebSocketConn[];
  selectedConnectionId: string | null;
  messages: Record<string, WebSocketMessage[]>;
  directionFilter: 'all' | 'in' | 'out';
  messageBuilderType: 'text' | 'json' | 'binary';
  messageBuilderContent: string;
  selectedMessageId: string | null;

  selectConnection: (id: string | null) => Promise<void>;
  addMessage: (msg: WebSocketMessage) => void;
  setDirectionFilter: (filter: 'all' | 'in' | 'out') => void;
  setMessageBuilderType: (type: 'text' | 'json' | 'binary') => void;
  setMessageBuilderContent: (content: string) => void;
  selectMessage: (id: string | null) => void;
  sendLiveMessage: (direction: 'to_server' | 'to_client') => Promise<void>;
}

export const useWebSocketStore = create<WebSocketState>((set, get) => ({
  connections: SAMPLE_CONNECTIONS,
  selectedConnectionId: 'ws-conn-1',
  messages: SAMPLE_MESSAGES,
  directionFilter: 'all',
  messageBuilderType: 'json',
  messageBuilderContent: JSON.stringify({ event: 'ping', timestamp: Date.now() }, null, 2),
  selectedMessageId: 'ws-msg-1',

  selectConnection: async (id) => {
    set({ selectedConnectionId: id, selectedMessageId: null });
    if (id && !get().messages[id]) {
      try {
        const msgs = await getWebsocketMessages(id);
        set((state) => ({
          messages: { ...state.messages, [id]: msgs },
        }));
      } catch (err) {
        console.error('Failed to fetch WS messages:', err);
      }
    }
  },

  addMessage: (msg) => {
    set((state) => {
      const connId = msg.connection_id;
      const current = state.messages[connId] || [];
      return {
        messages: {
          ...state.messages,
          [connId]: [...current, msg],
        },
      };
    });
  },

  setDirectionFilter: (directionFilter) => set({ directionFilter }),
  setMessageBuilderType: (messageBuilderType) => set({ messageBuilderType }),
  setMessageBuilderContent: (messageBuilderContent) => set({ messageBuilderContent }),
  selectMessage: (selectedMessageId) => set({ selectedMessageId }),

  sendLiveMessage: async (direction) => {
    const { selectedConnectionId, messageBuilderContent, messageBuilderType } = get();
    if (!selectedConnectionId || !messageBuilderContent.trim()) return;

    const newMsg: WebSocketMessage = {
      id: 'ws-msg-' + Date.now(),
      connection_id: selectedConnectionId,
      direction,
      msg_type: messageBuilderType,
      payload: messageBuilderContent,
      timestamp: Date.now(),
    };

    get().addMessage(newMsg);

    try {
      await sendWebsocketMessage(selectedConnectionId, direction, messageBuilderContent);
    } catch (err) {
      console.warn('Sent WS message via local mock pipeline');
    }
  },
}));
