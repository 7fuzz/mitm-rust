import { useState, useEffect, useRef } from 'react';
import { GlobalVariable, Environment } from '@/hooks/traffic/types';
import { Modal } from '../ui';
import { useHotkeys } from '@/hooks/ui/useHotkeys';

interface VariableQuickSwitcherProps {
  isOpen: boolean;
  onClose: () => void;
  variables: GlobalVariable[];
  activeEnvId: string;
  onUpdateVariable: (id: string, updates: Partial<GlobalVariable>, immediate?: boolean) => void;
  onReorder: (variables: GlobalVariable[]) => void;
  onAddVariable: (v: GlobalVariable) => void;
  onDeleteVariable: (id: string) => void;
  onEdit: () => void;
}

type InlineEditType = 'value' | 'rename-var' | 'rename-variant' | 'new-var' | 'new-variant';

export function VariableQuickSwitcherModal({ isOpen, onClose, variables, activeEnvId, onUpdateVariable, onReorder, onAddVariable, onDeleteVariable, onEdit }: VariableQuickSwitcherProps) {
  const envVars = variables.filter(v => v.environmentId === activeEnvId);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filter, setFilter] = useState('');
  const listRef = useRef<HTMLDivElement>(null);
  const filterInputRef = useRef<HTMLInputElement>(null);
  const editInputRef = useRef<HTMLInputElement>(null);
  
  const [inlineEdit, setInlineEdit] = useState<{
    type: InlineEditType;
    id?: string;
    value: string;
    extraValue?: string; // used for new-variant value
  } | null>(null);

  const filtered = envVars.filter(v => v.name.toLowerCase().includes(filter.toLowerCase()));

  useEffect(() => {
    if (isOpen) {
      setSelectedIndex(0);
      setFilter('');
      setInlineEdit(null);
    }
  }, [isOpen]);

  useEffect(() => {
    if (listRef.current && filtered.length > 0 && !inlineEdit) {
      const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedEl) {
        selectedEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }, [selectedIndex, filtered.length, inlineEdit]);

  useEffect(() => {
    if (inlineEdit) {
      setTimeout(() => editInputRef.current?.focus(), 10);
    }
  }, [inlineEdit]);

  const v = filtered[selectedIndex];

  const handleSaveInline = () => {
    if (!inlineEdit) return;

    if (inlineEdit.type === 'value' && v) {
      const newValues = v.values.map((val, idx) => 
        idx === v.activeIndex ? { ...val, value: inlineEdit.value } : val
      );
      onUpdateVariable(v.id, { values: newValues }, true);
    } 
    else if (inlineEdit.type === 'rename-var' && v) {
      if (inlineEdit.value.trim()) {
        onUpdateVariable(v.id, { name: inlineEdit.value.trim() }, true);
      }
    }
    else if (inlineEdit.type === 'rename-variant' && v) {
      if (inlineEdit.value.trim()) {
        const newValues = v.values.map((val, idx) => 
          idx === v.activeIndex ? { ...val, name: inlineEdit.value.trim() } : val
        );
        onUpdateVariable(v.id, { values: newValues }, true);
      }
    }
    else if (inlineEdit.type === 'new-variant' && v) {
      if (inlineEdit.value.trim()) {
        const newValues = [...v.values, { 
          id: crypto.randomUUID(), 
          name: inlineEdit.value.trim(), 
          value: inlineEdit.extraValue || '' 
        }];
        onUpdateVariable(v.id, { values: newValues, activeIndex: newValues.length - 1 }, true);
      }
    }
    else if (inlineEdit.type === 'new-var') {
      if (inlineEdit.value.trim()) {
        onAddVariable({
          id: crypto.randomUUID(),
          environmentId: activeEnvId,
          name: inlineEdit.value.trim(),
          values: [
            { id: crypto.randomUUID(), name: 'Default', value: '' },
            { id: crypto.randomUUID(), name: '(auto)', value: '' }
          ],
          activeIndex: 0,
          orderIndex: envVars.length
        });
        setFilter('');
      }
    }

    setInlineEdit(null);
  };

  useHotkeys([
    // 1. Inline Edit Controls
    {
      key: 'Enter',
      enabled: isOpen && !!inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        handleSaveInline();
      }
    },
    {
      key: 'Escape',
      enabled: isOpen && !!inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        setInlineEdit(null);
      }
    },
    // 2. Direct Shortcuts (Only when not inline editing)
    {
      key: 'Enter',
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        setInlineEdit({ type: 'value', id: v.id, value: v.values[v.activeIndex]?.value || '' });
      }
    },
    {
      key: 'f',
      ctrl: true,
      enabled: isOpen && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        if (document.activeElement === filterInputRef.current) {
          filterInputRef.current?.blur();
        } else {
          filterInputRef.current?.focus();
        }
      }
    },
    {
      key: 'n',
      ctrl: true,
      shift: false,
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        setInlineEdit({ type: 'new-variant', id: v.id, value: 'New Variant', extraValue: '' });
      }
    },
    {
      key: 'n',
      ctrl: true,
      shift: true,
      enabled: isOpen && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        setInlineEdit({ type: 'new-var', value: '' });
      }
    },
    {
      key: 'd',
      ctrl: true,
      shift: false,
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        const currentVariant = v.values[v.activeIndex];
        if (v.values.length > 1) {
          if (window.confirm(`Delete variant "${currentVariant.name}"?`)) {
            const newValues = v.values.filter((_, idx) => idx !== v.activeIndex);
            onUpdateVariable(v.id, { values: newValues, activeIndex: 0 }, true);
          }
        }
      }
    },
    {
      key: 'd',
      ctrl: true,
      shift: true,
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        if (window.confirm(`Permanently delete variable "${v.name}"?`)) {
          onDeleteVariable(v.id);
        }
      }
    },
    {
      key: 'r',
      ctrl: true,
      shift: false,
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        const currentVariant = v.values[v.activeIndex];
        if (currentVariant.name === '(auto)') return;
        setInlineEdit({ type: 'rename-variant', id: v.id, value: currentVariant.name });
      }
    },
    {
      key: 'r',
      ctrl: true,
      shift: true,
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        setInlineEdit({ type: 'rename-var', id: v.id, value: v.name });
      }
    },
    // Navigation
    {
      key: 'ArrowDown',
      enabled: isOpen && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        if (e.shiftKey) {
          const currentVar = filtered[selectedIndex];
          if (currentVar) {
            const currentIndexInEnv = envVars.findIndex(v => v.id === currentVar.id);
            if (currentIndexInEnv < envVars.length - 1) {
              const newEnvVars = [...envVars];
              const temp = newEnvVars[currentIndexInEnv];
              newEnvVars[currentIndexInEnv] = newEnvVars[currentIndexInEnv + 1];
              newEnvVars[currentIndexInEnv + 1] = temp;
              onReorder(newEnvVars);
              setSelectedIndex(prev => (prev + 1) % filtered.length);
            }
          }
        } else {
          setSelectedIndex(prev => (prev + 1) % Math.max(1, filtered.length));
        }
      }
    },
    {
      key: 'ArrowUp',
      enabled: isOpen && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        if (e.shiftKey) {
          const currentVar = filtered[selectedIndex];
          if (currentVar) {
            const currentIndexInEnv = envVars.findIndex(v => v.id === currentVar.id);
            if (currentIndexInEnv > 0) {
              const newEnvVars = [...envVars];
              const temp = newEnvVars[currentIndexInEnv];
              newEnvVars[currentIndexInEnv] = newEnvVars[currentIndexInEnv - 1];
              newEnvVars[currentIndexInEnv - 1] = temp;
              onReorder(newEnvVars);
              setSelectedIndex(prev => (prev - 1 + filtered.length) % filtered.length);
            }
          }
        } else {
          setSelectedIndex(prev => (prev - 1 + filtered.length) % Math.max(1, filtered.length));
        }
      }
    },
    {
      key: 'ArrowLeft',
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => {
        if (v && v.values.length > 0) {
          const nextIdx = (v.activeIndex - 1 + v.values.length) % v.values.length;
          onUpdateVariable(v.id, { activeIndex: nextIdx }, true);
        }
      }
    },
    {
      key: 'ArrowRight',
      enabled: isOpen && !!v && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => {
        if (v && v.values.length > 0) {
          const nextIdx = (v.activeIndex + 1) % v.values.length;
          onUpdateVariable(v.id, { activeIndex: nextIdx }, true);
        }
      }
    },
    {
      key: 'Escape',
      enabled: isOpen && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => onClose(),
    },
    {
      key: 'm',
      ctrl: true,
      enabled: isOpen && !inlineEdit,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        onEdit();
        onClose();
      }
    }
  ], [isOpen, selectedIndex, filtered, filter, envVars, v, inlineEdit]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Quick Variable Switcher" maxWidth="lg">
      <div className="p-4 space-y-4">
        <input 
          ref={filterInputRef}
          disabled={!!inlineEdit && inlineEdit.type !== 'new-var'}
          className={`w-full bg-zinc-950 border border-zinc-800 p-3 rounded text-amber-400 font-bold outline-none focus:border-amber-500 transition-colors text-sm font-mono shadow-inner ${!!inlineEdit && inlineEdit.type !== 'new-var' ? 'opacity-30' : ''}`}
          placeholder="Type to filter variables... (^F to focus)"
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        
        <div ref={listRef} className="space-y-1 max-h-96 overflow-y-auto pr-1 custom-scrollbar">
          {/* New Variable Inline Entry */}
          {inlineEdit?.type === 'new-var' && (
            <div className="p-3 rounded flex items-center justify-between border bg-amber-500/10 border-amber-500/50 shadow-lg shadow-amber-500/5">
              <div className="flex flex-col flex-1">
                <span className="text-[9px] font-black uppercase text-amber-500 mb-1">New Variable Name:</span>
                <input
                  ref={editInputRef}
                  className="bg-zinc-950 border border-zinc-800 p-1.5 rounded text-amber-400 font-bold text-xs outline-none focus:border-amber-500 w-full"
                  value={inlineEdit.value}
                  onChange={e => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                  placeholder="variable_name..."
                />
              </div>
            </div>
          )}

          {filtered.map((v, idx) => {
            const isSelected = selectedIndex === idx;
            const isEditingThis = isSelected && inlineEdit;
            
            return (
              <div 
                key={v.id} 
                className={`p-3 rounded flex flex-col gap-2 border transition-all cursor-pointer select-none ${isSelected ? 'bg-amber-500/10 border-amber-500/50 shadow-lg shadow-amber-500/5' : 'bg-transparent border-transparent hover:bg-zinc-900/50'}`}
                onClick={() => !inlineEdit && setSelectedIndex(idx)}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  setSelectedIndex(idx);
                  setInlineEdit({ type: 'value', id: v.id, value: v.values[v.activeIndex]?.value || '' });
                }}
              >
                <div className="flex items-center justify-between">
                  <div className="flex flex-col">
                    {isEditingThis && (inlineEdit.type === 'rename-var' || inlineEdit.type === 'new-variant') ? (
                       <div className="flex flex-col gap-1">
                          <span className="text-[8px] font-black uppercase text-amber-500">
                            {inlineEdit.type === 'rename-var' ? 'Rename Variable' : 'New Variant Name'}
                          </span>
                          <input
                            ref={editInputRef}
                            className="bg-zinc-950 border border-zinc-800 p-1 rounded text-amber-400 font-bold text-[11px] outline-none focus:border-amber-500"
                            value={inlineEdit.value}
                            onChange={e => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                          />
                       </div>
                    ) : (
                      <span 
                        className={`text-xs font-bold ${isSelected ? 'text-amber-400' : 'text-zinc-200'} hover:text-amber-300 transition-colors`}
                        title="Double-click to rename variable"
                        onDoubleClick={(e) => {
                          e.stopPropagation();
                          setSelectedIndex(idx);
                          setInlineEdit({ type: 'rename-var', id: v.id, value: v.name });
                        }}
                      >
                        {v.name}
                      </span>
                    )}
                    
                    <div className="flex items-center gap-2 mt-1">
                      {isEditingThis && inlineEdit.type === 'rename-variant' ? (
                        <div className="flex items-center gap-1">
                           <span className="text-[8px] font-black uppercase text-purple-500">Rename Variant:</span>
                           <input
                              ref={editInputRef}
                              className="bg-zinc-950 border border-zinc-800 p-0.5 px-1 rounded text-purple-400 font-bold text-[10px] outline-none focus:border-purple-500"
                              value={inlineEdit.value}
                              onChange={e => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                           />
                        </div>
                      ) : (
                        <span 
                          className="text-[10px] text-zinc-500 font-mono hover:text-zinc-300 transition-colors"
                          title="Double-click to rename variant"
                          onDoubleClick={(e) => {
                            e.stopPropagation();
                            if (v.values[v.activeIndex]?.name === '(auto)') return;
                            setSelectedIndex(idx);
                            setInlineEdit({ type: 'rename-variant', id: v.id, value: v.values[v.activeIndex]?.name || '' });
                          }}
                        >
                          Variant: <span className="text-purple-400 font-bold">{v.values[v.activeIndex]?.name}</span>
                        </span>
                      )}
                    </div>
                  </div>

                  <div className="flex flex-col items-end gap-1 flex-1 max-w-[60%]">
                    {isEditingThis && (inlineEdit.type === 'value' || inlineEdit.type === 'new-variant') ? (
                      <div className="w-full flex flex-col items-end">
                        <span className="text-[8px] font-black uppercase text-emerald-500 mb-1">
                          {inlineEdit.type === 'value' ? 'Edit Value' : 'Initial Value'}
                        </span>
                        <input
                          ref={inlineEdit.type === 'value' ? editInputRef : undefined}
                          className="w-full bg-zinc-950 border border-zinc-800 p-1.5 rounded text-emerald-text font-mono text-[10px] outline-none focus:border-emerald-500 text-right"
                          value={inlineEdit.type === 'value' ? inlineEdit.value : (inlineEdit.extraValue || '')}
                          onChange={e => {
                            if (inlineEdit.type === 'value') setInlineEdit({ ...inlineEdit, value: e.target.value });
                            else setInlineEdit({ ...inlineEdit, extraValue: e.target.value });
                          }}
                          placeholder="value..."
                          onKeyDown={e => {
                            if (e.key === 'Enter' && inlineEdit.type === 'new-variant') {
                              // special case to allow enter from extraValue field
                              handleSaveInline();
                            }
                          }}
                        />
                      </div>
                    ) : (
                      <div 
                        className="text-[10px] font-mono text-emerald-text truncate w-full text-right bg-zinc-950/50 px-2 py-1 rounded border border-zinc-800/50 hover:border-emerald-500/50 cursor-text transition-colors"
                        title="Double-click to edit value"
                        onDoubleClick={(e) => {
                          e.stopPropagation();
                          setSelectedIndex(idx);
                          setInlineEdit({ type: 'value', id: v.id, value: v.values[v.activeIndex]?.value || '' });
                        }}
                      >
                        {v.values[v.activeIndex]?.value || <span className="opacity-30 italic">empty</span>}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
          
          {filtered.length === 0 && inlineEdit?.type !== 'new-var' && (
            <div className="py-12 text-center border border-dashed border-zinc-800 rounded bg-zinc-900/20">
               <p className="text-[10px] text-zinc-600 uppercase font-black tracking-widest text-balance px-4">
                 No variables matching &quot;{filter}&quot;
               </p>
               <p className="text-[9px] text-zinc-700 mt-1 uppercase font-bold">Press Ctrl+Shift+N to create a new one</p>
            </div>
          )}
        </div>
        
        <div className="grid grid-cols-2 gap-4 pt-4 border-t border-zinc-800 text-[9px] text-zinc-500 font-bold uppercase tracking-widest font-mono">
          <div className="space-y-2">
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">↑↓</kbd> Navigate</div>
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">←→</kbd> Cycle Variant</div>
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">^F</kbd> Focus Filter</div>
          </div>
          <div className="space-y-2">
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">ENTER / 2x Click</kbd> {inlineEdit ? 'Save' : 'Edit Value'}</div>
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">^N</kbd> New Variant <span className="text-[7px] opacity-40 ml-auto">^⇧N for New Var</span></div>
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">^R</kbd> Rename Variant <span className="text-[7px] opacity-40 ml-auto">^⇧R for Ren Var</span></div>
          </div>
        </div>
      </div>
    </Modal>
  );
}

interface EnvironmentQuickSwitcherProps {
  isOpen: boolean;
  onClose: () => void;
  environments: Environment[];
  activeEnvId: string;
  onSetActive: (id: string) => void;
  onEdit: () => void;
}

export function EnvironmentQuickSwitcherModal({ isOpen, onClose, environments, activeEnvId, onSetActive, onEdit }: EnvironmentQuickSwitcherProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isOpen) {
      const idx = environments.findIndex(e => e.id === activeEnvId);
      setSelectedIndex(idx !== -1 ? idx : 0);
      setTimeout(() => containerRef.current?.focus(), 10);
    }
  }, [isOpen, environments, activeEnvId]);

  useEffect(() => {
    if (listRef.current && environments.length > 0) {
      const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedEl) {
        selectedEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }, [selectedIndex, environments.length]);

  useHotkeys([
    {
      key: 'ArrowDown',
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => setSelectedIndex(prev => (prev + 1) % environments.length),
    },
    {
      key: 'ArrowUp',
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => setSelectedIndex(prev => (prev - 1 + environments.length) % environments.length),
    },
    {
      key: 'Enter',
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => {
        const env = environments[selectedIndex];
        if (env) {
          onSetActive(env.id);
          onClose();
        }
      }
    },
    {
      key: 'Escape',
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => onClose(),
    },
    {
      key: 'm',
      ctrl: true,
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        onEdit();
        onClose();
      }
    }
  ], [isOpen, selectedIndex, environments]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Switch Environment" maxWidth="sm">
      <div 
        ref={containerRef}
        className="p-4 space-y-4 outline-none" 
        tabIndex={0} 
      >
        <div ref={listRef} className="space-y-1 max-h-96 overflow-y-auto outline-none pr-1 custom-scrollbar">
          {environments.map((env, idx) => (
            <div 
              key={env.id} 
              className={`p-3 rounded flex items-center justify-between cursor-pointer border outline-none transition-all ${selectedIndex === idx ? 'bg-amber-500/10 border-amber-500/50 shadow-lg shadow-amber-500/5' : 'bg-transparent border-transparent hover:bg-zinc-900/50'}`}
              onClick={() => {
                onSetActive(env.id);
                onClose();
              }}
            >
              <span className={`text-xs font-black uppercase tracking-[0.2em] ${activeEnvId === env.id ? 'text-amber-400' : 'text-zinc-400'}`}>
                {env.name}
              </span>
              {activeEnvId === env.id && <span className="text-[9px] font-black text-amber-500 uppercase px-2 py-0.5 bg-amber-500/10 rounded-full border border-amber-500/30">Active</span>}
            </div>
          ))}
        </div>

        <div className="flex flex-wrap items-center gap-x-6 gap-y-3 pt-4 border-t border-zinc-800 text-[10px] text-zinc-500 font-bold uppercase tracking-widest font-mono">
          <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">↑↓</kbd> Navigate</div>
          <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">Enter</kbd> Select</div>
          <div className="flex items-center gap-2 ml-auto"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">^M</kbd> Manage</div>
        </div>
      </div>
    </Modal>
  );
}
