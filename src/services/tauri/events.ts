import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { isTauriAvailable } from './ipc';
import type { TrafficItem, PendingFlow, WebSocketMessage, WebhookDelivery } from '../../types';

export async function listenTrafficCaptured(callback: (item: TrafficItem) => void): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<TrafficItem>('traffic_captured', (event) => callback(event.payload));
  }
  return () => {};
}

export async function listenInterceptPending(callback: (flow: PendingFlow) => void): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<PendingFlow>('intercept_pending', (event) => callback(event.payload));
  }
  return () => {};
}

export async function listenWsMessageCaptured(callback: (msg: WebSocketMessage) => void): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<WebSocketMessage>('ws_message_captured', (event) => callback(event.payload));
  }
  return () => {};
}

export async function listenWebhookCaptured(callback: (delivery: WebhookDelivery) => void): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<WebhookDelivery>('webhook_delivery_captured', (event) => callback(event.payload));
  }
  return () => {};
}
