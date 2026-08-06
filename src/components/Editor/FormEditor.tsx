import { useState, useEffect, useRef } from 'react';
import { Input, Textarea, Select } from '../ui';
import { invoke } from '@/lib/utils/tauri';

interface FormEntry {
  id: string;
  k: string;
  v: string;
  type: 'text' | 'file' | 'base64';
  fileName?: string;
  contentType?: string;
  fileContent?: string; // Store path or reference
}

export function FormEditor({ initialBody, contentType, onChange }: { initialBody: string; contentType: string; onChange: (v: string) => void }) {
  const [entries, setEntries] = useState<FormEntry[]>([]);
  const lastEmitted = useRef<string | null>(null);
  const isInternalUpdate = useRef(false);

  const isUrlEncoded = contentType.includes('x-www-form-urlencoded');

  useEffect(() => {
    if (initialBody === lastEmitted.current) return;
    if (isInternalUpdate.current) {
      isInternalUpdate.current = false;
      return;
    }

    const parsed: FormEntry[] = [];
    if (isUrlEncoded) {
      const params = new URLSearchParams(initialBody);
      params.forEach((v, k) => parsed.push({ id: crypto.randomUUID(), k, v, type: 'text' }));
    } else if (contentType.includes('multipart/form-data')) {
      if (initialBody.startsWith('{') && initialBody.endsWith('}')) {
        try {
          const data = JSON.parse(initialBody);
          if (data.__form_data) {
             setEntries(data.__form_data.map((e: FormEntry) => ({ ...e, id: e.id || crypto.randomUUID() })));
             return;
          }
        } catch { /* ignore */ }
      }
      
      const boundaryMatch = contentType.match(/boundary=(?:"([^"]+)"|([^;]+))/i);
      const boundary = boundaryMatch ? (boundaryMatch[1] || boundaryMatch[2]) : '';
      if (boundary && initialBody && !initialBody.includes("Form Editor modified")) {
        const parts = initialBody.split(`--${boundary}`);
        parts.forEach(part => {
          if (part.includes('name=')) {
            const nameMatch = part.match(/name="([^"]+)"/);
            const filenameMatch = part.match(/filename="([^"]+)"/);
            const valueMatch = part.split('\r\n\r\n')[1];
            
            if (nameMatch && valueMatch) {
              const k = nameMatch[1];
              const v = valueMatch.replace(/\r\n$/, '');
              if (filenameMatch) {
                parsed.push({ id: crypto.randomUUID(), k, v: '[FILE]', type: 'file', fileName: filenameMatch[1] });
              } else {
                parsed.push({ id: crypto.randomUUID(), k, v, type: 'text' });
              }
            }
          }
        });
      }
    }
    setEntries(parsed.length > 0 ? parsed : [{ id: crypto.randomUUID(), k: '', v: '', type: 'text' }]);
  }, [initialBody, contentType]);

  const updateBody = (newEntries: FormEntry[]) => {
    let newBodyString = "";
    if (contentType.includes('x-www-form-urlencoded')) {
      const params = new URLSearchParams();
      newEntries.forEach(e => { if (e.k) params.append(e.k, e.v); });
      newBodyString = params.toString();
    } else {
      newBodyString = JSON.stringify({
        __form_data: newEntries.map(({ id, fileContent, ...rest }) => rest),
        _hint: "Form Editor modified. (Multipart will be reconstructed on send)"
      });
    }
    lastEmitted.current = newBodyString;
    isInternalUpdate.current = true;
    onChange(newBodyString);
  };

  const updateEntry = (id: string, updates: Partial<FormEntry>) => {
    const updated = entries.map(e => e.id === id ? { ...e, ...updates } : e);
    setEntries(updated);
    updateBody(updated);
  };

  const addRow = () => setEntries([...entries, { id: crypto.randomUUID(), k: '', v: '', type: 'text' }]);
  
  const deleteRow = (id: string) => {
    const updated = entries.filter(e => e.id !== id);
    setEntries(updated);
    updateBody(updated);
  };

  const handleFileUpload = async (id: string, file: File) => {
    try {
      updateEntry(id, { fileName: file.name, contentType: file.type, v: 'Uploading...' });
      
      const buffer = await file.arrayBuffer();
      const content = Array.from(new Uint8Array(buffer));
      
      const path = await invoke<string>('upload_file', { 
        name: file.name, 
        content 
      });
      
      updateEntry(id, { v: path, fileName: file.name, contentType: file.type });
    } catch (_error) {
      updateEntry(id, { v: 'Upload Error', fileName: '', contentType: '' });
    }
  };

  const handleBase64FileUpload = (id: string, file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      updateEntry(id, {
        v: result,
        fileName: file.name,
        contentType: file.type || 'application/octet-stream'
      });
    };
    reader.onerror = () => {
      updateEntry(id, { v: 'Error reading file', fileName: file.name });
    };
    reader.readAsDataURL(file);
  };

  return (
    <div className="space-y-2">
      {entries.map(e => (
        <div key={e.id} className="flex gap-2 group items-start">
          <div className="flex flex-col gap-1 w-1/3">
             <Input 
               value={e.k} 
               onChange={(ev) => updateEntry(e.id, { k: ev.target.value })} 
               placeholder="Key" 
               variant="sky"
             />
             <Select 
               value={e.type} 
               onChange={(val) => updateEntry(e.id, { type: val as 'text' | 'file' | 'base64', v: '' })}
               disabled={isUrlEncoded}
               options={[
                 { value: 'text', label: 'Text' },
                 ...(!isUrlEncoded ? [
                   { value: 'file', label: 'File (Path)' },
                   { value: 'base64', label: 'File (Base64)' }
                 ] : [])
               ]}
             />
          </div>
          
          <div className="flex-1 flex flex-col gap-1">
            {e.type === 'text' ? (
              <Textarea 
                value={e.v} 
                onChange={(ev) => updateEntry(e.id, { v: ev.target.value })} 
                placeholder="Value" 
                rows={1}
                className="resize-none min-h-[34px] break-all" 
              />
            ) : e.type === 'base64' ? (
              <div className="flex flex-col gap-2 p-2 bg-zinc-950 border border-zinc-800 rounded min-h-[34px]">
                <div className="flex items-center justify-between gap-2">
                   <div className="flex flex-col gap-1 flex-1 min-w-0">
                      <Input 
                        value={e.fileName || ''} 
                        onChange={(ev) => updateEntry(e.id, { fileName: ev.target.value })}
                        placeholder="File Name (e.g. document.pdf)"
                        variant="sky"
                        className="px-2 py-1 text-[9px] bg-zinc-900"
                      />
                      <Input 
                        value={e.contentType || ''} 
                        onChange={(ev) => updateEntry(e.id, { contentType: ev.target.value })}
                        placeholder="Content-Type (e.g. application/pdf)"
                        variant="emerald"
                        className="px-2 py-1 text-[9px] bg-zinc-900"
                      />
                   </div>
                   {e.v && (
                     <span className="text-[9px] text-purple-text font-bold uppercase shrink-0">Base64</span>
                   )}
                </div>
                <Textarea 
                  value={e.v} 
                  onChange={(ev) => updateEntry(e.id, { v: ev.target.value })} 
                  placeholder="Base64 content or data:mime;base64,..." 
                  rows={2}
                  className="resize-none text-[10px] font-mono break-all bg-zinc-900" 
                />
                <input 
                  type="file" 
                  onChange={(ev) => {
                    const file = ev.target.files?.[0];
                    if (file) handleBase64FileUpload(e.id, file);
                  }}
                  className="text-[10px] text-zinc-400 file:mr-4 file:py-1 file:px-2 file:rounded file:border-0 file:text-[10px] file:font-semibold file:bg-zinc-800 file:text-zinc-300 hover:file:bg-zinc-700 cursor-pointer"
                />
              </div>
            ) : (
              <div className="flex flex-col gap-2 p-2 bg-zinc-950 border border-zinc-800 rounded min-h-[34px]">
                <div className="flex items-center justify-between gap-2">
                   <div className="flex flex-col gap-1 flex-1 min-w-0">
                      <span className="text-[10px] text-zinc-500 font-mono truncate">
                        {e.fileName || 'No file selected'}
                      </span>
                      <Input 
                        value={e.contentType || ''} 
                        onChange={(ev) => updateEntry(e.id, { contentType: ev.target.value })}
                        placeholder="Content-Type (e.g. image/jpeg)"
                        variant="emerald"
                        className="px-2 py-1 text-[9px] bg-zinc-900"
                      />
                   </div>
                   {e.v && e.v !== 'Uploading...' && e.v !== 'Upload Failed' && e.v !== 'Upload Error' && (
                     <span className="text-[9px] text-emerald-text font-bold uppercase shrink-0">Ready</span>
                   )}
                </div>
                <input 
                  type="file" 
                  onChange={(ev) => {
                    const file = ev.target.files?.[0];
                    if (file) handleFileUpload(e.id, file);
                  }}
                  className="text-[10px] text-zinc-400 file:mr-4 file:py-1 file:px-2 file:rounded file:border-0 file:text-[10px] file:font-semibold file:bg-zinc-800 file:text-zinc-300 hover:file:bg-zinc-700 cursor-pointer"
                />
              </div>
            )}
          </div>
          
          <button onClick={() => deleteRow(e.id)} className="p-2 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded">✕</button>
        </div>
      ))}
      <button onClick={addRow} className="w-full py-2 border border-dashed border-zinc-700 text-zinc-500 hover:text-sky-text hover:border-sky-500/50 rounded text-[10px] uppercase font-bold tracking-widest transition-colors">+ Add Form Data</button>
    </div>
  );
}
