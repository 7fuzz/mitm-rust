import type { RepeaterHistoryItem } from '../services/tauri/bridge';
import type { TrafficItem } from '../types';
import { formatRawCurl, formatReqAndRes, formatUrlBodyAndRes } from './reqResFormatter';

/** One recorded send (request as sent + response); both Repeater and Collection runs fit this shape */
export type RunRecord = Pick<
  RepeaterHistoryItem,
  'id' | 'method' | 'url' | 'requestHeaders' | 'requestBody' | 'statusCode' | 'responseHeaders' | 'responseBody' | 'durationMs' | 'executedAtMs'
>;

/** Path and query of a run's URL, falling back to the raw URL */
export const runPath = (url: string) => {
  try {
    const parsed = new URL(/^https?:\/\//.test(url) ? url : `https://${url}`);
    return parsed.pathname + parsed.search;
  } catch {
    return url;
  }
};

/** Converts a run (request as sent + response) into the shape the copy formatters take */
export const historyItemToTrafficItem = (hist: RunRecord): TrafficItem => {
  let host = '';
  let path = '/';
  try {
    const urlWithScheme = !hist.url.startsWith('http://') && !hist.url.startsWith('https://')
      ? `https://${hist.url}`
      : hist.url;
    const urlObj = new URL(urlWithScheme);
    host = urlObj.host;
    path = urlObj.pathname + urlObj.search;
  } catch {
    host = hist.url;
  }

  const ctHeader = (hist.responseHeaders || []).find(
    (h) => h.key?.toLowerCase() === 'content-type'
  );
  const contentType = ctHeader ? ctHeader.value : '';

  return {
    id: `rep-hist-${hist.id}`,
    method: hist.method,
    url: hist.url,
    host,
    path,
    contentType,
    statusCode: hist.statusCode,
    size: hist.responseBody ? hist.responseBody.length : 0,
    requestHeaders: (hist.requestHeaders || []).map((h) => ({ key: h.key, value: h.value })),
    responseHeaders: (hist.responseHeaders || []).map((h) => ({ key: h.key, value: h.value })),
    requestBody: hist.requestBody || '',
    responseBody: hist.responseBody || '',
    durationMs: hist.durationMs,
    timestamp: hist.executedAtMs,
    phase: 'done',
    isIntercepted: false,
    isRewritten: false,
    isFailed: hist.statusCode === 0,
  };
};

/** Copy formats that need both the request as sent and its response (same set as HTTP History) */
export const RUN_COPY_ACTIONS: Array<{
  label: string;
  icon: string;
  iconClass: string;
  format: (item: TrafficItem) => string;
  message: string;
  /** Copies only the request, so a menu that already copies the request can skip it */
  requestOnly?: boolean;
}> = [
  { label: 'Copy cURL', icon: 'terminal_line', iconClass: 'text-blue-500', format: formatRawCurl, message: 'Copied cURL!', requestOnly: true },
  { label: 'Copy cURL and Response', icon: 'transfer_line', iconClass: 'text-sky-500', format: formatReqAndRes, message: 'Copied cURL & Response!' },
  { label: 'Copy URL, Body, and Response', icon: 'file_code_line', iconClass: 'text-amber-500', format: formatUrlBodyAndRes, message: 'Copied!' },
];
