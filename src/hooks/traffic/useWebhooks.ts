import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  WebhookEndpoint,
  WebhookDelivery,
  WebhookTriggerRequest,
  WebhookForwardResult,
  WebhookListenerConfig
} from '@/types/webhook';

export function useWebhooks() {
  const [endpoints, setEndpoints] = useState<WebhookEndpoint[]>([]);
  const [deliveries, setDeliveries] = useState<WebhookDelivery[]>([]);
  const [listenerConfig, setListenerConfig] = useState<WebhookListenerConfig>({
    port: 9000,
    isRunning: true,
  });
  const [isLoading, setIsLoading] = useState<boolean>(true);

  // Load initial data
  const loadData = useCallback(async () => {
    try {
      setIsLoading(true);
      const [eps, dels, config] = await Promise.all([
        invoke<WebhookEndpoint[]>('get_webhook_endpoints'),
        invoke<WebhookDelivery[]>('get_webhook_deliveries', { limit: 100 }),
        invoke<WebhookListenerConfig>('get_webhook_listener_status'),
      ]);
      setEndpoints(eps);
      setDeliveries(dels);
      setListenerConfig(config);
    } catch (err) {
      console.error('Failed to load webhook data:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();

    // Listen for real-time captured webhooks
    const unlistenPromise = listen<WebhookDelivery>('webhook_captured', (event) => {
      const newDelivery = event.payload;
      setDeliveries((prev) => [newDelivery, ...prev.slice(0, 99)]);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [loadData]);

  // Endpoints CRUD
  const createEndpoint = async (endpoint: Partial<WebhookEndpoint>) => {
    const newEp: WebhookEndpoint = {
      id: '',
      name: endpoint.name || 'New Webhook Endpoint',
      pathSlug: endpoint.pathSlug || `/hook/${Math.random().toString(36).substring(2, 8)}`,
      mockStatus: endpoint.mockStatus || 200,
      mockHeaders: endpoint.mockHeaders || '{"Content-Type": "application/json"}',
      mockBody: endpoint.mockBody || '{"status": "received"}',
      autoForwardUrl: endpoint.autoForwardUrl || null,
      isActive: endpoint.isActive ?? true,
      createdAt: Date.now(),
    };
    const created = await invoke<WebhookEndpoint>('create_webhook_endpoint', { endpoint: newEp });
    setEndpoints((prev) => [created, ...prev]);
    return created;
  };

  const updateEndpoint = async (endpoint: WebhookEndpoint) => {
    await invoke('update_webhook_endpoint', { endpoint });
    setEndpoints((prev) => prev.map((e) => (e.id === endpoint.id ? endpoint : e)));
  };

  const deleteEndpoint = async (id: string) => {
    await invoke('delete_webhook_endpoint', { id });
    setEndpoints((prev) => prev.filter((e) => e.id !== id));
  };

  // Deliveries CRUD & Operations
  const clearDeliveries = async () => {
    await invoke('clear_webhook_deliveries');
    setDeliveries([]);
  };

  const deleteDelivery = async (id: string) => {
    await invoke('delete_webhook_delivery', { id });
    setDeliveries((prev) => prev.filter((d) => d.id !== id));
  };

  const forwardDelivery = async (deliveryId: string, targetUrl: string): Promise<WebhookForwardResult> => {
    const result = await invoke<WebhookForwardResult>('forward_webhook_delivery', {
      deliveryId,
      targetUrl,
    });
    // Refresh delivery list to show updated forwarding status
    const updatedDeliveries = await invoke<WebhookDelivery[]>('get_webhook_deliveries', { limit: 100 });
    setDeliveries(updatedDeliveries);
    return result;
  };

  const triggerWebhook = async (req: WebhookTriggerRequest): Promise<WebhookForwardResult> => {
    return await invoke<WebhookForwardResult>('trigger_webhook_request', { req });
  };

  const startListener = async (port: number) => {
    await invoke('start_webhook_listener', { port });
    setListenerConfig({ port, isRunning: true });
  };

  const stopListener = async () => {
    await invoke('stop_webhook_listener');
    setListenerConfig((prev) => ({ ...prev, isRunning: false }));
  };

  const calculateSignature = async (secret: string, body: string, provider: string) => {
    return await invoke<{ header_name: string; header_value: string }>('calculate_webhook_signature', {
      secret,
      body,
      provider,
    });
  };

  return {
    endpoints,
    deliveries,
    listenerConfig,
    isLoading,
    refresh: loadData,
    createEndpoint,
    updateEndpoint,
    deleteEndpoint,
    clearDeliveries,
    deleteDelivery,
    forwardDelivery,
    triggerWebhook,
    startListener,
    stopListener,
    calculateSignature,
  };
}
