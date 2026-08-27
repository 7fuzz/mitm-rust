import { isTauriAvailable } from '../services/tauri/ipc';
import { readFileAsBase64 } from '../services/tauri/bridge';
import type { MultipartField, UrlEncodedParam } from '../types';

function generateId(): string {
  return Math.random().toString(36).substring(2, 9);
}

/**
 * Converts JSON string into Multipart (form-data) JSON string.
 */
export function convertJsonToFormData(jsonStr: string): string {
  try {
    const parsed = JSON.parse(jsonStr);
    const fields: MultipartField[] = [];

    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      for (const [key, val] of Object.entries(parsed)) {
        const valStr = typeof val === 'object' ? JSON.stringify(val) : String(val ?? '');
        const isBase64 = valStr.startsWith('data:') && valStr.includes(';base64,');
        fields.push({
          id: generateId(),
          key,
          value: valStr,
          type: isBase64 ? 'base64' : 'text',
          enabled: true,
        });
      }
    }

    return JSON.stringify({ __form_data: fields }, null, 2);
  } catch (err) {
    console.error('Failed to convert JSON to Form-Data:', err);
    return JSON.stringify({ __form_data: [] }, null, 2);
  }
}

/**
 * Converts JSON string into UrlEncoded parameters JSON string.
 */
export function convertJsonToUrlEncoded(jsonStr: string): string {
  try {
    const parsed = JSON.parse(jsonStr);
    const params: UrlEncodedParam[] = [];

    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      for (const [key, val] of Object.entries(parsed)) {
        const valStr = typeof val === 'object' ? JSON.stringify(val) : String(val ?? '');
        params.push({
          id: generateId(),
          key,
          value: valStr,
          enabled: true,
        });
      }
    }

    return JSON.stringify(params, null, 2);
  } catch (err) {
    console.error('Failed to convert JSON to UrlEncoded:', err);
    return JSON.stringify([], null, 2);
  }
}

/**
 * Converts Multipart (form-data) JSON string into pretty JSON string.
 * Auto-converts files to Base64 data URLs if file path is provided.
 */
export async function convertFormDataToJson(formDataJson: string): Promise<string> {
  const resultObj: Record<string, any> = {};

  try {
    const parsed = JSON.parse(formDataJson);
    const fields: MultipartField[] = Array.isArray(parsed)
      ? parsed
      : Array.isArray(parsed?.__form_data)
      ? parsed.__form_data
      : [];

    for (const field of fields) {
      if (!field.enabled || !field.key.trim()) continue;

      if ((field.type === 'file_path' || field.type === 'base64') && field.value.trim()) {
        const pathOrUrl = field.value.trim();
        if (pathOrUrl.startsWith('data:') && pathOrUrl.includes(';base64,')) {
          resultObj[field.key] = pathOrUrl;
        } else if (isTauriAvailable()) {
          try {
            const b64DataUrl = await readFileAsBase64(pathOrUrl);
            resultObj[field.key] = b64DataUrl;
          } catch {
            resultObj[field.key] = pathOrUrl;
          }
        } else {
          resultObj[field.key] = pathOrUrl;
        }
      } else {
        // Try parsing nested JSON if stringified JSON
        let val: any = field.value;
        try {
          if (typeof val === 'string' && ((val.startsWith('{') && val.endsWith('}')) || (val.startsWith('[') && val.endsWith(']')))) {
            val = JSON.parse(val);
          }
        } catch {}
        resultObj[field.key] = val;
      }
    }
  } catch (err) {
    console.error('Failed to convert Form-Data to JSON:', err);
  }

  return JSON.stringify(resultObj, null, 2);
}

/**
 * Converts UrlEncoded parameters JSON string into pretty JSON string.
 */
export function convertUrlEncodedToJson(urlEncodedJson: string): string {
  const resultObj: Record<string, any> = {};

  try {
    const params: UrlEncodedParam[] = JSON.parse(urlEncodedJson);
    if (Array.isArray(params)) {
      for (const param of params) {
        if (!param.enabled || !param.key.trim()) continue;
        let val: any = param.value;
        try {
          if ((val.startsWith('{') && val.endsWith('}')) || (val.startsWith('[') && val.endsWith(']'))) {
            val = JSON.parse(val);
          }
        } catch {}
        resultObj[param.key] = val;
      }
    }
  } catch (err) {
    console.error('Failed to convert UrlEncoded to JSON:', err);
  }

  return JSON.stringify(resultObj, null, 2);
}

/**
 * Converts Raw text to JSON.
 */
export function convertRawToJson(rawStr: string): string {
  try {
    const parsed = JSON.parse(rawStr);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return JSON.stringify({ rawContent: rawStr }, null, 2);
  }
}

/**
 * Converts Form-Data to UrlEncoded.
 */
export function convertFormDataToUrlEncoded(formDataJson: string): string {
  try {
    const parsed = JSON.parse(formDataJson);
    const fields: MultipartField[] = Array.isArray(parsed)
      ? parsed
      : Array.isArray(parsed?.__form_data)
      ? parsed.__form_data
      : [];

    const params: UrlEncodedParam[] = fields
      .filter((f) => f.enabled && f.key.trim())
      .map((f) => ({
        id: generateId(),
        key: f.key,
        value: f.value,
        enabled: true,
      }));

    return JSON.stringify(params, null, 2);
  } catch {
    return JSON.stringify([], null, 2);
  }
}

/**
 * Converts UrlEncoded to Form-Data.
 */
export function convertUrlEncodedToFormData(urlEncodedJson: string): string {
  try {
    const params: UrlEncodedParam[] = JSON.parse(urlEncodedJson);
    const fields: MultipartField[] = (Array.isArray(params) ? params : [])
      .filter((p) => p.enabled && p.key.trim())
      .map((p) => ({
        id: generateId(),
        key: p.key,
        value: p.value,
        type: 'text',
        enabled: true,
      }));

    return JSON.stringify({ __form_data: fields }, null, 2);
  } catch {
    return JSON.stringify({ __form_data: [] }, null, 2);
  }
}
