import type { TrafficItem } from '../types';
import { serializeTruncatedTree } from '../components/common/json-tree/jsonTreeSerializer';

export const HTTP_STATUS_TEXTS: Record<number, string> = {
  100: 'Continue',
  101: 'Switching Protocols',
  102: 'Processing',
  103: 'Early Hints',
  200: 'OK',
  201: 'Created',
  202: 'Accepted',
  203: 'Non-Authoritative Information',
  204: 'No Content',
  205: 'Reset Content',
  206: 'Partial Content',
  207: 'Multi-Status',
  208: 'Already Reported',
  226: 'IM Used',
  300: 'Multiple Choices',
  301: 'Moved Permanently',
  302: 'Found',
  303: 'See Other',
  304: 'Not Modified',
  305: 'Use Proxy',
  307: 'Temporary Redirect',
  308: 'Permanent Redirect',
  400: 'Bad Request',
  401: 'Unauthorized',
  402: 'Payment Required',
  403: 'Forbidden',
  404: 'Not Found',
  405: 'Method Not Allowed',
  406: 'Not Acceptable',
  407: 'Proxy Authentication Required',
  408: 'Request Timeout',
  409: 'Conflict',
  410: 'Gone',
  411: 'Length Required',
  412: 'Precondition Failed',
  413: 'Payload Too Large',
  414: 'URI Too Long',
  415: 'Unsupported Media Type',
  416: 'Range Not Satisfiable',
  417: 'Expectation Failed',
  418: "I'm a teapot",
  421: 'Misdirected Request',
  422: 'Unprocessable Entity',
  423: 'Locked',
  424: 'Failed Dependency',
  425: 'Too Early',
  426: 'Upgrade Required',
  428: 'Precondition Required',
  429: 'Too Many Requests',
  431: 'Request Header Fields Too Large',
  451: 'Unavailable For Legal Reasons',
  500: 'Internal Server Error',
  501: 'Not Implemented',
  502: 'Bad Gateway',
  503: 'Service Unavailable',
  504: 'Gateway Timeout',
  505: 'HTTP Version Not Supported',
  506: 'Variant Also Negotiates',
  507: 'Insufficient Storage',
  508: 'Loop Detected',
  510: 'Not Extended',
  511: 'Network Authentication Required',
};

export function getHttpStatusText(code: number): string {
  if (HTTP_STATUS_TEXTS[code]) {
    return HTTP_STATUS_TEXTS[code];
  }
  if (code >= 200 && code < 300) return 'OK';
  if (code >= 300 && code < 400) return 'Redirect';
  if (code >= 400 && code < 500) return 'Client Error';
  if (code >= 500 && code < 600) return 'Server Error';
  return 'Unknown';
}

export interface CustomCopyOptions {
  requestFormat: 'curl' | 'request';
  includeRequestHeaders: boolean;
  includeRequestBody: boolean;
  includeResponse: boolean;
  includeResponseCode: boolean;
  includeResponseStatusText: boolean;
  includeResponseHeaders: boolean;
  includeResponseBody: boolean;
  truncateBodies?: boolean;
}

export const DEFAULT_COPY_CUSTOM_OPTIONS: CustomCopyOptions = {
  requestFormat: 'curl',
  includeRequestHeaders: true,
  includeRequestBody: true,
  includeResponse: true,
  includeResponseCode: true,
  includeResponseStatusText: false,
  includeResponseHeaders: false,
  includeResponseBody: true,
  truncateBodies: false,
};

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
 * Generates a full raw cURL command from a TrafficItem without truncation.
 */
export function formatRawCurl(item: TrafficItem): string {
  let curl = `curl -X ${item.method} "${item.url}"`;

  if (item.requestHeaders && item.requestHeaders.length > 0) {
    item.requestHeaders.forEach((h) => {
      if (!h.key) return;
      curl += ` \\\n  -H "${h.key}: ${(h.value || '').replace(/"/g, '\\"')}"`;
    });
  }

  if (item.requestBody && item.requestBody.trim()) {
    curl += ` \\\n  --data '${item.requestBody.replace(/'/g, "'\\''")}'`;
  }

  return curl;
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

/**
 * Formats URL, Body, and Response without headers (with method).
 */
export function formatUrlBodyAndRes(item: TrafficItem): string {
  return formatCustomCopy(item, {
    requestFormat: 'request',
    includeRequestHeaders: false,
    includeRequestBody: true,
    includeResponse: true,
    includeResponseCode: true,
    includeResponseStatusText: false,
    includeResponseHeaders: false,
    includeResponseBody: true,
    truncateBodies: false,
  });
}

/**
 * Fully customizable copy formatter.
 */
export function formatCustomCopy(item: TrafficItem, options: CustomCopyOptions): string {
  let req = '';

  // ─── Format Request ────────────────────────────────────────────────────────
  if (options.requestFormat === 'curl') {
    req = `curl -X ${item.method} "${item.url}"`;

    if (options.includeRequestHeaders && item.requestHeaders && item.requestHeaders.length > 0) {
      item.requestHeaders.forEach((h) => {
        if (!h.key) return;
        const val = options.truncateBodies
          ? truncateHeaderValue(h.key, h.value || '')
          : (h.value || '');
        req += ` \\\n  -H "${h.key}: ${val.replace(/"/g, '\\"')}"`;
      });
    }

    if (options.includeRequestBody && item.requestBody && item.requestBody.trim()) {
      let bodyPayload = item.requestBody;
      if (options.truncateBodies) {
        try {
          const parsed = JSON.parse(item.requestBody);
          bodyPayload = serializeTruncatedTree(parsed);
        } catch {
          if (item.requestBody.length > 500) {
            bodyPayload = `${item.requestBody.substring(0, 300)}... (truncated)`;
          }
        }
      }
      req += ` \\\n  --data '${bodyPayload.replace(/'/g, "'\\''")}'`;
    }
  } else {
    // Request (HTTP) shape
    req = `${item.method} ${item.url}`;

    const reqHeaderLines = options.includeRequestHeaders && item.requestHeaders && item.requestHeaders.length > 0
      ? item.requestHeaders
          .filter((h) => h.key)
          .map((h) => {
            const val = options.truncateBodies ? truncateHeaderValue(h.key, h.value || '') : (h.value || '');
            return `${h.key}: ${val}`;
          })
          .join('\n')
      : '';

    if (reqHeaderLines) {
      req += `\n${reqHeaderLines}`;
    }

    if (options.includeRequestBody && item.requestBody && item.requestBody.trim()) {
      let bodyPayload = item.requestBody;
      if (options.truncateBodies) {
        try {
          const parsed = JSON.parse(item.requestBody);
          bodyPayload = serializeTruncatedTree(parsed);
        } catch {
          if (item.requestBody.length > 500) {
            bodyPayload = `${item.requestBody.substring(0, 300)}... (truncated)`;
          }
        }
      }
      req += reqHeaderLines ? `\n\n${bodyPayload}` : `\n${bodyPayload}`;
    }
  }

  // ─── Format Response ───────────────────────────────────────────────────────
  let responseSection = '';
  if (options.includeResponse) {
    const statusCode = item.statusCode || 200;
    const statusText = getHttpStatusText(statusCode);
    let statusLine = '';

    if (options.includeResponseCode && options.includeResponseStatusText) {
      statusLine = `${statusCode} ${statusText}`;
    } else if (options.includeResponseCode) {
      statusLine = `${statusCode}`;
    } else if (options.includeResponseStatusText) {
      statusLine = `${statusText}`;
    }

    const resHeaderLines = options.includeResponseHeaders && item.responseHeaders && item.responseHeaders.length > 0
      ? item.responseHeaders
          .filter((h) => h.key)
          .map((h) => `${h.key}: ${h.value || ''}`)
          .join('\n')
      : '';

    let resBodyText = '';
    if (options.includeResponseBody && item.responseBody && item.responseBody.trim()) {
      if (item.responseBody.startsWith('base64:')) {
        const rawB64 = item.responseBody.substring(7);
        const mime = item.contentType || 'application/octet-stream';
        resBodyText = `[Binary Content (${mime}): ${rawB64.length} chars base64]`;
      } else if (options.truncateBodies) {
        try {
          const parsed = JSON.parse(item.responseBody);
          resBodyText = serializeTruncatedTree(parsed);
        } catch {
          if (item.responseBody.length > 1000) {
            resBodyText = `${item.responseBody.substring(0, 500)}\n... (truncated)`;
          } else {
            resBodyText = item.responseBody;
          }
        }
      } else {
        resBodyText = item.responseBody;
      }
    }

    const headerSection = [statusLine, resHeaderLines].filter(Boolean).join('\n');
    if (headerSection && resBodyText) {
      responseSection = `${headerSection}\n${resHeaderLines ? '\n' : ''}${resBodyText}`;
    } else if (headerSection) {
      responseSection = headerSection;
    } else if (resBodyText) {
      responseSection = resBodyText;
    }
  }

  // ─── Combine Request & Response ────────────────────────────────────────────
  if (req && responseSection) {
    return `${req}\n\n${responseSection}`;
  }
  return req || responseSection;
}
