import type { ParamItem } from '../services/tauri/bridge';

/**
 * Safely encodes a parameter key or value for inclusion in a query string.
 * Preserves template variables like {{VAR}} so they aren't mangled by percent-encoding.
 */
export function encodeParamSafely(str: string): string {
  if (!str) return '';
  const placeholders: string[] = [];
  const protectedStr = str.replace(/\{\{[^{}]*\}\}/g, (match) => {
    placeholders.push(match);
    return `___VARTAG_${placeholders.length - 1}___`;
  });

  let encoded = encodeURIComponent(protectedStr);

  placeholders.forEach((val, idx) => {
    encoded = encoded.replace(new RegExp(`___VARTAG_${idx}___`, 'g'), val);
  });

  return encoded;
}

/**
 * Safely decodes a parameter key or value from a query string.
 * Converts '+' to space and tolerates malformed URI sequences.
 */
export function decodeParamSafely(str: string): string {
  if (!str) return '';
  try {
    return decodeURIComponent(str.replace(/\+/g, ' '));
  } catch {
    return str;
  }
}

/**
 * Extracts the base URL (protocol, host, path) by stripping any query string.
 */
export function extractBaseUrl(url: string): string {
  const qIndex = url.indexOf('?');
  return qIndex === -1 ? url : url.slice(0, qIndex);
}

/**
 * Parses query parameters from a URL string into an array of ParamItem objects.
 * Preserves any disabled parameters from `existingParams` whose keys are not in the new query string.
 */
export function parseUrlQueryParams(url: string, existingParams: ParamItem[] = []): ParamItem[] {
  const qIndex = url.indexOf('?');
  const disabledParams = existingParams.filter((p) => !p.enabled);

  if (qIndex === -1) {
    return disabledParams;
  }

  const queryString = url.slice(qIndex + 1);
  if (!queryString.trim()) {
    return disabledParams;
  }

  const pairs = queryString.split('&');
  const newParams: ParamItem[] = [];

  for (let i = 0; i < pairs.length; i++) {
    const pair = pairs[i];
    if (pair === '' && i === pairs.length - 1) {
      continue;
    }
    const eqIndex = pair.indexOf('=');
    let key: string;
    let value: string;

    if (eqIndex === -1) {
      key = decodeParamSafely(pair);
      value = '';
    } else {
      key = decodeParamSafely(pair.slice(0, eqIndex));
      value = decodeParamSafely(pair.slice(eqIndex + 1));
    }

    const existing = existingParams.find(
      (p) => p.key === key && p.value === value && !newParams.some((np) => np.id === p.id)
    );

    newParams.push({
      id: existing ? existing.id : `p-${Date.now()}-${Math.random().toString(36).substring(2, 6)}-${i}`,
      key,
      value,
      enabled: true,
    });
  }

  // Preserve disabled parameters whose keys are not in the new active query
  const remainingDisabled = disabledParams.filter(
    (dp) => !newParams.some((np) => np.key.trim() !== '' && np.key === dp.key)
  );

  return [...newParams, ...remainingDisabled];
}

/**
 * Builds a complete URL by appending enabled parameters as a query string to the base URL.
 * If there are no enabled parameters with content, returns the base URL without '?'.
 */
export function buildUrlWithParams(baseUrlOrUrl: string, params: ParamItem[]): string {
  const baseUrl = extractBaseUrl(baseUrlOrUrl);
  const activeParams = params.filter(
    (p) => p.enabled && (p.key.trim() !== '' || p.value.trim() !== '')
  );

  if (activeParams.length === 0) {
    return baseUrl;
  }

  const queryParts = activeParams.map((p) => {
    const encKey = encodeParamSafely(p.key);
    const encVal = encodeParamSafely(p.value);
    if (p.value !== '') {
      return `${encKey}=${encVal}`;
    }
    return `${encKey}=`;
  });

  return `${baseUrl}?${queryParts.join('&')}`;
}
