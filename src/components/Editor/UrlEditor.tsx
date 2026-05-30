import { useState, useCallback, useRef } from 'react';
import { Input, Textarea } from '../ui';

interface Props {
  method?: string;
  onMethodChange?: (m: string) => void;
  url: string;
  onChange?: (newUrl: string) => void;
  readOnly?: boolean;
}

export function UrlEditor({ method = 'GET', onMethodChange, url, onChange, readOnly = false }: Props) {
  const [mode, setMode] = useState<'raw' | 'structured'>('raw');
  const [rawUrl, setRawUrl] = useState(url);
  const [prevUrl, setPrevUrl] = useState(url);
  const lastGeneratedUrl = useRef(url);

  const [domain, setDomain] = useState('');
  const [paths, setPaths] = useState<{ id: string; v: string }[]>([]);
  const [params, setParams] = useState<{ id: string; k: string; v: string }[]>([]);
  const [fragment, setFragment] = useState('');

  const parseUrlToStructured = useCallback((targetUrl: string) => {
    try {
      const parsed = new URL(targetUrl);
      setDomain(parsed.origin);

      const pathSegments = parsed.pathname
        .split('/')
        .filter(p => p !== '')
        .map(v => ({ id: crypto.randomUUID(), v }));
      setPaths(pathSegments);

      const p: { id: string; k: string; v: string }[] = [];
      parsed.searchParams.forEach((v, k) => p.push({ id: crypto.randomUUID(), k, v }));
      setParams(p);
      setFragment(parsed.hash.replace('#', ''));
    } catch (_err) {
      setDomain(targetUrl); setPaths([]); setParams([]); setFragment('');
    }
  }, []);

  if (url !== prevUrl && url !== lastGeneratedUrl.current) {
    setPrevUrl(url);
    setRawUrl(url);
    parseUrlToStructured(url);
  }

  const handleModeSwitch = (newMode: 'raw' | 'structured') => {
    if (newMode === 'structured' && mode === 'raw') parseUrlToStructured(rawUrl);
    setMode(newMode);
  };

  const updateStructuredUrl = (newDomain: string, newPaths: typeof paths, newParams: typeof params, newFrag: string) => {
    setDomain(newDomain); setPaths(newPaths); setParams(newParams); setFragment(newFrag);

    let reconstructed = newDomain.replace(/\/$/, '');
    if (newPaths.length > 0) reconstructed += '/' + newPaths.map(p => p.v).join('/');
    else reconstructed += '/';

    const safeEncode = (val: string) => encodeURIComponent(val).replace(/%7B/gi, '{').replace(/%7D/gi, '}');

    const query = newParams.filter((p) => p.k.trim() !== '').map((p) => `${safeEncode(p.k)}=${safeEncode(p.v)}`).join('&');
    if (query) reconstructed += `?${query}`;
    if (newFrag) reconstructed += `#${newFrag}`;

    setRawUrl(reconstructed);
    lastGeneratedUrl.current = reconstructed;
    if (onChange) onChange(reconstructed);
  };

  const addPath = () => updateStructuredUrl(domain, [...paths, { id: crypto.randomUUID(), v: '' }], params, fragment);
  const updatePath = (id: string, v: string) => updateStructuredUrl(domain, paths.map(p => p.id === id ? { ...p, v } : p), params, fragment);
  const deletePath = (id: string) => updateStructuredUrl(domain, paths.filter(p => p.id !== id), params, fragment);

  const addParam = () => updateStructuredUrl(domain, paths, [...params, { id: crypto.randomUUID(), k: '', v: '' }], fragment);
  const updateParam = (id: string, k: string, v: string) => updateStructuredUrl(domain, paths, params.map(p => p.id === id ? { ...p, k, v } : p), fragment);
  const deleteParam = (id: string) => updateStructuredUrl(domain, paths, params.filter(p => p.id !== id), fragment);

  return (
    <div className="flex flex-col bg-zinc-900/50 border border-zinc-800 rounded resize-y overflow-hidden min-h-48">

      <div className="bg-zinc-800/50 px-3 py-2 flex justify-between items-center border-b border-zinc-800 shrink-0">

        <div className="flex items-center gap-3">
          <select
            value={method}
            disabled={readOnly}
            onChange={(e) => onMethodChange && onMethodChange(e.target.value)}
            className={`bg-zinc-950 border border-zinc-700 px-2 py-1 rounded font-black outline-none transition-colors text-[10px] text-center uppercase tracking-widest
              ${readOnly ? 'text-zinc-500 appearance-none' : 'text-amber-500 focus:border-amber-500 cursor-pointer'}
            `}
          >
            <option value="GET">GET</option><option value="POST">POST</option><option value="PUT">PUT</option>
            <option value="DELETE">DELETE</option><option value="PATCH">PATCH</option><option value="HEAD">HEAD</option>
            <option value="OPTIONS">OPTIONS</option>
          </select>
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
              <label className="text-[9px] text-zinc-500 uppercase font-bold tracking-widest">Query Parameters</label>
              {params.length === 0 && <div className="text-xs text-zinc-600 italic font-mono p-3 bg-zinc-950/50 rounded border border-zinc-800 border-dashed">No parameters found.</div>}
              <div className="space-y-3">
                {params.map(p => (
                  <div key={p.id} className="flex gap-3 group items-start">
                    <div className="w-1/3 shrink-0">
                      <Input 
                        value={p.k} 
                        readOnly={readOnly} 
                        onChange={(e) => updateParam(p.id, e.target.value, p.v)} 
                        variant={readOnly ? 'default' : 'sky'}
                        className={readOnly ? 'text-zinc-500' : ''}
                        placeholder="Key" 
                      />
                    </div>
                    <div className="flex-1 min-w-0">
                      <Input 
                        value={p.v} 
                        readOnly={readOnly} 
                        onChange={(e) => updateParam(p.id, p.k, e.target.value)} 
                        variant={readOnly ? 'default' : 'emerald'}
                        className={readOnly ? 'text-zinc-400' : ''}
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
