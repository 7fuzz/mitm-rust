import { create } from 'zustand';
import type { WebhookEndpoint, WebhookDelivery, WebhookListenerConfig, WebhookReplayResult } from '../types';
import {
  getWebhookEndpoints,
  createWebhookEndpoint,
  deleteWebhookEndpoint,
  getWebhookDeliveries,
  clearWebhookDeliveries,
  getWebhookListenerStatus,
  startWebhookListener,
  setWebhookPort,
  stopWebhookListener,
  calculateWebhookSignature,
  replayWebhookDelivery,
  isTauriAvailable,
} from '../services/tauri/ipc';
import { listenWebhookCaptured } from '../services/tauri/events';

interface WebhookState {
  listenerConfig: WebhookListenerConfig;
  endpoints: WebhookEndpoint[];
  deliveries: WebhookDelivery[];
  selectedDeliveryId: string | null;
  isLoading: boolean;
  isInitialized: boolean;

  // Replay
  isReplaying: boolean;
  replayResult: WebhookReplayResult | null;

  // HMAC calculator
  hmacSecret: string;
  hmacBody: string;
  hmacProvider: 'github' | 'stripe' | 'shopify' | 'raw_sha256' | 'raw_sha1' | 'raw_sha512';
  computedSignature: { header_name: string; header_value: string } | null;

  // Actions
  initialize: () => Promise<void>;
  fetchStatus: () => Promise<void>;
  toggleServer: () => Promise<void>;
  savePort: (port: number) => Promise<void>;

  fetchEndpoints: () => Promise<void>;
  addEndpoint: (endpoint: Partial<WebhookEndpoint>) => Promise<WebhookEndpoint | undefined>;
  deleteEndpoint: (id: string) => Promise<void>;

  fetchDeliveries: () => Promise<void>;
  clearDeliveries: () => Promise<void>;
  selectDelivery: (id: string | null) => void;
  replayDelivery: (id: string, targetUrl: string) => Promise<WebhookReplayResult | null>;

  setHmacSecret: (s: string) => void;
  setHmacBody: (b: string) => void;
  setHmacProvider: (p: 'github' | 'stripe' | 'shopify' | 'raw_sha256' | 'raw_sha1' | 'raw_sha512') => void;
  calculateHmac: () => Promise<void>;
}

export const useWebhookStore = create<WebhookState>((set, get) => ({
  listenerConfig: { port: 9000, is_running: false },
  endpoints: [],
  deliveries: [],
  selectedDeliveryId: null,
  isLoading: false,
  isInitialized: false,

  isReplaying: false,
  replayResult: null,

  hmacSecret: 'whsec_secret_key_123',
  hmacBody: JSON.stringify({ event: 'ping', payload: 'test' }, null, 2),
  hmacProvider: 'github',
  computedSignature: null,

  initialize: async () => {
    if (get().isInitialized) return;
    set({ isInitialized: true, isLoading: true });

    try {
      if (isTauriAvailable()) {
        await listenWebhookCaptured((payload: any) => {
          const delivery: WebhookDelivery = payload.delivery || payload;
          const hitCount: number = payload.endpointHitCount;

          set((state) => {
            const updatedDeliveries = [delivery, ...state.deliveries.filter((d) => d.id !== delivery.id)];
            const updatedEndpoints = state.endpoints.map((ep) => {
              if (ep.id === delivery.endpointId || ep.path === delivery.endpointPath) {
                return { ...ep, hitCount: hitCount ?? (ep.hitCount + 1) };
              }
              return ep;
            });

            return {
              deliveries: updatedDeliveries,
              endpoints: updatedEndpoints,
              selectedDeliveryId: state.selectedDeliveryId || delivery.id,
            };
          });
        });
      }

      await Promise.all([
        get().fetchStatus(),
        get().fetchEndpoints(),
        get().fetchDeliveries(),
      ]);
    } catch (err) {
      console.error('Failed to initialize Webhook store:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  fetchStatus: async () => {
    try {
      const cfg = await getWebhookListenerStatus();
      set({ listenerConfig: cfg });
    } catch (err) {
      console.error('Failed to fetch webhook status:', err);
    }
  },

  toggleServer: async () => {
    const { listenerConfig } = get();
    try {
      if (listenerConfig.is_running) {
        await stopWebhookListener();
        set({ listenerConfig: { ...listenerConfig, is_running: false } });
      } else {
        await startWebhookListener(listenerConfig.port);
        set({ listenerConfig: { ...listenerConfig, is_running: true } });
      }
    } catch (err) {
      console.error('Failed to toggle webhook listener:', err);
      throw err;
    }
  },

  savePort: async (port) => {
    await setWebhookPort(port);
    set((state) => ({ listenerConfig: { ...state.listenerConfig, port } }));
  },

  fetchEndpoints: async () => {
    try {
      const eps = await getWebhookEndpoints();
      set({ endpoints: eps || [] });
    } catch (err) {
      console.error('Failed to fetch webhook endpoints:', err);
    }
  },

  addEndpoint: async (ep) => {
    try {
      const created = await createWebhookEndpoint(ep);
      set((state) => ({
        endpoints: [...state.endpoints.filter((e) => e.id !== created.id), created],
      }));
      return created;
    } catch (err) {
      console.error('Failed to add webhook endpoint:', err);
      return undefined;
    }
  },

  deleteEndpoint: async (id) => {
    set((state) => ({
      endpoints: state.endpoints.filter((e) => e.id !== id),
      deliveries: state.deliveries.filter((d) => d.endpointId !== id),
    }));
    try {
      await deleteWebhookEndpoint(id);
    } catch (err) {
      console.error('Failed to delete endpoint:', err);
    }
  },

  fetchDeliveries: async () => {
    try {
      const dels = await getWebhookDeliveries(500);
      set((state) => ({
        deliveries: dels || [],
        selectedDeliveryId: state.selectedDeliveryId || (dels && dels.length > 0 ? dels[0].id : null),
      }));
    } catch (err) {
      console.error('Failed to fetch deliveries:', err);
    }
  },

  clearDeliveries: async () => {
    // Keep the list when the backend delete fails, so it never looks cleared when it isn't
    try {
      await clearWebhookDeliveries();
    } catch (err) {
      console.error('Failed to clear deliveries:', err);
      throw err;
    }
    set({ deliveries: [], selectedDeliveryId: null, replayResult: null });
  },

  selectDelivery: (id) => set({ selectedDeliveryId: id, replayResult: null }),

  replayDelivery: async (id, targetUrl) => {
    set({ isReplaying: true, replayResult: null });
    try {
      const result = await replayWebhookDelivery(id, targetUrl);
      set({ replayResult: result });
      return result;
    } catch (err) {
      console.error(`Failed to replay webhook delivery ${id}:`, err);
      const errorResult: WebhookReplayResult = {
        success: false,
        statusCode: 0,
        responseBody: String(err),
        durationMs: 0,
      };
      set({ replayResult: errorResult });
      return errorResult;
    } finally {
      set({ isReplaying: false });
    }
  },

  setHmacSecret: (hmacSecret) => set({ hmacSecret }),
  setHmacBody: (hmacBody) => set({ hmacBody }),
  setHmacProvider: (hmacProvider) => set({ hmacProvider }),

  calculateHmac: async () => {
    const { hmacSecret, hmacBody, hmacProvider } = get();
    try {
      const res = await calculateWebhookSignature(hmacSecret, hmacBody, hmacProvider);
      set({ computedSignature: res });
    } catch (err) {
      console.error('Failed to calculate HMAC signature:', err);
    }
  },
}));
