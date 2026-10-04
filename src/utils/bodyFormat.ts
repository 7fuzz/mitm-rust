import type { MultipartField, UrlEncodedParam } from '../types';
import { isJsonString, convertJsonToFormData, convertJsonToUrlEncoded, convertFormDataToUrlEncoded, convertUrlEncodedToFormData, convertUrlEncodedToJson } from './bodyConverters';
import { tryPrettifyJson } from './prettifyJson';

export type BodyKind = 'none' | 'json' | 'urlencoded' | 'multipart' | 'xml' | 'text' | 'binary';

export const BODY_KIND_LABELS: Record<BodyKind, string> = {
  none: 'Empty',
  json: 'JSON',
  urlencoded: 'URL-Encoded',
  multipart: 'Multipart',
  xml: 'XML',
  text: 'Text',
  binary: 'Binary',
};

/** Bodies the backend couldn't hold as UTF-8 travel as `base64:<data>`. */
export const BINARY_PREFIX = 'base64:';

const BINARY_CT_RE = /^(image|video|audio|font)\/|octet-stream|application\/(pdf|zip|gzip|x-protobuf|protobuf|grpc|wasm|msgpack|x-msgpack|cbor)/;
const URLENCODED_RE = /^[^\s=&]+=[^\s&]*(&[^\s=&]*=?[^\s&]*)*$/;

type HeaderLike = { key: string; value: string; enabled?: boolean } | [string, string];

export function findHeader(headers: HeaderLike[] | undefined, name: string): string {
  const lower = name.toLowerCase();
  for (const h of headers || []) {
    const [key, value, enabled] = Array.isArray(h) ? [h[0], h[1], true] : [h.key, h.value, h.enabled !== false];
    if (enabled && key.trim().toLowerCase() === lower) return value;
  }
  return '';
}

export function multipartBoundary(contentType: string, body = ''): string | null {
  const fromHeader = contentType.match(/boundary=(?:"([^"]+)"|([^;\s]+))/i);
  if (fromHeader) return fromHeader[1] ?? fromHeader[2];
  const text = body.startsWith(BINARY_PREFIX) ? bytesToLatin1(bodyToBytes(body).slice(0, 512)) : body;
  const firstLine = text.match(/^--([^\r\n]+)\r?\n/);
  return firstLine && /content-disposition/i.test(text.slice(0, 1024)) ? firstLine[1].trim() : null;
}

export function detectBodyKind(contentType: string, body: string): BodyKind {
  if (!body || !body.trim()) return 'none';
  const ct = contentType.toLowerCase();

  if (ct.includes('multipart/') && multipartBoundary(ct, body)) return 'multipart';
  if (body.startsWith(BINARY_PREFIX)) return multipartBoundary(ct, body) ? 'multipart' : 'binary';
  if (ct.includes('x-www-form-urlencoded')) return 'urlencoded';
  if (ct.includes('json')) return 'json';
  if (ct.includes('xml')) return 'xml';
  if (BINARY_CT_RE.test(ct)) return 'binary';

  const trimmed = body.trim();
  if (isJsonString(trimmed)) return 'json';
  if (multipartBoundary('', body)) return 'multipart';
  if (trimmed.startsWith('<')) return 'xml';
  if (URLENCODED_RE.test(trimmed)) return 'urlencoded';
  return 'text';
}

// --- bytes ---------------------------------------------------------------

export function bodyToBytes(body: string): Uint8Array {
  if (body.startsWith(BINARY_PREFIX)) {
    const bin = atob(body.slice(BINARY_PREFIX.length).replace(/\s+/g, ''));
    return latin1ToBytes(bin);
  }
  return new TextEncoder().encode(body);
}

/** Text when the bytes are valid UTF-8, `base64:` otherwise. */
export function bytesToBody(bytes: Uint8Array): string {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return BINARY_PREFIX + btoa(bytesToLatin1(bytes));
  }
}

export function binaryBodySize(body: string): number {
  return bodyToBytes(body).length;
}

function bytesToLatin1(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    out += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return out;
}

function latin1ToBytes(str: string): Uint8Array {
  const bytes = new Uint8Array(str.length);
  for (let i = 0; i < str.length; i++) bytes[i] = str.charCodeAt(i);
  return bytes;
}

function utf8OrNull(latin1: string): string | null {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(latin1ToBytes(latin1));
  } catch {
    return null;
  }
}

// --- urlencoded ----------------------------------------------------------

const decodeFormComponent = (s: string) => {
  try {
    return decodeURIComponent(s.replace(/\+/g, ' '));
  } catch {
    return s;
  }
};

export function parseUrlEncoded(raw: string): UrlEncodedParam[] {
  return raw
    .trim()
    .split('&')
    .filter((pair) => pair !== '')
    .map((pair, i) => {
      const eq = pair.indexOf('=');
      const key = eq >= 0 ? pair.slice(0, eq) : pair;
      const value = eq >= 0 ? pair.slice(eq + 1) : '';
      return { id: `urlp_${i}`, enabled: true, key: decodeFormComponent(key), value: decodeFormComponent(value) };
    });
}

export function serializeUrlEncoded(params: UrlEncodedParam[]): string {
  return params
    .filter((p) => p.enabled && p.key.trim())
    .map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`)
    .join('&');
}

// --- multipart -----------------------------------------------------------

const dispositionParam = (headers: string, name: string): string | undefined => {
  const m = headers.match(new RegExp(`;\\s*${name}="((?:[^"\\\\]|\\\\.)*)"`, 'i')) ?? headers.match(new RegExp(`;\\s*${name}=([^;\\r\\n]+)`, 'i'));
  return m ? m[1] : undefined;
};

export function parseMultipart(body: string, boundary: string): MultipartField[] {
  const raw = bytesToLatin1(bodyToBytes(body));
  const delimiter = `--${boundary}`;
  const fields: MultipartField[] = [];

  const sections = raw.split(delimiter).slice(1);
  for (const section of sections) {
    if (section.startsWith('--')) break;
    const part = section.replace(/^\r?\n/, '').replace(/\r?\n$/, '');
    const sep = part.search(/\r?\n\r?\n/);
    const rawHeaders = sep >= 0 ? part.slice(0, sep) : part;
    const content = sep >= 0 ? part.slice(sep).replace(/^\r?\n\r?\n/, '') : '';
    const headers = utf8OrNull(rawHeaders) ?? rawHeaders;

    const name = dispositionParam(headers, 'name') ?? '';
    const fileName = dispositionParam(headers, 'filename');
    const contentType = headers.match(/^content-type:\s*(.+)$/im)?.[1].trim();
    const text = fileName === undefined ? utf8OrNull(content) : null;

    fields.push(
      text !== null
        ? { id: `field_${fields.length}`, enabled: true, key: name, value: text, type: 'text', content_type: contentType }
        : {
            id: `field_${fields.length}`,
            enabled: true,
            key: name,
            value: `data:${contentType || 'application/octet-stream'};base64,${btoa(content)}`,
            type: 'base64',
            file_name: fileName ?? 'file',
            content_type: contentType || 'application/octet-stream',
          }
    );
  }
  return fields;
}

/** Fields must already be `text` or `base64`; resolve file paths before calling. */
export function serializeMultipart(fields: MultipartField[], boundary: string): string {
  const quote = (s: string) => s.replace(/"/g, '%22').replace(/\r?\n/g, ' ');
  let out = '';
  for (const f of fields) {
    if (!f.enabled || !f.key.trim()) continue;
    const isFile = f.type !== 'text';
    let disposition = `Content-Disposition: form-data; name="${quote(f.key)}"`;
    if (isFile) disposition += `; filename="${quote(f.file_name || 'file')}"`;
    const head = utf8ToLatin1(disposition + '\r\n' + (f.content_type ? `Content-Type: ${f.content_type}\r\n` : '') + '\r\n');
    const content = isFile ? atob(f.value.slice(f.value.indexOf(',') + 1)) : utf8ToLatin1(f.value);
    out += `--${boundary}\r\n${head}${content}\r\n`;
  }
  out += `--${boundary}--\r\n`;
  return bytesToBody(latin1ToBytes(out));
}

function utf8ToLatin1(s: string): string {
  return bytesToLatin1(new TextEncoder().encode(s));
}

// --- structured request bodies (Repeater / Fuzzer) ----------------------

/** Body type values understood by the Rust request sender. */
export const BODY_TYPE_OPTIONS = [
  { value: 'none', label: 'None' },
  { value: 'json', label: 'JSON' },
  { value: 'raw', label: 'Raw' },
  { value: 'form', label: 'Multipart' },
  { value: 'urlencoded', label: 'URL-Encoded' },
  { value: 'binary', label: 'Binary' },
];

export const isUrlEncodedType = (t: string) => t === 'urlencoded' || t === 'x-www-form-urlencoded';
export const isMultipartType = (t: string) => t === 'form' || t === 'form-data' || t === 'multipart';

export function readStoredUrlEncoded(content: string | undefined): UrlEncodedParam[] {
  try {
    const parsed = JSON.parse(content || '');
    if (Array.isArray(parsed)) return parsed;
  } catch {}
  return [];
}

export function readStoredMultipart(content: string | undefined): MultipartField[] {
  try {
    const parsed = JSON.parse(content || '');
    if (parsed && Array.isArray(parsed.__form_data)) return parsed.__form_data;
    if (Array.isArray(parsed)) return parsed;
  } catch {}
  return [];
}

export const storeUrlEncoded = (params: UrlEncodedParam[]) => JSON.stringify(params, null, 2);
export const storeMultipart = (fields: MultipartField[]) => JSON.stringify({ __form_data: fields }, null, 2);

/** Turns a captured raw body into the bodyType/bodyContent pair Repeater and Fuzzer edit. */
export function toStructuredBody(contentType: string, body: string): { bodyType: string; bodyContent: string } {
  const kind = detectBodyKind(contentType, body);
  switch (kind) {
    case 'none':
      return { bodyType: 'none', bodyContent: '' };
    case 'json':
      return { bodyType: 'json', bodyContent: tryPrettifyJson(body) };
    case 'urlencoded':
      return { bodyType: 'urlencoded', bodyContent: storeUrlEncoded(parseUrlEncoded(body)) };
    case 'multipart': {
      const boundary = multipartBoundary(contentType, body);
      return boundary
        ? { bodyType: 'form', bodyContent: storeMultipart(parseMultipart(body, boundary)) }
        : { bodyType: 'raw', bodyContent: body };
    }
    case 'binary':
      return body.startsWith(BINARY_PREFIX) ? { bodyType: 'binary', bodyContent: body } : { bodyType: 'raw', bodyContent: body };
    default:
      return { bodyType: 'raw', bodyContent: body };
  }
}

/** Carries content across a body type switch so the new editor doesn't open empty. */
export function convertBodyContent(from: string, to: string, content: string): string {
  if (from === to || !content.trim()) return content;
  const src = isUrlEncodedType(from) ? 'urlencoded' : isMultipartType(from) ? 'form' : from;
  const dst = isUrlEncodedType(to) ? 'urlencoded' : isMultipartType(to) ? 'form' : to;

  if (dst === 'urlencoded') {
    if (readStoredUrlEncoded(content).length) return content;
    if (src === 'form') return convertFormDataToUrlEncoded(content);
    if (src === 'json' && isJsonString(content)) return convertJsonToUrlEncoded(content);
    if (src === 'raw' && URLENCODED_RE.test(content.trim())) return storeUrlEncoded(parseUrlEncoded(content));
    return storeUrlEncoded([]);
  }
  if (dst === 'form') {
    if (readStoredMultipart(content).length) return content;
    if (src === 'urlencoded') return convertUrlEncodedToFormData(content);
    if (src === 'json' && isJsonString(content)) return convertJsonToFormData(content);
    const boundary = src === 'raw' ? multipartBoundary('', content) : null;
    return storeMultipart(boundary ? parseMultipart(content, boundary) : []);
  }
  if (src === 'urlencoded') return dst === 'json' ? convertUrlEncodedToJson(content) : serializeUrlEncoded(readStoredUrlEncoded(content));
  if (src === 'form') return content;
  if (dst === 'json') return tryPrettifyJson(content);
  return content;
}
