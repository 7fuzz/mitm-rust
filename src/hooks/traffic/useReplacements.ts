import { useState, useEffect, useCallback } from 'react';
import { SyncData, ReplacementEntry, ReplacementCategory } from './types';
import { invoke } from '@tauri-apps/api/core';

export interface ReplacementsData {
  URL_REPLACEMENTS: Record<string, string>;
  HEADER_REPLACEMENTS: Record<string, string>;
  BODY_KEY_REPLACEMENTS: Record<string, string>;
  URL_PARAM_REPLACEMENTS: Record<string, string>;
  TEXT_REPLACEMENTS: Record<string, string>;
}

const DEFAULT_REPLACEMENTS: ReplacementsData = {
  URL_REPLACEMENTS: {},
  HEADER_REPLACEMENTS: {},
  BODY_KEY_REPLACEMENTS: {},
  URL_PARAM_REPLACEMENTS: {},
  TEXT_REPLACEMENTS: {}
};

// Helper function for nested JSON body transformation
function transformObjectHelper(obj: any, bodyReplacements: Record<string, string>): any {
  if (obj === null || typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(item => transformObjectHelper(item, bodyReplacements));
  const result: Record<string, any> = {};
  for (const [key, value] of Object.entries(obj)) {
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
  const [orderedReplacements, setOrderedReplacements] = useState<ReplacementEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const _setRawReplacements = (grouped: ReplacementsData, ordered: ReplacementEntry[]) => {
    setReplacements(grouped);
    setOrderedReplacements(ordered);
    setIsLoading(false);
  };

  const fetchReplacements = useCallback(async () => {
    try {
      const data = await invoke<SyncData>('sync_data');
      
      const grouped: ReplacementsData = {
        URL_REPLACEMENTS: {},
        HEADER_REPLACEMENTS: {},
        BODY_KEY_REPLACEMENTS: {},
        URL_PARAM_REPLACEMENTS: {},
        TEXT_REPLACEMENTS: {}
      };

      data.replacements.forEach(r => {
        const cat = r.type;
        if (r.is_active && grouped[cat]) {
          grouped[cat][r.pattern] = r.replacement;
        }
      });

      setReplacements(grouped);
      setOrderedReplacements(data.replacements);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const saveReplacements = useCallback(async (data: ReplacementsData | ReplacementEntry[]) => {
    setError(null);
    try {
      let items: ReplacementEntry[] = [];
      if (Array.isArray(data)) {
        items = data;
      } else {
        (Object.keys(data) as ReplacementCategory[]).forEach((type) => {
          const patterns = data[type];
          Object.entries(patterns).forEach(([pattern, replacement]) => {
            items.push({ 
              id: crypto.randomUUID(), 
              type, 
              pattern, 
              replacement, 
              is_active: true, 
              order_index: 0 
            });
          });
        });
      }

      await invoke('save_replacements_bulk', { replacements: items });
      await fetchReplacements();
      return { success: true };
    } catch (err) {
      const errMsg = String(err);
      setError(errMsg);
      return { success: false, error: errMsg };
    }
  }, [fetchReplacements]);

  const updateOrder = useCallback(async (items: ReplacementEntry[]) => {
    setError(null);
    try {
      await invoke('update_replacement_order', { items });
      setOrderedReplacements(items);
      return { success: true };
    } catch (e) {
      const errMsg = String(e);
      setError(errMsg);
      return { success: false, error: errMsg };
    }
  }, []);

  const deleteReplacement = useCallback(async (id: string) => {
    setError(null);
    try {
      await invoke('delete_replacement', { id });
      await fetchReplacements();
      return { success: true };
    } catch (e) {
      const errMsg = String(e);
      setError(errMsg);
      return { success: false, error: errMsg };
    }
  }, [fetchReplacements]);

  // Apply functions
  const applyAllReplacements = useCallback((request: { url: string, headers: [string, string][], body: string }) => {
    let { url, headers, body } = request;

    // 1. Global Text Replacements
    for (const [pattern, replacement] of Object.entries(replacements.TEXT_REPLACEMENTS)) {
      url = url.replaceAll(pattern, replacement);
      
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

      headers = headers.map(([k, v]) => [k, v.replaceAll(pattern, replacement)]);
    }

    // 2. URL Replacements
    for (const [pattern, replacement] of Object.entries(replacements.URL_REPLACEMENTS)) {
      url = url.replaceAll(pattern, replacement);
    }

    // 3. URL Param Replacements
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

    // 4. Header Replacements
    headers = headers.map(([k, v]) => {
      const lowerK = k.toLowerCase();
      for (const [pattern, replacement] of Object.entries(replacements.HEADER_REPLACEMENTS)) {
        if (lowerK === pattern.toLowerCase()) {
          return [k, replacement];
        }
      }
      return [k, v];
    });

    // 5. Body Replacements
    if (body) {
      try {
        const parsed = JSON.parse(body);
        
        if (parsed.__form_data && Array.isArray(parsed.__form_data)) {
           parsed.__form_data = (parsed.__form_data as any[]).map((item) => {
             const lowerK = (item.k || "").toLowerCase();
             if (replacements.BODY_KEY_REPLACEMENTS[lowerK]) {
               return { ...item, v: replacements.BODY_KEY_REPLACEMENTS[lowerK] };
             }
             return item;
           });
           body = JSON.stringify(parsed, null, 2);
        } else {
           const transformed = transformObjectHelper(parsed, replacements.BODY_KEY_REPLACEMENTS);
           body = JSON.stringify(transformed, null, 2);
        }
      } catch {
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
  }, [fetchReplacements]);

  return {
    replacements,
    orderedReplacements,
    isLoading,
    error,
    _setRawReplacements,
    fetchReplacements,
    saveReplacements,
    updateOrder,
    deleteReplacement,
    applyAllReplacements,
  };
}
