import { useState, useCallback, useRef } from 'react';
import { Input, Textarea, Select } from '../ui';

interface UrlParam {
  id: string;
  k: string;
  v: string;
  enabled: boolean;
}

interface Props {
  method?: string;
  onMethodChange?: (m: string) => void;
  url: string;
  onChange?: (newUrl: string) => void;
  urlParams?: string; // JSON string of UrlParam[] for persistence (includes disabled params)
  onUrlParamsChange?: (params: string) => void;
  readOnly?: boolean;
}

export function UrlEditor({ method = 'GET', onMethodChange, url, onChange, urlParams, onUrlParamsChange, readOnly = false }: Props) {
  const [mode, setMode] = useState<'raw' | 'structured'>('raw');
  const [rawUrl, setRawUrl] = useState(url);
  const [prevUrl, setPrevUrl] = useState(url);
  const lastGeneratedUrl = useRef(url);

  const [domain, setDomain] = useState('');
  const [paths, setPaths] = useState<{ id: string; v: string }[]>([]);
  const [params, setParams] = useState<UrlParam[]>([]);
  const [fragment, setFragment] = useState('');
  const paramsInitialized = useRef(false);

  const parseUrlToStructured = useCallback((targetUrl: string, savedParams?: UrlParam[]) => {
    try {
      const parsed = new URL(targetUrl);
      setDomain(parsed.origin);

      const pathSegments = parsed.pathname
        .split('/')
        .filter(p => p !== '')
        .map(v => ({ id: crypto.randomUUID(), v }));
      setPaths(pathSegments);

      if (savedParams && savedParams.length > 0) {
        // Use saved params (includes disabled ones), but merge any new params from URL
        const urlSearchParams: { k: string; v: string }[] = [];
        parsed.searchParams.forEach((v, k) => urlSearchParams.push({ k, v }));

        // Start with saved params, then add any URL params that don't exist in saved
        const merged = [...savedParams];
        urlSearchParams.forEach(up => {
          const exists = merged.some(sp => sp.k === up.k && sp.v === up.v && sp.enabled);
          if (!exists) {
            // Check if there's a disabled param with same key - don't duplicate
            const disabledMatch = merged.find(sp => sp.k === up.k && !sp.enabled);
            if (!disabledMatch) {
              merged.push({ id: crypto.randomUUID(), k: up.k, v: up.v, enabled: true });
            }
          }
        });
        setParams(merged);
      } else {
        const p: UrlParam[] = [];
        parsed.searchParams.forEach((v, k) => p.push({ id: crypto.randomUUID(), k, v, enabled: true }));
        setParams(p);
      }
      setFragment(parsed.hash.replace('#', ''));
    } catch (_err) {
      if (savedParams && savedParams.length > 0) {
        setParams(savedParams);
      } else {
        setParams([]);
      }
      setDomain(targetUrl); setPaths([]); setFragment('');
    }
  }, []);

  if (url !== prevUrl && url !== lastGeneratedUrl.current) {
    setPrevUrl(url);
    setRawUrl(url);
    paramsInitialized.current = false;
  }

  const handleModeSwitch = (newMode: 'raw' | 'structured') => {
    if (newMode === 'structured' && mode === 'raw') {
      // Parse saved params from prop if available and not yet initialized
      let savedParams: UrlParam[] | undefined;
      if (urlParams) {
        try { savedParams = JSON.parse(urlParams); } catch { /* ignore */ }
      }
      parseUrlToStructured(rawUrl, savedParams);
      paramsInitialized.current = true;
    }
    setMode(newMode);
  };

  const emitUrlParams = useCallback((newParams: UrlParam[]) => {
    if (onUrlParamsChange) {
      onUrlParamsChange(JSON.stringify(newParams));
    }
  }, [onUrlParamsChange]);

  const updateStructuredUrl = (newDomain: string, newPaths: typeof paths, newParams: UrlParam[], newFrag: string) => {
    setDomain(newDomain); setPaths(newPaths); setParams(newParams); setFragment(newFrag);

    let reconstructed = newDomain.replace(/\/$/, '');
    if (newPaths.length > 0) reconstructed += '/' + newPaths.map(p => p.v).join('/');
    else reconstructed += '/';

    const safeEncode = (val: string) => encodeURIComponent(val).replace(/%7B/gi, '{').replace(/%7D/gi, '}');

    // Only include enabled params in the URL
    const query = newParams.filter((p) => p.enabled && p.k.trim() !== '').map((p) => `${safeEncode(p.k)}=${safeEncode(p.v)}`).join('&');
    if (query) reconstructed += `?${query}`;
    if (newFrag) reconstructed += `#${newFrag}`;

    setRawUrl(reconstructed);
    lastGeneratedUrl.current = reconstructed;
    if (onChange) onChange(reconstructed);
    emitUrlParams(newParams);
  };

  const addPath = () => updateStructuredUrl(domain, [...paths, { id: crypto.randomUUID(), v: '' }], params, fragment);
  const updatePath = (id: string, v: string) => updateStructuredUrl(domain, paths.map(p => p.id === id ? { ...p, v } : p), params, fragment);
  const deletePath = (id: string) => updateStructuredUrl(domain, paths.filter(p => p.id !== id), params, fragment);

  const addParam = () => updateStructuredUrl(domain, paths, [...params, { id: crypto.randomUUID(), k: '', v: '', enabled: true }], fragment);
  const updateParam = (id: string, k: string, v: string) => updateStructuredUrl(domain, paths, params.map(p => p.id === id ? { ...p, k, v } : p), fragment);
  const toggleParam = (id: string) => updateStructuredUrl(domain, paths, params.map(p => p.id === id ? { ...p, enabled: !p.enabled } : p), fragment);
  const deleteParam = (id: string) => updateStructuredUrl(domain, paths, params.filter(p => p.id !== id), fragment);

  return (
    <div className="flex flex-col bg-zinc-900/50 border border-zinc-800 rounded resize-y overflow-hidden min-h-48">

      <div className="bg-zinc-800/50 px-3 py-2 flex justify-between items-center border-b border-zinc-800 shrink-0">

        <div className="flex items-center gap-3">
          <Select
            value={method}
            disabled={readOnly}
            onChange={(val) => onMethodChange && onMethodChange(val)}
            options={[
              { value: "GET", label: "GET" },
              { value: "POST", label: "POST" },
              { value: "PUT", label: "PUT" },
              { value: "DELETE", label: "DELETE" },
              { value: "PATCH", label: "PATCH" },
              { value: "HEAD", label: "HEAD" },
              { value: "OPTIONS", label: "OPTIONS" }
            ]}
            className="w-32"
          />
          <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest hidden sm:block">Target_URL</span>
        </div>

        <div className="flex bg-zinc-950 p-0.5 rounded items-center">
          <button onClick={() => handleModeSwitch('raw')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'raw' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>Raw</button>
          <button onClick={() => handleModeSwitch('structured')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'structured' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>Structured</button>
        </div>
      </div>

      <div className="p-4 flex-1 overflow-y-auto min-h-0">
        {mode === 'raw' ? (
          <Textarea
            value={rawUrl}
            readOnly={readOnly}
            onChange={(e) => { setRawUrl(e.target.value); if (onChange) onChange(e.target.value); }}
            variant={readOnly ? 'default' : 'emerald'}
            className={`h-full min-h-32 ${readOnly ? 'text-zinc-400 border-dashed focus:border-zinc-700' : ''}`}
            spellCheck={false}
          />
        ) : (
          <div className="space-y-8">
            <div className="space-y-2">
              <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest">Base Domain</label>
              <Input 
                value={domain} 
                readOnly={readOnly} 
                onChange={(e) => updateStructuredUrl(e.target.value, paths, params, fragment)} 
                variant={readOnly ? 'default' : 'emerald'}
                className={readOnly ? 'text-zinc-400' : ''}
                placeholder="https://api.example.com"
              />
            </div>

            <div className="space-y-2">
              <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest">Path Segments</label>
              {paths.length === 0 && <div className="text-xs text-zinc-600 italic font-mono p-3 bg-zinc-950/50 rounded border border-zinc-800 border-dashed">/ (Root)</div>}
              <div className="flex flex-wrap gap-3 items-center">
                {paths.map((p) => (
                  <div key={p.id} className="flex gap-2 items-center group">
                    <span className="text-zinc-600 font-black text-sm">/</span>
                    <Input 
                      value={p.v} 
                      readOnly={readOnly} 
                      onChange={(e) => updatePath(p.id, e.target.value)} 
                      variant={readOnly ? 'default' : 'fuchsia'}
                      className={`w-40 ${readOnly ? 'text-zinc-400' : ''}`}
                      placeholder="path" 
                    />
                    {!readOnly && <button onClick={() => deletePath(p.id)} className="text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded px-1.5 py-1 transition-colors">✕</button>}
                  </div>
                ))}
              </div>
              {!readOnly && <button onClick={addPath} className="mt-3 py-2 w-full border border-dashed border-zinc-700 text-zinc-500 hover:text-fuchsia-400 hover:border-fuchsia-500/50 rounded text-[9px] uppercase font-bold tracking-widest transition-colors">+ Add Path</button>}
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest">Query Parameters</label>
                {params.length > 0 && (
                  <span className="text-[9px] text-zinc-600 font-mono">{params.filter(p => p.enabled).length}/{params.length} active</span>
                )}
              </div>
              {params.length === 0 && <div className="text-xs text-zinc-600 italic font-mono p-3 bg-zinc-950/50 rounded border border-zinc-800 border-dashed">No parameters found.</div>}
              <div className="space-y-3">
                {params.map(p => (
                  <div key={p.id} className={`flex gap-3 group items-start transition-opacity duration-150 ${!p.enabled ? 'opacity-40' : ''}`}>
                    {!readOnly && (
                      <label className="flex items-center pt-2 cursor-pointer shrink-0" title={p.enabled ? 'Disable parameter' : 'Enable parameter'}>
                        <input
                          type="checkbox"
                          checked={p.enabled}
                          onChange={() => toggleParam(p.id)}
                          className="w-3.5 h-3.5 rounded border-zinc-600 bg-zinc-800 text-sky-500 focus:ring-sky-500/30 focus:ring-offset-0 cursor-pointer accent-sky-500"
                        />
                      </label>
                    )}
                    <div className="w-1/3 shrink-0">
                      <Input 
                        value={p.k} 
                        readOnly={readOnly} 
                        onChange={(e) => updateParam(p.id, e.target.value, p.v)} 
                        variant={readOnly ? 'default' : 'sky'}
                        className={`${readOnly ? 'text-zinc-500' : ''} ${!p.enabled ? 'line-through' : ''}`}
                        placeholder="Key" 
                      />
                    </div>
                    <div className="flex-1 min-w-0">
                      <Input 
                        value={p.v} 
                        readOnly={readOnly} 
                        onChange={(e) => updateParam(p.id, p.k, e.target.value)} 
                        variant={readOnly ? 'default' : 'emerald'}
                        className={`${readOnly ? 'text-zinc-400' : ''} ${!p.enabled ? 'line-through' : ''}`}
                        placeholder="Value" 
                      />
                    </div>
                    {!readOnly && <button onClick={() => deleteParam(p.id)} className="p-2 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded transition-colors shrink-0">✕</button>}
                  </div>
                ))}
              </div>
              {!readOnly && <button onClick={addParam} className="mt-3 py-2 w-full border border-dashed border-zinc-700 text-zinc-500 hover:text-sky-text hover:border-sky-500/50 rounded text-[9px] uppercase font-bold tracking-widest transition-colors">+ Add Parameter</button>}
            </div>

            {(fragment || !readOnly) && (
              <div className="space-y-2">
                <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest">Fragment (Hash)</label>
                <div className="flex gap-2 items-center">
                  <span className="text-zinc-600 font-black">#</span>
                  <Input 
                    value={fragment} 
                    readOnly={readOnly} 
                    onChange={(e) => updateStructuredUrl(domain, paths, params, e.target.value)} 
                    variant={readOnly ? 'default' : 'amber'}
                    className={`flex-1 ${readOnly ? 'text-zinc-400' : ''}`}
                    placeholder="section"
                  />
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

