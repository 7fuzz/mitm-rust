import { useState, useEffect, useRef } from 'react';
import { Input, Textarea } from '../ui';

interface Props {
  initialHeaders: Record<string, string>;
  onChange: (headers: Record<string, string>) => void;
}

export function HeaderEditor({ initialHeaders, onChange }: Props) {
  const [mode, setMode] = useState<'structured' | 'raw'>('structured');
  const [rawText, setRawText] = useState('');
  const [entries, setEntries] = useState<{ id: string; k: string; v: string }[]>([]);

  const isInternalUpdate = useRef(false);

  useEffect(() => {
    if (isInternalUpdate.current) {
      isInternalUpdate.current = false;
      return;
    }
    const arr = Object.entries(initialHeaders || {}).map(([k, v]) => ({ id: crypto.randomUUID(), k, v }));
    setEntries(arr);
    setRawText(arr.map((e) => `${e.k}: ${e.v}`).join('\n'));
  }, [initialHeaders]);

  const notifyParent = (newEntries: typeof entries) => {
    const obj: Record<string, string> = {};
    newEntries.forEach((e) => { if (e.k.trim() !== '') obj[e.k] = e.v; });
    isInternalUpdate.current = true;
    onChange(obj);
  };

  const parseRawToEntries = (text: string) => {
    return text.split('\n').map((line) => {
      const idx = line.indexOf(':');
      if (idx === -1) return { id: crypto.randomUUID(), k: line.trim(), v: '' };
      return { id: crypto.randomUUID(), k: line.substring(0, idx).trim(), v: line.substring(idx + 1).trim() };
    }).filter((e) => e.k !== '');
  };

  const handleModeSwitch = (newMode: 'structured' | 'raw') => {
    if (newMode === 'raw' && mode === 'structured') {
      setRawText(entries.filter((e) => e.k.trim() !== '').map((e) => `${e.k}: ${e.v}`).join('\n'));
    } else if (newMode === 'structured' && mode === 'raw') {
      const parsed = parseRawToEntries(rawText);
      setEntries(parsed); notifyParent(parsed);
    }
    setMode(newMode);
  };

  const handleRawChange = (text: string) => {
    setRawText(text);
    const newEntries = parseRawToEntries(text);
    setEntries(newEntries); notifyParent(newEntries);
  };

  const updateKey = (id: string, newK: string) => { const updated = entries.map((e) => (e.id === id ? { ...e, k: newK } : e)); setEntries(updated); notifyParent(updated); };
  const updateVal = (id: string, newV: string) => { const updated = entries.map((e) => (e.id === id ? { ...e, v: newV } : e)); setEntries(updated); notifyParent(updated); };
  const addRow = () => setEntries([...entries, { id: crypto.randomUUID(), k: '', v: '' }]);
  const deleteRow = (id: string) => { const updated = entries.filter((e) => e.id !== id); setEntries(updated); notifyParent(updated); };

  return (
    <div className="flex flex-col h-full min-h-25 overflow-hidden p-2">
      <div className="flex justify-between items-center mb-3 shrink-0">
        <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">Headers</span>
        <div className="flex bg-zinc-950 p-0.5 rounded items-center">
          <button onClick={() => handleModeSwitch('structured')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'structured' ? 'bg-zinc-800 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>Structured</button>
          <button onClick={() => handleModeSwitch('raw')} className={`px-3 py-1 text-[10px] font-bold uppercase rounded transition-all duration-200 ${mode === 'raw' ? 'bg-zinc-800 text-zinc-50 shadow-sm' : 'text-zinc-500 hover:text-zinc-300'}`}>Raw</button>
        </div>
      </div>

      {mode === 'structured' ? (
        <div className="flex flex-col flex-1 min-h-0">
          <div className="flex-1 overflow-y-auto space-y-2 pr-2 scrollbar-thin scrollbar-thumb-zinc-700">
            {entries.length === 0 && <div className="text-xs text-zinc-600 italic font-mono p-2 bg-zinc-950/50 rounded border border-zinc-800 border-dashed text-center">No headers defined.</div>}
            {entries.map((entry) => (
              <div key={entry.id} className="flex gap-2 items-start group">
                <div className="w-[40%] shrink-0">
                  <Input placeholder="Header-Name" value={entry.k} onChange={(e) => updateKey(entry.id, e.target.value)} variant="sky" />
                </div>
                <div className="flex-1 min-w-0">
                  <Input placeholder="value..." value={entry.v} onChange={(e) => updateVal(entry.id, e.target.value)} />
                </div>
                <button onClick={() => deleteRow(entry.id)} className="p-2 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded transition-colors shrink-0" title="Delete Header">✕</button>
              </div>
            ))}
          </div>
          <button onClick={addRow} className="mt-2 py-2 border border-dashed border-zinc-700 text-zinc-500 hover:text-sky-text hover:border-sky-500/50 rounded text-[10px] uppercase font-bold tracking-widest transition-colors shrink-0">+ Add Header</button>
        </div>
      ) : (
        <Textarea
          value={rawText}
          onChange={(e) => handleRawChange(e.target.value)}
          placeholder="Accept: application/json&#10;Authorization: Bearer token..."
          className="flex-1 w-full min-h-25 h-full"
          spellCheck={false}
        />
      )}
    </div>
  );
}
