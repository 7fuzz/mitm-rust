import { useState } from 'react';
import { JsonEditor } from './JsonEditor';
import { FormEditor } from './FormEditor';
import { formToJson, jsonToUrlEncoded, jsonToMultipartStructured } from '@/lib/utils/converter';
import { Textarea } from '../ui';

interface Props {
  body: string;
  headers: Record<string, string>;
  onChange: (newBody: string) => void;
  onHeadersChange?: (newHeaders: Record<string, string>) => void;
}

export function BodyEditor({ body, headers, onChange, onHeadersChange }: Props) {
  const contentTypeKey = Object.keys(headers).find(k => k.toLowerCase() === 'content-type');
  const contentType = contentTypeKey ? headers[contentTypeKey].toLowerCase() : '';

  const [mode, setMode] = useState<'raw' | 'json' | 'form'>(() => {
    if (contentType.includes('application/json')) return 'json';
    if (contentType.includes('form-urlencoded') || contentType.includes('multipart/form-data')) return 'form';
    return 'raw';
  });
  const [prevContentType, setPrevContentType] = useState(contentType);

  if (contentType !== prevContentType) {
    setPrevContentType(contentType);
    if (contentType.includes('application/json')) setMode('json');
    else if (contentType.includes('form-urlencoded') || contentType.includes('multipart/form-data')) setMode('form');
    else setMode('raw');
  }

  const updateContentType = (newType: string) => {
    if (!onHeadersChange) return;
    const newHeaders = { ...headers };
    const key = Object.keys(newHeaders).find(k => k.toLowerCase() === 'content-type') || 'Content-Type';
    newHeaders[key] = newType;
    onHeadersChange(newHeaders);
  };

  const handleConvertToJSON = () => {
    const converted = formToJson(body, contentType);
    if (converted) {
      onChange(converted);
      updateContentType('application/json');
      setMode('json');
    } else {
      alert("Could not convert to JSON. Make sure no files are attached.");
    }
  };

  const handleConvertToForm = (type: 'urlencoded' | 'multipart') => {
    const converted = type === 'urlencoded' ? jsonToUrlEncoded(body) : jsonToMultipartStructured(body);
    if (converted) {
      onChange(converted);
      updateContentType(type === 'urlencoded' ? 'application/x-www-form-urlencoded' : 'multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW');
      setMode('form');
    } else {
      alert("Could not convert to Form Data. Ensure body is valid JSON.");
    }
  };

  const isBodyJson = (() => {
    if (!body.trim()) return true;
    try { JSON.parse(body); return true; } catch { return false; }
  })();

  const isBodyForm = body.includes('__form_data') || contentType.includes('form-urlencoded') || contentType.includes('multipart/form-data') || (!body.trim());

  return (
    <div className="flex flex-col h-full bg-zinc-900/50 border border-zinc-800 rounded resize-y overflow-hidden min-h-37.5">

      <div className="bg-zinc-800/50 px-3 py-1.5 flex justify-between items-center border-b border-zinc-800 shrink-0">
        <div className="flex items-center gap-3">
          <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">Body Format:</span>
          <div className="flex bg-zinc-950 p-0.5 rounded items-center">
            <button onClick={() => setMode('raw')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'raw' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>
              raw
            </button>
            {(isBodyJson || mode === 'json') && (
              <button onClick={() => setMode('json')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'json' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>
                json
              </button>
            )}
            {(isBodyForm || mode === 'form') && (
              <button onClick={() => setMode('form')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'form' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>
                form
              </button>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {mode === 'form' && (
            <button 
              onClick={handleConvertToJSON}
              className="text-[9px] font-black uppercase tracking-widest text-emerald-text hover:text-emerald-text bg-emerald-500/10 px-2 py-1 rounded border border-emerald-500/20 transition-all"
            >
              Convert to JSON
            </button>
          )}
          {mode === 'json' && (
            <div className="flex gap-1">
              <button 
                onClick={() => handleConvertToForm('urlencoded')}
                className="text-[9px] font-black uppercase tracking-widest text-amber-500 hover:text-amber-400 bg-amber-500/10 px-2 py-1 rounded border border-amber-500/20 transition-all"
              >
                to URL-Encoded
              </button>
              <button 
                onClick={() => handleConvertToForm('multipart')}
                className="text-[9px] font-black uppercase tracking-widest text-rose-500 hover:text-rose-400 bg-rose-500/10 px-2 py-1 rounded border border-rose-500/20 transition-all"
              >
                to Multipart
              </button>
            </div>
          )}
          {contentType && <span className="text-[9px] text-emerald-text/70 font-mono italic truncate max-w-40 hidden sm:inline">Detected: {contentType.split(';')[0]}</span>}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-3 min-h-0">
        {mode === 'raw' && (
          <Textarea
            value={body}
            onChange={(e) => onChange(e.target.value)}
            spellCheck={false}
            className="w-full h-full min-h-25 leading-relaxed bg-transparent border-transparent focus:border-transparent"
          />
        )}
        {mode === 'json' && <JsonEditor initialBody={body} onChange={onChange} />}
        {mode === 'form' && <FormEditor initialBody={body} contentType={contentType} onChange={onChange} />}
      </div>
    </div>
  );
}
