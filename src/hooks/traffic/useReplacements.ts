import { useState, useEffect, useCallback } from 'react';

export interface ReplacementsData {
  URL_REPLACEMENTS: Record<string, string>;
  HEADER_REPLACEMENTS: Record<string, string>;
  BODY_KEY_REPLACEMENTS: Record<string, string>;
  URL_PARAM_REPLACEMENTS: Record<string, string>;
  TEXT_REPLACEMENTS: Record<string, string>;
}

export interface OrderedReplacement {
  id: string;
  type: string;
  pattern: string;
  replacement: string;
  is_active: boolean;
  order_index: number;
}

const DEFAULT_REPLACEMENTS: ReplacementsData = {
  URL_REPLACEMENTS: {},
  HEADER_REPLACEMENTS: {},
  BODY_KEY_REPLACEMENTS: {},
  URL_PARAM_REPLACEMENTS: {},
  TEXT_REPLACEMENTS: {}
};

// Helper function for nested JSON body transformation
function transformObjectHelper(obj: unknown, bodyReplacements: Record<string, string>): unknown {
  if (obj === null || typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(item => transformObjectHelper(item, bodyReplacements));
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
    const lowerKey = key.toLowerCase();
    if (bodyReplacements[lowerKey]) {
      result[key] = bodyReplacements[lowerKey];
    } else if (typeof value === 'object') {
      result[key] = transformObjectHelper(value, bodyReplacements);
    } else {
      result[key] = value;
    }
  }
  return result;
}

export function useReplacements() {
  const [replacements, setReplacements] = useState<ReplacementsData>(DEFAULT_REPLACEMENTS);
  const [orderedReplacements, setOrderedReplacements] = useState<OrderedReplacement[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchReplacements = useCallback(async () => {
    try {
      const res = await fetch('/api/replacements');
      setError(null);
      if (!res.ok) throw new Error('Failed to fetch');
      const data = await res.json();
      if (data.grouped) {
        setReplacements(data.grouped);
        setOrderedReplacements(data.ordered || []);
      } else {
        setReplacements(data);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const saveReplacements = useCallback(async (data: ReplacementsData | OrderedReplacement[], incremental = false) => {
    setError(null);
    try {
      const res = await fetch(`/api/replacements${incremental ? '?incremental=true' : ''}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      if (!res.ok) throw new Error('Failed to save');
      const result = await res.json();
      if (result.success) {
        await fetchReplacements();
      }
      return result;
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : 'Unknown error';
      setError(errMsg);
      return { success: false, error: errMsg };
    }
  }, [fetchReplacements]);

  const updateOrder = useCallback(async (items: OrderedReplacement[]) => {
    setError(null);
    try {
      const res = await fetch('/api/replacements', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ items }),
      });
      if (!res.ok) throw new Error('Failed to update order');
      const result = await res.json();
      if (result.success) {
        setOrderedReplacements(items);
      }
      return result;
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : 'Unknown error';
      setError(errMsg);
      return { success: false, error: errMsg };
    }
  }, []);

  const deleteReplacement = useCallback(async (id: string) => {
    setError(null);
    try {
      const res = await fetch('/api/replacements', {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id }),
      });
      if (!res.ok) throw new Error('Failed to delete');
      await fetchReplacements();
      return { success: true };
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : 'Unknown error';
      setError(errMsg);
      return { success: false, error: errMsg };
    }
  }, [fetchReplacements]);

  // Apply functions
  const applyAllReplacements = useCallback((request: { url: string, headers: Record<string, string>, body: string }) => {
    let { url, headers, body } = request;

    // 1. Global Text Replacements (applied to URL, Headers, and Body as strings)
    for (const [pattern, replacement] of Object.entries(replacements.TEXT_REPLACEMENTS)) {
      url = url.replaceAll(pattern, replacement);
      
      // Handle structured body separately to avoid breaking JSON
      if (body.startsWith('{') && body.includes('\"__form_data\"')) {
        try {
          const parsed = JSON.parse(body);
          if (parsed.__form_data) {
            parsed.__form_data = (parsed.__form_data as any[]).map((item) => ({
              ...item,
              k: (item.k || "").replaceAll(pattern, replacement),
              v: (item.v || "").replaceAll(pattern, replacement)
            }));
            body = JSON.stringify(parsed);
          }
        } catch {
          body = body.replaceAll(pattern, replacement);
        }
      } else {
        body = body.replaceAll(pattern, replacement);
      }

      const newHeaders: Record<string, string> = {};
      for (const [k, v] of Object.entries(headers)) {
        newHeaders[k] = v.replaceAll(pattern, replacement);
      }
      headers = newHeaders;
    }

    // 2. URL Replacements (String replacement on URL)
    for (const [pattern, replacement] of Object.entries(replacements.URL_REPLACEMENTS)) {
      url = url.replaceAll(pattern, replacement);
    }

    // 3. URL Param Replacements (Key-based)
    try {
      const parsedUrl = new URL(url);
      let searchParamsChanged = false;
      for (const [key, replacement] of Object.entries(replacements.URL_PARAM_REPLACEMENTS)) {
        if (parsedUrl.searchParams.has(key)) {
          parsedUrl.searchParams.set(key, replacement);
          searchParamsChanged = true;
        }
      }
      if (searchParamsChanged) url = parsedUrl.toString();
    } catch { /* skip if invalid URL */ }

    // 4. Header Replacements (Key-based)
    const updatedHeaders = { ...headers };
    for (const [k, v] of Object.entries(replacements.HEADER_REPLACEMENTS)) {
      // Find matching key case-insensitively
      const actualKey = Object.keys(updatedHeaders).find(key => key.toLowerCase() === k.toLowerCase());
      if (actualKey) {
        updatedHeaders[actualKey] = v;
      }
    }
    headers = updatedHeaders;

    // 5. Body Replacements (Key-based)
    if (body) {
      try {
        // Handle JSON (including our internal __form_data structure)
        const parsed = JSON.parse(body);
        
        if (parsed.__form_data && Array.isArray(parsed.__form_data)) {
           // SPECIAL CASE: Structured form data
           parsed.__form_data = (parsed.__form_data as any[]).map((item) => {
             const lowerK = (item.k || "").toLowerCase();
             if (replacements.BODY_KEY_REPLACEMENTS[lowerK]) {
               return { ...item, v: replacements.BODY_KEY_REPLACEMENTS[lowerK] };
             }
             return item;
           });
           body = JSON.stringify(parsed, null, 2);
        } else {
           // Normal JSON
           const transformed = transformObjectHelper(parsed, replacements.BODY_KEY_REPLACEMENTS);
           body = JSON.stringify(transformed, null, 2);
        }
      } catch {
        // Handle Form Data
        if (body.includes('=') && (body.includes('&') || body.length > 0)) {
           const params = new URLSearchParams(body);
           let changed = false;
           for (const [k, v] of Object.entries(replacements.BODY_KEY_REPLACEMENTS)) {
             if (params.has(k)) {
               params.set(k, v);
               changed = true;
             }
           }
           if (changed) body = params.toString();
        }
      }
    }

    return { url, headers, body };
  }, [replacements]);

  useEffect(() => {
    fetchReplacements();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    replacements,
    orderedReplacements,
    isLoading,
    error,
    fetchReplacements,
    saveReplacements,
    updateOrder,
    deleteReplacement,
    applyAllReplacements,
  };
}
