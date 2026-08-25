export type NavModule =
  | 'http-history'
  | 'intercept'
  | 'repeater'
  | 'collections'
  | 'websockets'
  | 'webhooks'
  | 'workspace'
  | 'utilities'
  | 'settings';

export interface KeyValuePair {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
}

export interface HeaderPair {
  key: string;
  value: string;
}

export interface TrafficItem {
  id: string;
  method: string;
  host: string;
  path: string;
  url: string;
  statusCode: number;
  contentType: string;
  size: number;
  durationMs: number;
  timestamp: number;
  requestHeaders: HeaderPair[];
  requestBody: string;
  responseHeaders: HeaderPair[];
  responseBody: string;
  ip?: string;
  isIntercepted?: boolean;
}

export interface InterceptRule {
  id: string;
  target: 'domain' | 'path' | 'method';
  pattern: string;
  action: 'intercept' | 'pass';
  enabled: boolean;
}

export interface InterceptConfig {
  enabled: boolean;
  mode: 'all' | 'rules_only' | 'off';
  direction: 'request' | 'response' | 'both';
  autoTimeoutSeconds: number;
}

export interface PendingFlow {
  id: string;
  timestamp: number;
  method: string;
  url: string;
  headers: HeaderPair[];
  body: string;
  direction: 'request' | 'response';
  originalItem: Partial<TrafficItem>;
}

export interface RepeaterRequestItem {
  id: string;
  name: string;
  groupId?: string | null;
  method: string;
  url: string;
  params: KeyValuePair[];
  headers: KeyValuePair[];
  bodyType: 'none' | 'json' | 'form-data' | 'x-www-form-urlencoded' | 'raw';
  body: string;
  formData?: KeyValuePair[];
  autoExtractRules?: AutoExtractRule[];
  created_at?: number;
  updated_at?: number;
}

export interface AutoExtractRule {
  id: string;
  type: 'jsonpath' | 'regex';
  expression: string;
  targetVariable: string;
  enabled: boolean;
}

export interface RepeaterGroup {
  id: string;
  name: string;
  description?: string;
  environmentIds?: string[];
  created_at?: number;
}

export interface RepeaterHistoryItem {
  id: string;
  requestId: string;
  timestamp: number;
  method: string;
  url: string;
  statusCode: number;
  durationMs: number;
  size: number;
  requestHeaders: HeaderPair[];
  requestBody: string;
  responseHeaders: HeaderPair[];
  responseBody: string;
}

export interface VariableItem {
  id: string;
  key: string;
  value: string;
  environmentId: string; // 'global' or environment ID
  isSecret: boolean;
  type: 'string' | 'json' | 'number' | 'boolean';
  description?: string;
}

export interface EnvironmentItem {
  id: string;
  name: string;
  isDefault?: boolean;
  color?: string;
}

export interface ReplacementRule {
  id: string;
  domain: string;
  target: 'header' | 'body' | 'url';
  isRegex: boolean;
  pattern: string;
  replacement: string;
  enabled: boolean;
}

export interface CollectionLink {
  groupId: string;
  environmentId: string;
}

export interface WebSocketConn {
  connectionId: string;
  url: string;
  status: 'connected' | 'disconnected' | 'connecting';
  handshakeTime: number;
  protocol?: string;
}

export interface WebSocketMessage {
  id: string;
  connection_id: string;
  direction: 'to_server' | 'to_client';
  msg_type: 'text' | 'json' | 'binary';
  payload: string;
  timestamp: number;
  is_intercepted?: boolean;
}

export interface WebhookEndpoint {
  id: string;
  path: string;
  name: string;
  secretKey: string;
  createdAt: number;
  hitCount: number;
}

export interface WebhookDelivery {
  id: string;
  endpointId: string;
  endpointPath: string;
  timestamp: number;
  headers: HeaderPair[];
  payload: string;
  signatureStatus: 'valid' | 'invalid' | 'none';
  computedHmac?: string;
  providedHmac?: string;
}

export interface WebhookListenerConfig {
  port: number;
  is_running: boolean;
}

export interface ProxyStatus {
  mode: 'normal' | 'intercept' | 'off';
  bindings: string[];
  activeCount?: number;
}

export interface AppPreferences {
  theme: 'dark' | 'light';
  layoutMode: 'horizontal' | 'vertical';
  fontSize: 'sm' | 'md' | 'lg';
  webhookPort: number;
  proxyPort: number;
  proxyHost: string;
  dbPath: string;
}
