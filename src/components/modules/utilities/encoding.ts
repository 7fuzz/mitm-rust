import CryptoJS from 'crypto-js';
import type { Direction, EncodingAlgorithm } from '../../../stores/useUtilitiesStore';

export type HexDelimiter = '' | ' ' | ':' | '\\x' | '0x';

export interface EncodingOptions {
  algorithm: EncodingAlgorithm;
  direction: Direction;
  hexDelimiter: HexDelimiter;
  hexCase: 'lower' | 'upper';
  urlFull: boolean;
}

export interface JwtParts {
  header: string;
  payload: string;
  signature: string;
  claims: Record<string, unknown> | null;
}

export interface TransformResult {
  output: string;
  error: string | null;
  jwt?: JwtParts;
}

const bytesToBinaryString = (bytes: Uint8Array) => {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return binary;
};

const binaryStringToText = (binary: string) => {
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return new TextDecoder().decode(bytes);
};

const toBase64 = (text: string) => btoa(bytesToBinaryString(new TextEncoder().encode(text)));
const toBase64Url = (b64: string) => b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');

const padBase64 = (s: string) => {
  let out = s;
  while (out.length % 4 !== 0) out += '=';
  return out;
};

const ok = (output: string): TransformResult => ({ output, error: null });
const fail = (error: string): TransformResult => ({ output: '', error });

const HTML_ENTITIES: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

function encode(input: string, opts: EncodingOptions): TransformResult {
  switch (opts.algorithm) {
    case 'base64':
      return ok(toBase64(input));
    case 'base64url':
      return ok(toBase64Url(toBase64(input)));
    case 'url':
      if (opts.urlFull) {
        return ok(
          Array.from(new TextEncoder().encode(input))
            .map((b) => '%' + b.toString(16).toUpperCase().padStart(2, '0'))
            .join('')
        );
      }
      return ok(encodeURIComponent(input));
    case 'hex': {
      const parts = Array.from(new TextEncoder().encode(input)).map((b) => {
        const h = b.toString(16).padStart(2, '0');
        const cased = opts.hexCase === 'upper' ? h.toUpperCase() : h;
        return opts.hexDelimiter === '\\x' || opts.hexDelimiter === '0x' ? opts.hexDelimiter + cased : cased;
      });
      if (opts.hexDelimiter === '\\x') return ok(parts.join(''));
      if (opts.hexDelimiter === '0x') return ok(parts.join(' '));
      return ok(parts.join(opts.hexDelimiter));
    }
    case 'html':
      return ok(input.replace(/[&<>"']/g, (m) => HTML_ENTITIES[m]));
    case 'binary':
      return ok(
        Array.from(new TextEncoder().encode(input))
          .map((b) => b.toString(2).padStart(8, '0'))
          .join(' ')
      );
    case 'jwt': {
      let payload: unknown;
      try {
        payload = JSON.parse(input);
      } catch {
        return fail('Expected a JSON payload.');
      }
      const header = toBase64Url(toBase64(JSON.stringify({ alg: 'HS256', typ: 'JWT' })));
      const body = toBase64Url(toBase64(JSON.stringify(payload)));
      const signature = toBase64Url(CryptoJS.HmacSHA256(`${header}.${body}`, 'secret').toString(CryptoJS.enc.Base64));
      return ok(`${header}.${body}.${signature}`);
    }
  }
}

const decodeJwtSegment = (segment: string) => binaryStringToText(atob(padBase64(segment.replace(/-/g, '+').replace(/_/g, '/'))));

const prettyJson = (text: string) => {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
};

function decode(input: string, opts: EncodingOptions): TransformResult {
  switch (opts.algorithm) {
    case 'base64':
    case 'base64url': {
      let cleaned = input.trim().replace(/\s+/g, '');
      if (opts.algorithm === 'base64url') cleaned = cleaned.replace(/-/g, '+').replace(/_/g, '/');
      cleaned = padBase64(cleaned);
      if (!/^[A-Za-z0-9+/=]+$/.test(cleaned)) return fail('Contains characters outside the Base64 alphabet.');
      return ok(binaryStringToText(atob(cleaned)));
    }
    case 'url':
      try {
        return ok(decodeURIComponent(input.replace(/\+/g, '%20')));
      } catch (e: any) {
        return fail(e.message);
      }
    case 'hex': {
      const cleaned = input.trim().replace(/0x|\\x|[:,\s\-_]/gi, '');
      if (!cleaned) return ok('');
      if (cleaned.length % 2 !== 0) return fail(`Odd number of hex digits (${cleaned.length}).`);
      if (!/^[0-9a-fA-F]+$/.test(cleaned)) return fail('Contains non-hex characters.');
      const bytes = new Uint8Array(cleaned.length / 2);
      for (let i = 0; i < bytes.length; i++) bytes[i] = parseInt(cleaned.substr(i * 2, 2), 16);
      return ok(new TextDecoder().decode(bytes));
    }
    case 'html': {
      const doc = new DOMParser().parseFromString(input, 'text/html');
      return ok(doc.documentElement.textContent || '');
    }
    case 'binary': {
      const cleaned = input.replace(/[^01]/g, '');
      if (!cleaned) return ok('');
      if (cleaned.length % 8 !== 0) return fail(`${cleaned.length} bits is not a multiple of 8.`);
      const bytes = new Uint8Array(cleaned.length / 8);
      for (let i = 0; i < bytes.length; i++) bytes[i] = parseInt(cleaned.substr(i * 8, 8), 2);
      return ok(new TextDecoder().decode(bytes));
    }
    case 'jwt': {
      const parts = input.trim().split('.');
      if (parts.length < 2 || parts.length > 3) return fail('Expected header.payload.signature');
      const header = prettyJson(decodeJwtSegment(parts[0]));
      const payloadRaw = decodeJwtSegment(parts[1]);
      const payload = prettyJson(payloadRaw);
      let claims: Record<string, unknown> | null = null;
      try {
        const parsed = JSON.parse(payloadRaw);
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) claims = parsed;
      } catch {}
      const signature = parts[2] ?? '';
      return {
        output: `${header}\n\n${payload}${signature ? `\n\n${signature}` : ''}`,
        error: null,
        jwt: { header, payload, signature, claims },
      };
    }
  }
}

export function transform(input: string, opts: EncodingOptions): TransformResult {
  if (!input) return ok('');
  try {
    return opts.direction === 'encode' ? encode(input, opts) : decode(input, opts);
  } catch (err: any) {
    return fail(err.message || String(err));
  }
}

const SAMPLES: Record<EncodingAlgorithm, Record<Direction, string>> = {
  base64: { encode: 'MITM-Developer-Security-Suite-2.0', decode: 'TUlUTS1EZXZlbG9wZXItU2VjdXJpdHktU3VpdGUtMi4w' },
  base64url: { encode: 'MITM-Developer-Security-Suite-2.0', decode: 'TUlUTS1EZXZlbG9wZXItU2VjdXJpdHktU3VpdGUtMi4w' },
  url: {
    encode: 'https://api.internal.local/v1/auth?client_id=sec-suite&redirect=https://app.dev/#token',
    decode: 'https%3A%2F%2Fapi.internal.local%2Fv1%2Fauth%3Fclient_id%3Dsec-suite%26redirect%3Dhttps%3A%2F%2Fapp.dev%2F%23token',
  },
  hex: {
    encode: 'MITM-Developer-Security-Suite-2.0',
    decode: '4d 49 54 4d 2d 44 65 76 65 6c 6f 70 65 72 2d 53 65 63 75 72 69 74 79 2d 53 75 69 74 65 2d 32 2e 30',
  },
  html: {
    encode: '<div class="alert" data-user="admin & user">Notice: "Access Granted"</div>',
    decode: '&lt;div class=&quot;alert&quot; data-user=&quot;admin &amp; user&quot;&gt;Notice: &quot;Access Granted&quot;&lt;/div&gt;',
  },
  binary: { encode: 'MITM', decode: '01001101 01001001 01010100 01001101' },
  jwt: {
    encode: '{\n  "sub": "user-123",\n  "name": "Admin User",\n  "iat": 1700000000\n}',
    decode:
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTEyMyIsIm5hbWUiOiJBZG1pbiBVc2VyIiwicm9sZXMiOlsic2VjdXJpdHlfc3VpdGUiLCJhdWRpdG9yIl0sImlhdCI6MTcwMDAwMDAwMH0.u2fC9M14Y0T_9w7s4e0o3f8c5b1a2d6e7f8a9b0c1d2',
  },
};

export const sampleFor = (algorithm: EncodingAlgorithm, direction: Direction) => SAMPLES[algorithm][direction];
