import { create } from 'zustand';
import type { WebhookEndpoint, WebhookDelivery, WebhookListenerConfig } from '../types';
import { getWebhookEndpoints, createWebhookEndpoint, deleteWebhookEndpoint, getWebhookDeliveries, clearWebhookDeliveries, getWebhookListenerStatus, startWebhookListener, stopWebhookListener, calculateWebhookSignature } from '../services/tauri/ipc';

const SAMPLE_ENDPOINTS: WebhookEndpoint[] = [
  { id: 'ep-1', path: '/api/v1/github-webhook', name: 'GitHub Repo Push Event', secretKey: 'whsec_github_secret_9921', createdAt: Date.now() - 86400000, hitCount: 14 },
  { id: 'ep-2', path: '/v1/stripe/invoice', name: 'Stripe Invoice Payment Success', secretKey: 'whsec_stripe_key_882', createdAt: Date.now() - 43200000, hitCount: 5 },
];

const SAMPLE_DELIVERIES: WebhookDelivery[] = [
  {
    id: 'del-1',
    endpointId: 'ep-1',
    endpointPath: '/api/v1/github-webhook',
    timestamp: Date.now() - 15000,
    headers: [
      { key: 'Host', value: 'localhost:9000' },
      { key: 'User-Agent', value: 'GitHub-Hookshot/384910' },
      { key: 'X-GitHub-Event', value: 'push' },
      { key: 'X-Hub-Signature-256', value: 'sha256=d3b07384d113edec49eaa6238ad5ff00' },
    ],
    payload: JSON.stringify({ ref: 'refs/heads/main', repository: { full_name: 'mitm-rust/server' }, commits: [{ id: 'a982f1', message: 'feat: add tauri v2 proxy bindings' }] }, null, 2),
    signatureStatus: 'valid',
    computedHmac: 'sha256=d3b07384d113edec49eaa6238ad5ff00',
    providedHmac: 'sha256=d3b07384d113edec49eaa6238ad5ff00',
  },
  {
    id: 'del-2',
    endpointId: 'ep-2',
    endpointPath: '/v1/stripe/invoice',
    timestamp: Date.now() - 45000,
    headers: [
      { key: 'Host', value: 'localhost:9000' },
      { key: 'Stripe-Signature', value: 't=1672531199,v1=bad_signature_string_value' },
    ],
    payload: JSON.stringify({ id: 'in_1M492049281', object: 'invoice', amount_paid: 2500, paid: true }, null, 2),
    signatureStatus: 'invalid',
    computedHmac: 't=1672531199,v1=f82a1738c1...',
    providedHmac: 't=1672531199,v1=bad_signature_string_value',
  },
];

interface WebhookState {
  listenerConfig: WebhookListenerConfig;
  endpoints: WebhookEndpoint[];
  deliveries: WebhookDelivery[];
  selectedDeliveryId: string | null;

  // HMAC calculator
  hmacSecret: string;
  hmacBody: string;
  hmacProvider: 'github' | 'stripe' | 'raw_sha256' | 'raw_sha1' | 'raw_md5';
  computedSignature: { header_name: string; header_value: string } | null;

  // Actions
  fetchStatus: () => Promise<void>;
  toggleServer: () => Promise<void>;
  setPort: (port: number) => void;

  fetchEndpoints: () => Promise<void>;
  addEndpoint: (endpoint: Partial<WebhookEndpoint>) => Promise<void>;
  deleteEndpoint: (id: string) => Promise<void>;

  fetchDeliveries: () => Promise<void>;
  clearDeliveries: () => Promise<void>;
  selectDelivery: (id: string | null) => void;
  replayDelivery: (id: string, targetUrl: string) => Promise<void>;

  setHmacSecret: (s: string) => void;
  setHmacBody: (b: string) => void;
  setHmacProvider: (p: 'github' | 'stripe' | 'raw_sha256' | 'raw_sha1' | 'raw_md5') => void;
  calculateHmac: () => Promise<void>;
}

export const useWebhookStore = create<WebhookState>((set, get) => ({
  listenerConfig: { port: 9000, is_running: true },
  endpoints: SAMPLE_ENDPOINTS,
  deliveries: SAMPLE_DELIVERIES,
  selectedDeliveryId: 'del-1',

  hmacSecret: 'whsec_secret_key_123',
  hmacBody: JSON.stringify({ event: 'ping', payload: 'test' }, null, 2),
  hmacProvider: 'github',
  computedSignature: null,

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
    }
  },

  setPort: (port) => {
    set((state) => ({ listenerConfig: { ...state.listenerConfig, port } }));
  },

  fetchEndpoints: async () => {
    try {
      const eps = await getWebhookEndpoints();
      if (eps && eps.length > 0) set({ endpoints: eps });
    } catch (err) {
      console.error('Failed to fetch webhook endpoints:', err);
    }
  },

  addEndpoint: async (ep) => {
    try {
      const created = await createWebhookEndpoint(ep);
      set((state) => ({ endpoints: [...state.endpoints, created] }));
    } catch (err) {
      console.error('Failed to add webhook endpoint:', err);
    }
  },

  deleteEndpoint: async (id) => {
    set((state) => ({ endpoints: state.endpoints.filter((e) => e.id !== id) }));
    try {
      await deleteWebhookEndpoint(id);
    } catch (err) {
      console.error('Failed to delete endpoint:', err);
    }
  },

  fetchDeliveries: async () => {
    try {
      const dels = await getWebhookDeliveries();
      if (dels && dels.length > 0) set({ deliveries: dels });
    } catch (err) {
      console.error('Failed to fetch deliveries:', err);
    }
  },

  clearDeliveries: async () => {
    set({ deliveries: [], selectedDeliveryId: null });
    try {
      await clearWebhookDeliveries();
    } catch (err) {
      console.error('Failed to clear deliveries:', err);
    }
  },

  selectDelivery: (id) => set({ selectedDeliveryId: id }),

  replayDelivery: async (id, targetUrl) => {
    console.log(`Replaying webhook delivery ${id} to target ${targetUrl}`);
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
