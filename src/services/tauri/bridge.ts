import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { isTauriAvailable, safeInvoke } from './ipc';
import type {
  TrafficItem,
  PendingFlow,
  RepeaterRequestItem,
  RepeaterHistoryItem,
  WebSocketMessage,
  WebhookDelivery,
  VariableItem,
  EnvironmentItem,
  InterceptConfig,
  InterceptRule,
} from '../../types';

export const tauriBridge = {
  proxy: {
    getProxyStatus: () => safeInvoke('get_proxy_status'),
    setProxyMode: (mode: string) => safeInvoke('set_proxy_mode', { mode }),
    updateNetworkSettings: (bindings: string[]) => safeInvoke('update_network_settings', { bindings }),
    updateInterceptConfig: (config: InterceptConfig) => safeInvoke('update_state', { config }),
    updateFilterConfig: (config: { rules: InterceptRule[] }) => safeInvoke('update_filter_config', { config }),
    resumeFlow: (id: string, action: 'forward' | 'drop', modifiedBody?: string, modifiedHeaders?: Record<string, string>) =>
      safeInvoke('resume_flow', { id, action: { action, modifiedBody, modifiedHeaders } }),
  },
  history: {
    get: () => safeInvoke<TrafficItem[]>('get_http_history', undefined, []),
    clear: () => safeInvoke<void>('clear_http_history'),
    deleteItem: (id: string) => safeInvoke<void>('delete_history_item', { id }),
  },
  repeater: {
    getRequests: (groupId?: string | null) => safeInvoke<RepeaterRequestItem[]>('get_repeater_requests', { groupId }, []),
    execute: (request: Partial<RepeaterRequestItem>) => safeInvoke<RepeaterHistoryItem>('execute_repeater_request', { request }),
    createItem: (item: Partial<RepeaterRequestItem>) => safeInvoke('create_repeater_item', { item }),
    updateRequest: (request: Partial<RepeaterRequestItem>) => safeInvoke('update_repeater_request', { request }),
    deleteRequest: (id: string) => safeInvoke('delete_repeater_request', { id }),
    getHistory: (repeaterId: string) => safeInvoke('get_repeater_history', { repeaterId }, []),
  },
  websocket: {
    sendMessage: (msg: Partial<WebSocketMessage>) => safeInvoke('send_ws_message', { msg }),
  },
  webhook: {
    getDeliveries: () => safeInvoke<WebhookDelivery[]>('get_webhook_deliveries', undefined, []),
  },
  workspace: {
    getSyncData: () => safeInvoke('get_sync_data'),
    createVariable: (variable: Partial<VariableItem>) => safeInvoke('create_variable', { variable }),
    createEnvironment: (env: Partial<EnvironmentItem>) => safeInvoke('create_environment', { env }),
  },
  utilities: {
    calculateCvss: (vector: string) =>
      safeInvoke<{ base_score: number; severity: string; vector_string: string; metrics: Record<string, string> }>(
        'calculate_cvss',
        { vector }
      ),
    convertEncoding: (input: string, mode: string) =>
      safeInvoke<string>('convert_encoding', { input, mode }),
  },
  listenTraffic: (cb: (item: TrafficItem) => void): Promise<UnlistenFn> => {
    if (isTauriAvailable()) {
      return listen<TrafficItem>('traffic_captured', (e) => cb(e.payload));
    }
    return Promise.resolve(() => {});
  },
  listenIntercept: (cb: (flow: PendingFlow) => void): Promise<UnlistenFn> => {
    if (isTauriAvailable()) {
      return listen<PendingFlow>('intercept_pending', (e) => cb(e.payload));
    }
    return Promise.resolve(() => {});
  },
  listenWsMessage: (cb: (msg: WebSocketMessage) => void): Promise<UnlistenFn> => {
    if (isTauriAvailable()) {
      return listen<WebSocketMessage>('ws_message_captured', (e) => cb(e.payload));
    }
    return Promise.resolve(() => {});
  },
  listenWebhook: (cb: (delivery: WebhookDelivery) => void): Promise<UnlistenFn> => {
    if (isTauriAvailable()) {
      return listen<WebhookDelivery>('webhook_delivery_captured', (e) => cb(e.payload));
    }
    return Promise.resolve(() => {});
  },
};
