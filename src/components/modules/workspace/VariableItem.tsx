import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GlobalVariable } from '@/hooks/traffic/types';
import { DebouncedInput } from '../../ui/DebouncedInput';

interface VariableItemProps {
  v: GlobalVariable;
  autoSave: boolean;
  updateVariable: (id: string, updates: Partial<GlobalVariable>, immediate?: boolean) => void;
  saveVariable: (id: string, updates: Partial<GlobalVariable>) => void;
  deleteVariable: (id: string) => void;
}

export function VariableItem({ v, autoSave, updateVariable, saveVariable, deleteVariable }: VariableItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: v.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    zIndex: isDragging ? 50 : undefined,
    position: 'relative' as const,
  };

  return (
    <div 
      ref={setNodeRef} 
      style={style}
      className={`p-4 flex items-start gap-6 hover:bg-zinc-900/20 transition-colors group ${isDragging ? 'bg-zinc-900 border-emerald-500/50 opacity-50' : 'bg-transparent'}`}
    >
      <div 
        {...attributes} 
        {...listeners}
        className="mt-1 cursor-grab active:cursor-grabbing text-zinc-700 hover:text-zinc-500 transition-colors shrink-0"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><circle cx="9" cy="5" r="1"></circle><circle cx="9" cy="12" r="1"></circle><circle cx="9" cy="19" r="1"></circle><circle cx="15" cy="5" r="1"></circle><circle cx="15" cy="12" r="1"></circle><circle cx="15" cy="19" r="1"></circle></svg>
      </div>

      <div className="flex flex-col gap-1 w-1/4">
        <DebouncedInput 
          value={v.name}
          onChange={(val) => updateVariable(v.id, { name: val })}
          className="bg-transparent border-none !px-0 focus-within:!border-none"
          inputClassName="text-amber-400 font-bold text-xs !px-0"
          placeholder="variable_name"
          showIcon={false}
        />
        <div className="flex items-center gap-2">
          <span className="text-[9px] text-zinc-600 font-mono uppercase">ID: {v.id.substring(0, 8)}...</span>
          {!autoSave && (
            <button 
              onClick={() => saveVariable(v.id, v)}
              className="text-[8px] font-black uppercase tracking-widest text-emerald-text hover:text-emerald-text transition-colors"
            >
              [Save]
            </button>
          )}
        </div>
      </div>
      <div className="flex-1 flex flex-col gap-3">
        {v.values.map((val, idx) => (
          <div key={val.id} className="flex items-center gap-2">
            <DebouncedInput 
              value={val.name}
              onChange={(newVal) => {
                const newVals = v.values.map(vv => vv.id === val.id ? { ...vv, name: newVal } : vv);
                updateVariable(v.id, { values: newVals });
              }}
              className={`w-24 bg-zinc-900/50 border border-zinc-800 rounded`}
              inputClassName={`${val.name === '(auto)' ? 'text-purple-400' : 'text-sky-text'} font-bold`}
              placeholder="Variant"
              disabled={val.name === '(auto)'}
              showIcon={false}
            />
            <DebouncedInput 
              value={val.value}
              onChange={(newVal) => {
                const newVals = v.values.map(vv => vv.id === val.id ? { ...vv, value: newVal } : vv);
                updateVariable(v.id, { values: newVals });
              }}
              className="flex-1 bg-zinc-900/50 border border-zinc-800 rounded"
              placeholder="Value..."
              showIcon={false}
            />
            <div className="flex items-center gap-1 ml-2">
              <button 
                onClick={() => updateVariable(v.id, { activeIndex: idx })}
                className={`px-2 py-1 rounded text-[8px] font-black tracking-tighter border transition-all ${v.activeIndex === idx ? 'bg-emerald-500 border-emerald-400 text-zinc-950' : 'bg-transparent border-zinc-800 text-zinc-600 hover:text-zinc-300'}`}
              >
                {v.activeIndex === idx ? 'Active' : 'Set'}
              </button>
              {v.values.length > 1 && val.name !== '(auto)' && (
                <button onClick={() => updateVariable(v.id, { values: v.values.filter(vv => vv.id !== val.id), activeIndex: 0 })} className="p-1 text-zinc-700 hover:text-rose-500"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg></button>
              )}
            </div>
          </div>
        ))}
        <button 
          onClick={() => updateVariable(v.id, { values: [...v.values, { id: crypto.randomUUID(), name: `Variant ${v.values.length + 1}`, value: '' }] })}
          className="text-[9px] text-zinc-500 hover:text-emerald-text font-bold uppercase tracking-widest w-fit"
        >
          + New Variant
        </button>
      </div>
      <button onClick={() => deleteVariable(v.id)} className="p-2 text-zinc-700 hover:text-rose-500 opacity-0 group-hover:opacity-100 transition-all"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg></button>
    </div>
  );
}
