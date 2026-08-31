import type { TrafficItem } from '../types';
import { serializeTruncatedTree } from '../components/common/json-tree/jsonTreeSerializer';

/**
 * Truncate long header values like Bearer tokens or large strings
 */
export function truncateHeaderValue(key: string, value: string): string {
  if (!value) return value;

  // Bearer token truncation: e.g. "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  const bearerMatch = value.match(/^(Bearer\s+)(\S+)(.*)$/i);
  if (bearerMatch) {
    const prefix = bearerMatch[1];
    const token = bearerMatch[2];
    const rest = bearerMatch[3] || '';
    if (token.length > 25) {
      return `${prefix}${token.substring(0, 20)}...${rest}`;
    }
    return value;
  }

  // Basic auth or other token prefixes
  const basicMatch = value.match(/^(Basic\s+)(\S+)(.*)$/i);
  if (basicMatch && basicMatch[2].length > 25) {
    return `${basicMatch[1]}${basicMatch[2].substring(0, 20)}...${basicMatch[3] || ''}`;
  }

  // Auth / key / token / secret headers
  const lowerKey = key.toLowerCase();
  if (
    (lowerKey.includes('auth') ||
      lowerKey.includes('token') ||
      lowerKey.includes('secret') ||
      lowerKey.includes('api-key') ||
      lowerKey === 'apikey') &&
    value.length > 30
  ) {
    return `${value.substring(0, 20)}...`;
  }

  // Any other very long header value (> 200 chars)
  if (value.length > 200) {
    return `${value.substring(0, 100)}...`;
  }

  return value;
}

/**
 * Generates a cURL command from a TrafficItem with header and body truncation.
 */
export function formatCurlCommand(item: TrafficItem): string {
  let curl = `curl -X ${item.method} "${item.url}"`;

  // Headers with Bearer and long value truncation
  if (item.requestHeaders && item.requestHeaders.length > 0) {
    item.requestHeaders.forEach((h) => {
      if (!h.key) return;
      const truncatedVal = truncateHeaderValue(h.key, h.value || '');
      curl += ` \\\n  -H "${h.key}: ${truncatedVal.replace(/"/g, '\\"')}"`;
    });
  }

  // Request Body: JSON truncated like tree or raw text
  if (item.requestBody && item.requestBody.trim()) {
    let bodyPayload = item.requestBody;
    try {
      const parsed = JSON.parse(item.requestBody);
      bodyPayload = serializeTruncatedTree(parsed);
    } catch {
      if (item.requestBody.length > 500) {
        bodyPayload = `${item.requestBody.substring(0, 300)}... (truncated)`;
      }
    }
    curl += ` \\\n  --data '${bodyPayload.replace(/'/g, "'\\''")}'`;
  }

  return curl;
}

/**
 * Formats response status code and body with tree truncation.
 * Header is omitted as requested ("for respose just code and body, no need header for now").
 */
export function formatResponseBody(item: TrafficItem): string {
  const code = item.statusCode || 200;

  if (!item.responseBody || !item.responseBody.trim()) {
    return `${code}`;
  }

  if (item.responseBody.startsWith('base64:')) {
    const rawB64 = item.responseBody.substring(7);
    const mime = item.contentType || 'application/octet-stream';
    return `${code}\n[Binary Content (${mime}): ${rawB64.length} chars base64]`;
  }

  try {
    const parsed = JSON.parse(item.responseBody);
    const treeFormatted = serializeTruncatedTree(parsed);
    return `${code}\n${treeFormatted}`;
  } catch {
    if (item.responseBody.length > 1000) {
      return `${code}\n${item.responseBody.substring(0, 500)}\n... (truncated)`;
    }
    return `${code}\n${item.responseBody}`;
  }
}

/**
 * Formats both Request (as cURL) and Response (code and body) for copying.
 */
export function formatReqAndRes(item: TrafficItem): string {
  const curl = formatCurlCommand(item);
  const response = formatResponseBody(item);
  return `${curl}\n\n${response}`;
}
