/**
 * Utility for converting request bodies between different formats
 */

export interface FormEntry {
  id?: string;
  k: string;
  v: string;
  type: 'text' | 'file';
  fileName?: string;
  contentType?: string;
}

export const jsonToUrlEncoded = (jsonString: string): string | null => {
  try {
    const obj = JSON.parse(jsonString);
    if (typeof obj !== 'object' || obj === null) return null;
    
    const params = new URLSearchParams();
    Object.entries(obj).forEach(([k, v]) => {
      params.append(k, typeof v === 'object' ? JSON.stringify(v) : String(v));
    });
    return params.toString();
  } catch {
    return null;
  }
};

export const jsonToMultipartStructured = (jsonString: string): string | null => {
  try {
    const obj = JSON.parse(jsonString);
    if (typeof obj !== 'object' || obj === null) return null;
    
    const entries: FormEntry[] = Object.entries(obj).map(([k, v]) => ({
      k,
      v: typeof v === 'object' ? JSON.stringify(v) : String(v),
      type: 'text'
    }));

    return JSON.stringify({
      __form_data: entries,
      _hint: "Converted from JSON"
    });
  } catch {
    return null;
  }
};

export const formToJson = (body: string, contentType: string = ''): string | null => {
  if (!body || body.trim() === '') return JSON.stringify({}, null, 2);
  const trimmed = body.trim();

  try {
    // 1. Check if body is ALREADY valid JSON (e.g. {"a": 1} or [1,2,3])
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      const parsed = JSON.parse(trimmed);
      
      // If it's our internal structured multipart __form_data:
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed) && (parsed as any).__form_data) {
        const hasFiles = ((parsed as any).__form_data as FormEntry[]).some(e => e.type === 'file');
        if (hasFiles) throw new Error("Cannot convert form with files to JSON");
        
        const result: Record<string, unknown> = {};
        ((parsed as any).__form_data as FormEntry[]).forEach(e => {
          if (e.k) result[e.k] = e.v;
        });
        return JSON.stringify(result, null, 2);
      }

      return JSON.stringify(parsed, null, 2);
    }

    // 2. Try URL-encoded params (e.g. "a=1&b=2" or "foo=bar")
    if (contentType.includes('x-www-form-urlencoded') || trimmed.includes('=')) {
      if (trimmed.includes('=')) {
        const params = new URLSearchParams(trimmed);
        const result: Record<string, unknown> = {};
        let count = 0;
        params.forEach((v, k) => {
          if (k.trim() !== '') {
            result[k] = v;
            count++;
          }
        });
        if (count > 0) {
          return JSON.stringify(result, null, 2);
        }
      }
    }

    // 3. Handle raw multipart parsing (best effort)
    if (contentType.includes('multipart/form-data')) {
      const boundaryMatch = contentType.match(/boundary=(?:"([^"]+)"|([^;]+))/i);
      const boundary = boundaryMatch ? (boundaryMatch[1] || boundaryMatch[2]) : '';
      if (boundary) {
        const result: Record<string, unknown> = {};
        const parts = trimmed.split(`--${boundary}`);
        
        for (const part of parts) {
          if (part.includes('filename=')) throw new Error("Cannot convert form with files to JSON");
          if (part.includes('name=')) {
            const nameMatch = part.match(/name="([^"]+)"/);
            const valueParts = part.split(/\r?\n\r?\n/);
            if (nameMatch && valueParts.length > 1) {
              const k = nameMatch[1];
              const v = valueParts[1].replace(/\r?\n--.*$/, '').replace(/\r?\n$/, '');
              result[k] = v;
            }
          }
        }
        return JSON.stringify(result, null, 2);
      }
    }

    // 4. Line-by-line key: value or key = value fallback (e.g. multiline text)
    const lines = trimmed.split(/\r?\n/).filter(l => l.trim() !== '');
    const lineObj: Record<string, unknown> = {};
    let matchedLines = 0;
    for (const line of lines) {
      const colonIdx = line.indexOf(':');
      const eqIdx = line.indexOf('=');
      if (colonIdx > 0 && (eqIdx === -1 || colonIdx < eqIdx)) {
        const k = line.slice(0, colonIdx).trim();
        const v = line.slice(colonIdx + 1).trim();
        if (k) { lineObj[k] = v; matchedLines++; }
      } else if (eqIdx > 0) {
        const k = line.slice(0, eqIdx).trim();
        const v = line.slice(eqIdx + 1).trim();
        if (k) { lineObj[k] = v; matchedLines++; }
      }
    }
    if (matchedLines > 0) {
      return JSON.stringify(lineObj, null, 2);
    }

    // 5. Fallback: Wrap raw string into {"data": rawString}
    return JSON.stringify({ data: trimmed }, null, 2);
  } catch (err) {
    console.error("Conversion error:", err);
    return null;
  }
};
