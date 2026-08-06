import { useState, useEffect } from 'react';
import { JsonEditor } from './JsonEditor';
import { FormEditor } from './FormEditor';
import { formToJson, jsonToUrlEncoded, jsonToMultipartStructured } from '@/lib/utils/converter';
import { Textarea, useDialog } from '../ui';

export type BodyMode = 'raw' | 'json' | 'urlencoded' | 'multipart';

interface Props {
  body: string;
  bodyMode?: BodyMode;
  bodyJson?: string;
  bodyUrlencoded?: string;
  bodyMultipart?: string;
  headers: [string, string][];
  onChange: (newBody: string) => void;
  onModeChange?: (mode: BodyMode) => void;
  onBodyJsonChange?: (val: string) => void;
  onBodyUrlencodedChange?: (val: string) => void;
  onBodyMultipartChange?: (val: string) => void;
  onHeadersChange?: (newHeaders: [string, string][]) => void;
}

export function BodyEditor({
  body,
  bodyMode = 'raw',
  bodyJson = '',
  bodyUrlencoded = '',
  bodyMultipart = '',
  headers,
  onChange,
  onModeChange,
  onBodyJsonChange,
  onBodyUrlencodedChange,
  onBodyMultipartChange,
  onHeadersChange,
}: Props) {
  const { alert } = useDialog();
  const contentTypeEntry = headers.find(([k]) => k.toLowerCase() === 'content-type');
  const contentType = contentTypeEntry ? contentTypeEntry[1].toLowerCase() : '';

  const [mode, setMode] = useState<BodyMode>(bodyMode);

  useEffect(() => {
    setMode(bodyMode || 'raw');
  }, [bodyMode]);

  const updateContentType = (newType: string) => {
    if (!onHeadersChange) return;
    let found = false;
    const newHeaders = headers.map(([k, v]) => {
      if (k.toLowerCase() === 'content-type') {
        found = true;
        return [k, newType] as [string, string];
      }
      return [k, v] as [string, string];
    });

    if (!found) {
      newHeaders.push(['Content-Type', newType]);
    }
    onHeadersChange(newHeaders);
  };

  const handleSwitchMode = (targetMode: BodyMode) => {
    setMode(targetMode);
    onModeChange?.(targetMode);

    if (targetMode === 'raw') {
      onChange(body);
    } else if (targetMode === 'json') {
      updateContentType('application/json');
      const nextJson = bodyJson || (body.startsWith('{') || body.startsWith('[') ? body : '{\n  \n}');
      onBodyJsonChange?.(nextJson);
      onChange(nextJson);
    } else if (targetMode === 'urlencoded') {
      updateContentType('application/x-www-form-urlencoded');
      const nextUrl = bodyUrlencoded || (contentType.includes('x-www-form-urlencoded') ? body : '');
      onBodyUrlencodedChange?.(nextUrl);
      onChange(nextUrl);
    } else if (targetMode === 'multipart') {
      updateContentType('multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW');
      const nextMulti = bodyMultipart || (contentType.includes('multipart') || body.includes('__form_data') ? body : '');
      onBodyMultipartChange?.(nextMulti);
      onChange(nextMulti);
    }
  };

  const handleCopyToRaw = () => {
    onModeChange?.('raw');
    setMode('raw');
    onChange(body);
  };

  const handleConvertToJSON = async () => {
    const converted = formToJson(body, contentType);
    if (converted) {
      onBodyJsonChange?.(converted);
      onChange(converted);
      updateContentType('application/json');
      onModeChange?.('json');
      setMode('json');
    } else {
      await alert("Conversion Failed", "Could not convert current body to valid JSON.");
    }
  };

  const handleConvertToForm = async (type: 'urlencoded' | 'multipart') => {
    const converted = type === 'urlencoded' ? jsonToUrlEncoded(body) : jsonToMultipartStructured(body);
    if (converted) {
      if (type === 'urlencoded') {
        onBodyUrlencodedChange?.(converted);
        updateContentType('application/x-www-form-urlencoded');
        onModeChange?.('urlencoded');
        setMode('urlencoded');
      } else {
        onBodyMultipartChange?.(converted);
        updateContentType('multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW');
        onModeChange?.('multipart');
        setMode('multipart');
      }
      onChange(converted);
    } else {
      await alert("Conversion Failed", "Could not convert to Form Data. Ensure body is valid JSON or parameter list.");
    }
  };

  return (
    <div className="flex flex-col h-full bg-zinc-900/50 border border-zinc-800 rounded resize-y overflow-hidden min-h-37.5">

      <div className="bg-zinc-800/50 px-3 py-1.5 flex justify-between items-center border-b border-zinc-800 shrink-0 flex-wrap gap-2">
        <div className="flex items-center gap-3">
          <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">Body Mode:</span>
          <div className="flex bg-zinc-950 p-0.5 rounded items-center">
            <button
              onClick={() => handleSwitchMode('raw')}
              className={`px-2.5 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${
                mode === 'raw' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              raw
            </button>
            <button
              onClick={() => handleSwitchMode('json')}
              className={`px-2.5 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${
                mode === 'json' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              json
            </button>
            <button
              onClick={() => handleSwitchMode('urlencoded')}
              className={`px-2.5 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${
                mode === 'urlencoded' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              x-www-form-urlencoded
            </button>
            <button
              onClick={() => handleSwitchMode('multipart')}
              className={`px-2.5 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${
                mode === 'multipart' ? 'bg-zinc-700 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              form-data
            </button>
          </div>
        </div>

        {/* Copy / Transform Actions */}
        <div className="flex items-center gap-1.5">
          {mode !== 'raw' && (
            <button
              onClick={handleCopyToRaw}
              className="text-[9px] font-black uppercase tracking-widest text-zinc-400 hover:text-zinc-200 bg-zinc-800 px-2 py-1 rounded border border-zinc-700 transition-all"
              title="Copy active formatted body into raw text"
            >
              📋 Copy to Raw
            </button>
          )}
          {mode !== 'json' && (
            <button
              onClick={handleConvertToJSON}
              className="text-[9px] font-black uppercase tracking-widest text-emerald-text hover:text-emerald-300 bg-emerald-500/10 px-2 py-1 rounded border border-emerald-500/20 transition-all"
              title="Transform current body into JSON"
            >
              ⚡ to JSON
            </button>
          )}
          {mode !== 'urlencoded' && (
            <button
              onClick={() => handleConvertToForm('urlencoded')}
              className="text-[9px] font-black uppercase tracking-widest text-amber-500 hover:text-amber-400 bg-amber-500/10 px-2 py-1 rounded border border-amber-500/20 transition-all"
              title="Transform current body into URL-encoded parameters"
            >
              ⚡ to URL-Encoded
            </button>
          )}
          {mode !== 'multipart' && (
            <button
              onClick={() => handleConvertToForm('multipart')}
              className="text-[9px] font-black uppercase tracking-widest text-purple-400 hover:text-purple-300 bg-purple-500/10 px-2 py-1 rounded border border-purple-500/20 transition-all"
              title="Transform current body into Multipart Form Data"
            >
              ⚡ to Multipart
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-3 min-h-0">
        {mode === 'raw' && (
          <Textarea
            value={body}
            onChange={(e) => {
              onChange(e.target.value);
            }}
            spellCheck={false}
            className="w-full h-full min-h-25 leading-relaxed bg-transparent border-transparent focus:border-transparent"
          />
        )}
        {mode === 'json' && (
          <JsonEditor
            initialBody={bodyJson || body}
            onChange={(newVal) => {
              onBodyJsonChange?.(newVal);
              onChange(newVal);
            }}
          />
        )}
        {mode === 'urlencoded' && (
          <FormEditor
            initialBody={bodyUrlencoded || body}
            contentType="application/x-www-form-urlencoded"
            onChange={(newVal) => {
              onBodyUrlencodedChange?.(newVal);
              onChange(newVal);
            }}
          />
        )}
        {mode === 'multipart' && (
          <FormEditor
            initialBody={bodyMultipart || body}
            contentType="multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW"
            onChange={(newVal) => {
              onBodyMultipartChange?.(newVal);
              onChange(newVal);
            }}
          />
        )}
      </div>
    </div>
  );
}
