// Helper utility for detecting, decoding, previewing, and saving base64 content

export interface Base64Info {
  isBase64: boolean;
  cleanB64: string;
  mimeType: string;
  extension: string;
  sizeBytes: number;
  previewType: 'image' | 'pdf' | 'audio' | 'video' | 'text' | 'json' | 'binary';
}

export function detectBase64(str: unknown): Base64Info | null {
  if (typeof str !== 'string') return null;
  let raw = str.trim();
  if (!raw) return null;

  let mimeType = 'application/octet-stream';
  let cleanB64 = raw;

  // Prefix checks
  if (raw.startsWith('base64:')) {
    cleanB64 = raw.substring(7).trim();
  } else if (raw.startsWith('data:')) {
    const match = raw.match(/^data:([^;]+);base64,(.+)$/s);
    if (match) {
      mimeType = match[1];
      cleanB64 = match[2].trim();
    } else {
      return null;
    }
  }

  // Remove whitespace
  cleanB64 = cleanB64.replace(/\s/g, '');

  // Must be valid length and base64 charset
  if (cleanB64.length < 16) return null;
  if (!/^[A-Za-z0-9+/=]+$/.test(cleanB64) && !/^[A-Za-z0-9-_=]+$/.test(cleanB64)) {
    return null;
  }

  try {
    // Attempt decode check
    const normalized = cleanB64.replace(/-/g, '+').replace(/_/g, '/');
    const padded = normalized.padEnd(normalized.length + ((4 - (normalized.length % 4)) % 4), '=');
    const binaryString = atob(padded);
    const sizeBytes = binaryString.length;

    // Detect MIME type and extension from magic bytes if not already set via data: URI
    let extension = 'bin';
    let previewType: Base64Info['previewType'] = 'binary';

    const headerHex = Array.from(binaryString.slice(0, 16))
      .map(c => c.charCodeAt(0).toString(16).padStart(2, '0'))
      .join('')
      .toLowerCase();

    if (headerHex.startsWith('89504e47')) {
      mimeType = 'image/png';
      extension = 'png';
      previewType = 'image';
    } else if (headerHex.startsWith('ffd8ff')) {
      mimeType = 'image/jpeg';
      extension = 'jpg';
      previewType = 'image';
    } else if (headerHex.startsWith('47494638')) {
      mimeType = 'image/gif';
      extension = 'gif';
      previewType = 'image';
    } else if (headerHex.startsWith('52494646') && headerHex.includes('57454250')) {
      mimeType = 'image/webp';
      extension = 'webp';
      previewType = 'image';
    } else if (headerHex.startsWith('25504446')) {
      mimeType = 'application/pdf';
      extension = 'pdf';
      previewType = 'pdf';
    } else if (headerHex.startsWith('504b0304')) {
      mimeType = 'application/zip';
      extension = 'zip';
      previewType = 'binary';
    } else if (headerHex.startsWith('1f8b')) {
      mimeType = 'application/gzip';
      extension = 'gz';
      previewType = 'binary';
    } else if (headerHex.startsWith('4f676753')) {
      mimeType = 'audio/ogg';
      extension = 'ogg';
      previewType = 'audio';
    } else if (headerHex.startsWith('494433') || headerHex.startsWith('fffb') || headerHex.startsWith('fff3')) {
      mimeType = 'audio/mp3';
      extension = 'mp3';
      previewType = 'audio';
    } else if (headerHex.startsWith('0000001866747970') || headerHex.startsWith('0000002066747970') || headerHex.startsWith('0000001c66747970')) {
      mimeType = 'video/mp4';
      extension = 'mp4';
      previewType = 'video';
    } else {
      // Check printable text / JSON
      let isText = true;
      for (let i = 0; i < Math.min(binaryString.length, 512); i++) {
        const code = binaryString.charCodeAt(i);
        if (code < 9 || (code > 13 && code < 32) || code === 127) {
          isText = false;
          break;
        }
      }
      if (isText) {
        const trimmed = binaryString.trim();
        if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
          try {
            JSON.parse(trimmed);
            mimeType = 'application/json';
            extension = 'json';
            previewType = 'json';
          } catch {
            mimeType = 'text/plain';
            extension = 'txt';
            previewType = 'text';
          }
        } else {
          mimeType = 'text/plain';
          extension = 'txt';
          previewType = 'text';
        }
      }
    }

    if (mimeType.startsWith('image/')) previewType = 'image';
    else if (mimeType.startsWith('audio/')) previewType = 'audio';
    else if (mimeType.startsWith('video/')) previewType = 'video';

    return {
      isBase64: true,
      cleanB64: padded,
      mimeType,
      extension,
      sizeBytes,
      previewType,
    };
  } catch {
    return null;
  }
}

export async function saveBase64ToFile(
  base64Data: string,
  suggestedName: string = 'download',
  mimeType: string = 'application/octet-stream'
): Promise<{ success: boolean; path?: string; cancelled?: boolean }> {
  let cleanB64 = base64Data.trim();
  if (cleanB64.startsWith('base64:')) cleanB64 = cleanB64.substring(7).trim();
  if (cleanB64.includes(';base64,')) cleanB64 = cleanB64.split(';base64,')[1].trim();
  cleanB64 = cleanB64.replace(/\s/g, '');

  const normalized = cleanB64.replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized.padEnd(normalized.length + ((4 - (normalized.length % 4)) % 4), '=');

  const binaryString = atob(padded);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }

  // Attempt Tauri native save dialog first
  try {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { writeFile } = await import('@tauri-apps/plugin-fs');

    const ext = suggestedName.includes('.') ? suggestedName.split('.').pop()! : 'bin';
    const filePath = await save({
      defaultPath: suggestedName,
      filters: [{ name: 'File', extensions: [ext, '*'] }]
    });

    if (filePath) {
      await writeFile(filePath, bytes);
      return { success: true, path: filePath };
    }
    return { success: false, cancelled: true };
  } catch {
    // Fallback to browser download link
    const blob = new Blob([bytes], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = suggestedName;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return { success: true };
  }
}
