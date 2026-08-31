import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { isTauriAvailable } from './ipc';
import type {
  TrafficItem,
  PendingFlow,
  WebhookDeliveryCapturedPayload,
  WebSocketMessageCapturedPayload,
  WebSocketConnectionPayload,
} from '../../types';

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

export async function listenWsMessageCaptured(
  callback: (payload: WebSocketMessageCapturedPayload) => void,
): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<WebSocketMessageCapturedPayload>('websocket_message_event', (event) =>
      callback(event.payload),
    );
  }
  return () => {};
}

export async function listenWsConnectionEvent(
  callback: (payload: WebSocketConnectionPayload) => void,
): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<WebSocketConnectionPayload>('websocket_connection_event', (event) =>
      callback(event.payload),
    );
  }
  return () => {};
}

export async function listenWebhookCaptured(callback: (payload: WebhookDeliveryCapturedPayload) => void): Promise<UnlistenFn> {
  if (isTauriAvailable()) {
    return await listen<WebhookDeliveryCapturedPayload>('webhook_delivery_captured', (event) => callback(event.payload));
  }
  return () => {};
}
