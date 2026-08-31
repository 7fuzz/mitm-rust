import { detectBase64 } from './base64Helper';

export interface MediaDetectionResult {
  isMedia: boolean;
  previewType: 'image' | 'audio' | 'video' | 'pdf' | 'svg';
  mimeType: string;
  extension: string;
  dataUri: string;
  cleanB64?: string;
  sizeBytes?: number;
}

interface HeaderItemLike {
  key?: string;
  name?: string;
  value?: string;
}

export function detectMediaResponse(
  body: string | undefined | null,
  headers?: HeaderItemLike[] | Record<string, string>
): MediaDetectionResult | null {
  if (!body && !headers) return null;
  const rawBody = (body || '').trim();

  // 1. Extract Content-Type header if present
  let contentType = '';
  if (Array.isArray(headers)) {
    const ctHeader = headers.find(
      (h) => (h.key || (h as any).name || '').toLowerCase() === 'content-type'
    );
    if (ctHeader && ctHeader.value) {
      contentType = ctHeader.value.split(';')[0].trim().toLowerCase();
    }
  } else if (headers && typeof headers === 'object') {
    for (const [k, v] of Object.entries(headers)) {
      if (k.toLowerCase() === 'content-type' && typeof v === 'string') {
        contentType = v.split(';')[0].trim().toLowerCase();
        break;
      }
    }
  }

  // 2. Check if body is an SVG string
  if (
    contentType === 'image/svg+xml' ||
    (rawBody.startsWith('<svg') && rawBody.includes('</svg>')) ||
    (rawBody.startsWith('<?xml') && rawBody.includes('<svg'))
  ) {
    const encodedSvg = encodeURIComponent(rawBody);
    const dataUri = `data:image/svg+xml;utf8,${encodedSvg}`;
    return {
      isMedia: true,
      previewType: 'svg',
      mimeType: 'image/svg+xml',
      extension: 'svg',
      dataUri,
      sizeBytes: rawBody.length,
    };
  }

  // 3. Check if body already has data: URI
  if (rawBody.startsWith('data:')) {
    const match = rawBody.match(/^data:([^;]+);base64,(.+)$/s);
    if (match) {
      const mime = match[1].toLowerCase();
      const b64 = match[2].trim();
      let previewType: MediaDetectionResult['previewType'] | null = null;
      let extension = 'bin';

      if (mime.startsWith('image/')) {
        previewType = 'image';
        extension = mime.split('/')[1] || 'png';
      } else if (mime.startsWith('audio/')) {
        previewType = 'audio';
        extension = mime.split('/')[1] || 'mp3';
      } else if (mime.startsWith('video/')) {
        previewType = 'video';
        extension = mime.split('/')[1] || 'mp4';
      } else if (mime === 'application/pdf') {
        previewType = 'pdf';
        extension = 'pdf';
      }

      if (previewType) {
        return {
          isMedia: true,
          previewType,
          mimeType: mime,
          extension,
          dataUri: rawBody,
          cleanB64: b64,
          sizeBytes: Math.round((b64.length * 3) / 4),
        };
      }
    }
  }

  // 4. Try base64 detection from body text (min length 20 chars)
  const b64Info = detectBase64(rawBody, 20);
  if (
    b64Info &&
    (b64Info.previewType === 'image' ||
      b64Info.previewType === 'audio' ||
      b64Info.previewType === 'video' ||
      b64Info.previewType === 'pdf')
  ) {
    const dataUri = `data:${b64Info.mimeType};base64,${b64Info.cleanB64}`;
    return {
      isMedia: true,
      previewType: b64Info.previewType,
      mimeType: b64Info.mimeType,
      extension: b64Info.extension,
      dataUri,
      cleanB64: b64Info.cleanB64,
      sizeBytes: b64Info.sizeBytes,
    };
  }

  // 5. Fallback check on Content-Type header when body is binary or raw string
  if (contentType) {
    let previewType: MediaDetectionResult['previewType'] | null = null;
    let extension = 'bin';

    if (contentType.startsWith('image/')) {
      previewType = 'image';
      extension = contentType.replace('image/', '') || 'png';
    } else if (contentType.startsWith('audio/')) {
      previewType = 'audio';
      extension = contentType.replace('audio/', '') || 'mp3';
    } else if (contentType.startsWith('video/')) {
      previewType = 'video';
      extension = contentType.replace('video/', '') || 'mp4';
    } else if (contentType === 'application/pdf') {
      previewType = 'pdf';
      extension = 'pdf';
    }

    if (previewType && rawBody.length > 0) {
      // If body looks like base64 or raw
      let cleanB64 = rawBody.replace(/\s/g, '');
      let dataUri = '';

      // Test if rawBody is valid base64
      if (/^[A-Za-z0-9+/=]+$/.test(cleanB64) || /^[A-Za-z0-9-_=]+$/.test(cleanB64)) {
        const normalized = cleanB64.replace(/-/g, '+').replace(/_/g, '/');
        const padded = normalized.padEnd(normalized.length + ((4 - (normalized.length % 4)) % 4), '=');
        dataUri = `data:${contentType};base64,${padded}`;
        cleanB64 = padded;
      } else {
        // Raw binary string: convert to base64 via btoa with binary character encoding
        try {
          const b64 = btoa(
            encodeURIComponent(rawBody).replace(/%([0-9A-F]{2})/g, (_, p1) =>
              String.fromCharCode(parseInt(p1, 16))
            )
          );
          dataUri = `data:${contentType};base64,${b64}`;
          cleanB64 = b64;
        } catch {
          dataUri = `data:${contentType};base64,${btoa(rawBody)}`;
          cleanB64 = btoa(rawBody);
        }
      }

      return {
        isMedia: true,
        previewType,
        mimeType: contentType,
        extension,
        dataUri,
        cleanB64,
        sizeBytes: rawBody.length,
      };
    }
  }

  return null;
}
