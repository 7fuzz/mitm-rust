import { invoke } from '@tauri-apps/api/core';
import type {
  TrafficItem,
  ProxyConfig,
  ProxyStatus,
  InterceptConfig,
  InterceptRule,
  RepeaterRequestItem,
  RepeaterGroup,
  RepeaterHistoryItem,
  VariableItem,
  EnvironmentItem,
  ReplacementRule,
  WebSocketConn,
  WebSocketMessage,
  WebhookEndpoint,
  WebhookDelivery,
  WebhookListenerConfig,
  WebhookReplayResult,
  AppPreferences,
} from '../../types';

// Check if running inside Tauri runtime
export const isTauriAvailable = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// Safe invoke wrapper with browser fallback support
export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (isTauriAvailable()) {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] Command "${cmd}" failed:`, err);
      if (fallback !== undefined) return fallback;
      throw err;
    }
  } else {
    console.debug(`[Browser Mock IPC] Command "${cmd}" called with args:`, args);
    if (fallback !== undefined) return fallback;
    return undefined as unknown as T;
  }
}

// CA Commands
export async function getRootCaPem(): Promise<string> {
  return safeInvoke<string>('get_root_ca_pem', undefined, '-----BEGIN CERTIFICATE-----\nMock Root CA Certificate for MITM Proxy\n-----END CERTIFICATE-----');
}

export async function exportRootCa(destinationPath: string): Promise<void> {
  return safeInvoke<void>('export_root_ca', { destinationPath });
}

export async function regenerateRootCa(): Promise<string> {
  return safeInvoke<string>('regenerate_root_ca', undefined, '-----BEGIN CERTIFICATE-----\nRegenerated Mock Root CA Certificate\n-----END CERTIFICATE-----');
}

// Proxy Status & Config
export async function getProxyStatus(): Promise<ProxyStatus> {
  return safeInvoke<ProxyStatus>('get_proxy_status', undefined, {
    mode: 'normal',
    bindings: ['0.0.0.0:8080'],
    activeCount: 1,
  });
}

export async function setProxyMode(mode: string): Promise<void> {
  return safeInvoke<void>('set_proxy_mode', { mode });
}

export async function updateNetworkSettings(bindings: string[]): Promise<void> {
  return safeInvoke<void>('update_network_settings', { bindings });
}

// HTTP Traffic History Commands
export async function getHttpHistory(): Promise<TrafficItem[]> {
  return safeInvoke<TrafficItem[]>('get_http_history', undefined, []);
}

export async function clearHttpHistory(): Promise<void> {
  return safeInvoke<void>('clear_http_history', undefined);
}

// Intercept Flow
export async function updateInterceptConfig(config: InterceptConfig): Promise<void> {
  return safeInvoke<void>('update_state', { config });
}

export async function updateFilterConfig(config: { rules: InterceptRule[] }): Promise<void> {
  return safeInvoke<void>('update_filter_config', { config });
}

export async function resumeFlow(id: string, action: { action: 'forward' | 'drop'; modifiedBody?: string; modifiedHeaders?: Record<string, string> }): Promise<void> {
  return safeInvoke<void>('resume_flow', { id, action });
}

// Repeater
export async function getRepeaterRequests(groupId?: string | null): Promise<RepeaterRequestItem[]> {
  return safeInvoke<RepeaterRequestItem[]>('get_repeater_requests', { groupId }, []);
}

export async function executeRepeaterRequest(request: Partial<RepeaterRequestItem>): Promise<RepeaterHistoryItem> {
  return safeInvoke<RepeaterHistoryItem>('execute_repeater_request', { request }, {
    id: 'hist-' + Date.now(),
    requestId: request.id || 'req-1',
    timestamp: Date.now(),
    method: request.method || 'GET',
    url: request.url || 'https://api.example.com/v1/health',
    statusCode: 200,
    durationMs: 45,
    size: 512,
    requestHeaders: request.headers || [{ key: 'User-Agent', value: 'MITM-Developer-Studio' }],
    requestBody: request.body || '',
    responseHeaders: [
      { key: 'Content-Type', value: 'application/json' },
      { key: 'Server', value: 'nginx/1.24.0' },
      { key: 'X-RateLimit-Limit', value: '1000' }
    ],
    responseBody: JSON.stringify({ status: 'ok', message: 'Executed successfully', timestamp: new Date().toISOString() }, null, 2),
  });
}

export async function createRepeaterItem(item: Partial<RepeaterRequestItem>): Promise<RepeaterRequestItem> {
  return safeInvoke<RepeaterRequestItem>('create_repeater_item', { item }, {
    id: 'rep-' + Date.now(),
    name: item.name || 'New Request',
    method: item.method || 'GET',
    url: item.url || 'https://api.example.com',
    params: item.params || [],
    headers: item.headers || [],
    bodyType: item.bodyType || 'none',
    body: item.body || '',
    groupId: item.groupId || null,
  });
}

export async function updateRepeaterRequest(item: RepeaterRequestItem): Promise<void> {
  return safeInvoke<void>('update_repeater_request', { item });
}

export async function deleteRepeaterRequest(id: string): Promise<void> {
  return safeInvoke<void>('delete_repeater_request', { id });
}

export async function getRepeaterHistory(requestId: string): Promise<RepeaterHistoryItem[]> {
  return safeInvoke<RepeaterHistoryItem[]>('get_repeater_history', { requestId }, []);
}

// Groups / Collections
export async function createRepeaterGroup(name: string, description?: string): Promise<RepeaterGroup> {
  return safeInvoke<RepeaterGroup>('create_repeater_group', { name, description }, {
    id: 'grp-' + Date.now(),
    name,
    description,
    environmentIds: [],
  });
}

export async function renameRepeaterGroup(id: string, name: string): Promise<void> {
  return safeInvoke<void>('rename_repeater_group', { id, name });
}

export async function deleteRepeaterGroup(id: string): Promise<void> {
  return safeInvoke<void>('delete_repeater_group', { id });
}

// Workspace & Environments
export async function createVariable(variable: Partial<VariableItem>): Promise<VariableItem> {
  return safeInvoke<VariableItem>('create_variable', { variable }, {
    id: 'var-' + Date.now(),
    key: variable.key || 'NEW_VAR',
    value: variable.value || '',
    environmentId: variable.environmentId || 'global',
    isSecret: variable.isSecret || false,
    type: variable.type || 'string',
  });
}

export async function updateVariable(variable: VariableItem): Promise<void> {
  return safeInvoke<void>('update_variable', { variable });
}

export async function deleteVariable(id: string): Promise<void> {
  return safeInvoke<void>('delete_variable', { id });
}

export async function createEnvironment(name: string, color?: string): Promise<EnvironmentItem> {
  return safeInvoke<EnvironmentItem>('create_environment', { name, color }, {
    id: 'env-' + Date.now(),
    name,
    color: color || '#38bdf8',
  });
}

export async function deleteEnvironment(id: string): Promise<void> {
  return safeInvoke<void>('delete_environment', { id });
}

export async function setActiveEnvironment(id: string): Promise<void> {
  return safeInvoke<void>('set_active_environment', { id });
}

// Automated Replacements
export async function saveReplacementsBulk(replacements: ReplacementRule[]): Promise<void> {
  return safeInvoke<void>('save_replacements_bulk', { replacements });
}

export async function deleteReplacement(id: string): Promise<void> {
  return safeInvoke<void>('delete_replacement', { id });
}

// WebSockets
export async function getWsConnections(): Promise<WebSocketConn[]> {
  return safeInvoke<WebSocketConn[]>('get_ws_connections', undefined, []);
}

export async function getWsMessages(connectionId: number, limit?: number): Promise<WebSocketMessage[]> {
  return safeInvoke<WebSocketMessage[]>('get_ws_messages', { connectionId, limit }, []);
}

export async function connectWsClient(url: string, headers?: [string, string][]): Promise<WebSocketConn> {
  return safeInvoke<WebSocketConn>('connect_ws_client', { url, headers }, {
    connectionId: Date.now(),
    url,
    status: 'connected',
    handshakeTime: Date.now(),
    isClientSession: true,
    messageCount: 0,
  });
}

export async function disconnectWsClient(connectionId: number): Promise<void> {
  return safeInvoke<void>('disconnect_ws_client', { connectionId });
}

export async function sendWsMessage(
  connectionId: number,
  direction: 'to_server' | 'to_client',
  msgType: 'text' | 'json' | 'binary',
  payload: string,
): Promise<WebSocketMessage> {
  return safeInvoke<WebSocketMessage>('send_ws_message', { connectionId, direction, msgType, payload }, {
    id: Date.now(),
    connection_id: connectionId,
    direction,
    msg_type: msgType,
    payload,
    timestamp: Date.now(),
    length: payload.length,
    is_injected: true,
  });
}

export async function clearWsMessages(connectionId?: number): Promise<void> {
  return safeInvoke<void>('clear_ws_messages', { connectionId });
}

export async function deleteWsConnection(connectionId: number): Promise<void> {
  return safeInvoke<void>('delete_ws_connection', { connectionId });
}

export async function setWsMitmEnabled(enabled: boolean): Promise<ProxyConfig> {
  return safeInvoke<ProxyConfig>('set_ws_mitm_enabled', { enabled }, {
    proxyEnabled: true,
    interceptEnabled: false,
    interceptMode: 'request',
    proxyMode: 'on',
    port: 8080,
    host: '0.0.0.0',
    wsMitmEnabled: enabled,
  });
}

export async function getProxyState(): Promise<ProxyConfig> {
  return safeInvoke<ProxyConfig>('get_proxy_state', undefined, {
    proxyEnabled: true,
    interceptEnabled: false,
    interceptMode: 'request',
    proxyMode: 'on',
    port: 8080,
    host: '0.0.0.0',
    wsMitmEnabled: false,
  });
}

// Webhooks
export async function getWebhookEndpoints(): Promise<WebhookEndpoint[]> {
  return safeInvoke<WebhookEndpoint[]>('get_webhook_endpoints', undefined, []);
}

export async function createWebhookEndpoint(endpoint: Partial<WebhookEndpoint>): Promise<WebhookEndpoint> {
  const payload: WebhookEndpoint = {
    id: endpoint.id || 'wh-ep-' + Date.now(),
    path: endpoint.path || '/webhook/test',
    name: endpoint.name || 'Test Endpoint',
    secretKey: endpoint.secretKey || 'whsec_secret123',
    createdAt: endpoint.createdAt || Date.now(),
    hitCount: endpoint.hitCount || 0,
  };
  return safeInvoke<WebhookEndpoint>('create_webhook_endpoint', { endpoint: payload }, payload);
}

export async function deleteWebhookEndpoint(id: string): Promise<void> {
  return safeInvoke<void>('delete_webhook_endpoint', { id });
}

export async function getWebhookDeliveries(limit?: number): Promise<WebhookDelivery[]> {
  return safeInvoke<WebhookDelivery[]>('get_webhook_deliveries', { limit }, []);
}

export async function clearWebhookDeliveries(): Promise<void> {
  return safeInvoke<void>('clear_webhook_deliveries');
}

export async function getWebhookListenerStatus(): Promise<WebhookListenerConfig> {
  return safeInvoke<WebhookListenerConfig>('get_webhook_listener_status', undefined, { port: 9000, is_running: false });
}

export async function startWebhookListener(port: number): Promise<void> {
  return safeInvoke<void>('start_webhook_listener', { port });
}

export async function setWebhookPort(port: number): Promise<void> {
  return safeInvoke<void>('set_webhook_port', { port });
}

export async function stopWebhookListener(): Promise<void> {
  return safeInvoke<void>('stop_webhook_listener');
}

export async function calculateWebhookSignature(secret: string, body: string, provider: string): Promise<{ header_name: string; header_value: string }> {
  return safeInvoke<{ header_name: string; header_value: string }>('calculate_webhook_signature', { secret, body, provider }, {
    header_name: 'X-Hub-Signature-256',
    header_value: 'sha256=mocked_hmac_signature_hex_value',
  });
}

export async function replayWebhookDelivery(id: number, targetUrl: string): Promise<WebhookReplayResult> {
  return safeInvoke<WebhookReplayResult>('replay_webhook_delivery', { id, targetUrl }, {
    success: true,
    statusCode: 200,
    responseBody: '{"status":"ok"}',
    durationMs: 45,
  });
}

// Settings
export async function updatePrefs(prefs: Partial<AppPreferences>): Promise<void> {
  return safeInvoke<void>('update_prefs', { prefs });
}
