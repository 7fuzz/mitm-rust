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

export const formToJson = (body: string, contentType: string): string | null => {
  try {
    // 1. Handle our internal structured multipart
    if (body.startsWith('{') && body.includes('"__form_data"')) {
      const parsed = JSON.parse(body);
      if (parsed.__form_data) {
        const hasFiles = (parsed.__form_data as FormEntry[]).some(e => e.type === 'file');
        if (hasFiles) throw new Error("Cannot convert form with files to JSON");
        
        const result: Record<string, unknown> = {};
        (parsed.__form_data as FormEntry[]).forEach(e => {
          if (e.k) result[e.k] = e.v;
        });
        return JSON.stringify(result, null, 2);
      }
    }

    // 2. Handle urlencoded
    if (contentType.includes('x-www-form-urlencoded')) {
      const params = new URLSearchParams(body);
      const result: Record<string, unknown> = {};
      params.forEach((v, k) => {
        result[k] = v;
      });
      return JSON.stringify(result, null, 2);
    }

    // 3. Handle raw multipart parsing (best effort)
    if (contentType.includes('multipart/form-data')) {
        const boundaryMatch = contentType.match(/boundary=(?:"([^"]+)"|([^;]+))/i);
        const boundary = boundaryMatch ? (boundaryMatch[1] || boundaryMatch[2]) : '';
        if (!boundary) return null;

        const result: Record<string, unknown> = {};
        const parts = body.split(`--${boundary}`);
        
        for (const part of parts) {
            if (part.includes('filename=')) throw new Error("Cannot convert form with files to JSON");
            if (part.includes('name=')) {
                const nameMatch = part.match(/name="([^"]+)"/);
                const valueParts = part.split('\r\n\r\n');
                if (nameMatch && valueParts.length > 1) {
                    const k = nameMatch[1];
                    const v = valueParts[1].replace(/\r\n--.*$/, '').replace(/\r\n$/, '');
                    result[k] = v;
                }
            }
        }
        return JSON.stringify(result, null, 2);
    }

    return null;
  } catch (err) {
    console.error("Conversion error:", err);
    return null;
  }
};
