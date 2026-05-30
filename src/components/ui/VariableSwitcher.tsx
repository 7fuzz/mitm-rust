import { useState, useRef, useEffect } from 'react';
import { useTraffic, GlobalVariable } from '@/hooks/traffic';
import { VariableEditorModal } from '../Modals/VariableEditorModal';

export function VariableSwitcher() {
  const { variables, activeEnvId, updateVariable, deleteVariable, simpleMode } = useTraffic();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  
  // Modal state
  const [editingVar, setEditingVar] = useState<GlobalVariable | null>(null);

  const envVariables = variables.filter(v => v.environmentId === activeEnvId);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  if (simpleMode || envVariables.length === 0) return null;

  return (
    <>
      <VariableEditorModal 
        isOpen={!!editingVar}
        variable={editingVar}
        onClose={() => setEditingVar(null)}
        onUpdate={updateVariable}
        onDelete={deleteVariable}
      />

      <div className="relative" ref={dropdownRef}>
        <button
          onClick={() => setIsOpen(!isOpen)}
          className={`flex items-center gap-2 bg-zinc-950 p-1 rounded-full border px-3 shadow-inner transition-all ${isOpen ? 'border-emerald-500/50 ring-1 ring-emerald-500/20 shadow-emerald-500/10' : 'border-zinc-800 shadow-app-shadow/50 hover:border-zinc-700'}`}
        >
          <span className="text-[9px] text-zinc-500 font-black uppercase tracking-widest hidden sm:inline-block">Vars:</span>
          <span className="text-emerald-text text-[10px] uppercase font-bold truncate max-w-24">
            {envVariables.length} Defined
          </span>
          <svg 
            width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"
            className={`text-zinc-500 transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`}
          >
            <polyline points="6 9 12 15 18 9"></polyline>
          </svg>
        </button>

        {isOpen && (
          <div className="absolute top-full mt-2 left-0 w-72 bg-zinc-950 border border-zinc-800 rounded-lg shadow-2xl z-50 overflow-hidden animate-in fade-in zoom-in-95 duration-150 origin-top-left">
            <div className="p-3 border-b border-zinc-800 bg-zinc-900/30 flex items-center justify-between">
              <span className="text-[9px] font-black uppercase tracking-widest text-zinc-500">Quick_Variant_Switch</span>
              <span className="text-[8px] font-mono text-zinc-600 bg-zinc-950 px-1.5 py-0.5 rounded border border-zinc-800">{envVariables.length} TOTAL</span>
            </div>
            
            <div className="max-h-96 overflow-y-auto p-1 scrollbar-thin scrollbar-thumb-zinc-800">
              {envVariables.map(v => (
                <div key={v.id} className="p-2 flex flex-col gap-1.5 hover:bg-zinc-900/40 rounded-md transition-colors border border-transparent hover:border-zinc-800/50 mb-1">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5 truncate">
                      <span className="text-[10px] font-bold text-amber-400 truncate tracking-wide">{v.name || 'Unnamed_Var'}</span>
                      <button 
                        onClick={(e) => { e.stopPropagation(); setEditingVar(v); }}
                        className="p-1 text-zinc-600 hover:text-emerald-400 transition-colors"
                        title="Edit Variable"
                      >
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
                      </button>
                    </div>
                    <span className="text-[8px] font-mono text-zinc-600">idx: {v.activeIndex}</span>
                  </div>
                  
                  <div className="flex flex-wrap gap-1">
                    {v.values.map((val, idx) => (
                      <button
                        key={val.id}
                        onClick={() => updateVariable(v.id, { activeIndex: idx }, true)}
                        title={`${val.name}: ${val.value}`}
                        className={`px-2 py-1 rounded text-[8px] font-bold tracking-tighter border transition-all ${v.activeIndex === idx ? 'bg-emerald-500 border-emerald-400 text-zinc-950' : 'bg-zinc-900 border-zinc-800 text-zinc-500 hover:text-zinc-300 hover:border-zinc-700'}`}
                      >
                        {val.name}
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            
            <div className="p-2 border-t border-zinc-800 bg-zinc-900/20 text-center">
               <span className="text-[8px] text-zinc-600 uppercase tracking-widest font-bold">Shift+Click workspace for full edit</span>
            </div>
          </div>
        )}
      </div>
    </>
  );
}
