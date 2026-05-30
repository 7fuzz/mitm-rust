import { useState, useEffect, useRef } from 'react';
import { GlobalVariable, Environment } from '@/hooks/traffic/types';
import { Modal } from '../ui/Modal';
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

export function VariableQuickSwitcherModal({ isOpen, onClose, variables, activeEnvId, onUpdateVariable, onReorder, onAddVariable, onDeleteVariable, onEdit }: VariableQuickSwitcherProps) {
  const envVars = variables.filter(v => v.environmentId === activeEnvId);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filter, setFilter] = useState('');
  const [commandMode, setCommandMode] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);
  
  const filtered = envVars.filter(v => v.name.toLowerCase().includes(filter.toLowerCase()));

  useEffect(() => {
    if (isOpen) {
      setSelectedIndex(0);
      setFilter('');
    }
  }, [isOpen]);

  useEffect(() => {
    if (listRef.current && filtered.length > 0) {
      const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedEl) {
        selectedEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }, [selectedIndex, filtered.length]);

  const v = filtered[selectedIndex];

  useHotkeys([
    // Command Mode Actions
    {
      key: 'Escape',
      enabled: isOpen && commandMode,
      stopPropagation: true,
      handler: () => setCommandMode(false),
    },
    {
      key: ['e', 'Enter'],
      enabled: isOpen && commandMode && !!v,
      handler: () => {
        const currentVariant = v.values[v.activeIndex];
        const newValue = window.prompt(`Update value for "${v.name}" [${currentVariant.name}]:`, currentVariant.value);
        if (newValue !== null) {
          const newValues = v.values.map((val, idx) => 
            idx === v.activeIndex ? { ...val, value: newValue } : val
          );
          onUpdateVariable(v.id, { values: newValues }, true);
        }
        setCommandMode(false);
      }
    },
    {
      key: 'v',
      enabled: isOpen && commandMode && !!v,
      handler: () => {
        const varName = window.prompt(`Add new variant name for "${v.name}":`, 'New Variant');
        if (varName) {
          const varValue = window.prompt(`Value for "${varName}":`, '');
          const newValues = [...v.values, { id: crypto.randomUUID(), name: varName, value: varValue || '' }];
          onUpdateVariable(v.id, { values: newValues }, true);
        }
        setCommandMode(false);
      }
    },
    {
      key: 'd',
      enabled: isOpen && commandMode && !!v,
      handler: (e) => {
        if (e.shiftKey) {
          if (window.confirm(`Permanently delete variable "${v.name}"?`)) {
            onDeleteVariable(v.id);
          }
        } else {
          const currentVariant = v.values[v.activeIndex];
          if (v.values.length > 1) {
            if (window.confirm(`Delete variant "${currentVariant.name}"?`)) {
              const newValues = v.values.filter((_, idx) => idx !== v.activeIndex);
              onUpdateVariable(v.id, { values: newValues, activeIndex: 0 }, true);
            }
          } else {
            alert("Cannot delete the last variant. Delete the variable instead.");
          }
        }
        setCommandMode(false);
      }
    },
    {
      key: 'r',
      enabled: isOpen && commandMode && !!v,
      handler: () => {
        const newName = window.prompt(`Rename variable "${v.name}" to:`, v.name);
        if (newName && newName !== v.name) {
          onUpdateVariable(v.id, { name: newName }, true);
        }
        setCommandMode(false);
      }
    },
    {
      key: 'n',
      enabled: isOpen && commandMode,
      handler: () => {
        const newName = window.prompt('New variable name:', '');
        if (newName) {
          onAddVariable({
            id: crypto.randomUUID(),
            environmentId: activeEnvId,
            name: newName,
            values: [
              { id: crypto.randomUUID(), name: 'Default', value: '' },
              { id: crypto.randomUUID(), name: '(auto)', value: '' }
            ],
            activeIndex: 0,
            orderIndex: variables.filter(v => v.environmentId === activeEnvId).length
          });
        }
        setCommandMode(false);
      }
    },
    // Navigation (Available even when typing in filter)
    {
      key: 'ArrowDown',
      enabled: isOpen && !commandMode,
      ignoreInputs: false,
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
          setSelectedIndex(prev => (prev + 1) % filtered.length);
        }
      }
    },
    {
      key: 'ArrowUp',
      enabled: isOpen && !commandMode,
      ignoreInputs: false,
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
          setSelectedIndex(prev => (prev - 1 + filtered.length) % filtered.length);
        }
      }
    },
    {
      key: 'ArrowLeft',
      enabled: isOpen && !commandMode,
      ignoreInputs: false,
      handler: () => {
        if (v && v.values.length > 0) {
          const nextIdx = (v.activeIndex - 1 + v.values.length) % v.values.length;
          onUpdateVariable(v.id, { activeIndex: nextIdx }, true);
        }
      }
    },
    {
      key: 'ArrowRight',
      enabled: isOpen && !commandMode,
      ignoreInputs: false,
      handler: () => {
        if (v && v.values.length > 0) {
          const nextIdx = (v.activeIndex + 1) % v.values.length;
          onUpdateVariable(v.id, { activeIndex: nextIdx }, true);
        }
      }
    },
    {
      key: 'Enter',
      enabled: isOpen && !commandMode,
      ignoreInputs: false,
      handler: () => setCommandMode(true),
    },
    {
      key: 'Escape',
      enabled: isOpen && !commandMode,
      ignoreInputs: false,
      stopPropagation: true,
      handler: () => onClose(),
    },
    {
      key: 'm',
      enabled: isOpen && !commandMode && filter === '',
      handler: () => {
        onEdit();
        onClose();
      }
    }
  ], [isOpen, commandMode, selectedIndex, filtered, filter, envVars]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Quick Variable Switcher" maxWidth="lg">
      <div className="p-4 space-y-4">
        <input 
          autoFocus
          className="w-full bg-zinc-950 border border-zinc-800 p-3 rounded text-amber-400 font-bold outline-none focus:border-amber-500 transition-colors text-sm font-mono"
          placeholder="Filter variables..."
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        
        <div ref={listRef} className="space-y-1 max-h-96 overflow-y-auto">
          {filtered.map((v, idx) => (
            <div 
              key={v.id} 
              className={`p-3 rounded flex items-center justify-between cursor-pointer border ${selectedIndex === idx ? (commandMode ? 'bg-sky-500/10 border-sky-500/50' : 'bg-amber-500/10 border-amber-500/50') : 'bg-transparent border-transparent hover:bg-zinc-900'}`}
              onClick={() => {
                const nextIdx = (v.activeIndex + 1) % v.values.length;
                onUpdateVariable(v.id, { activeIndex: nextIdx }, true);
              }}
            >
              <div className="flex flex-col">
                <div className="flex items-center gap-2">
                   <span className="text-xs font-bold text-zinc-200">{v.name}</span>
                   {commandMode && selectedIndex === idx && (
                     <div className="flex items-center gap-1 animate-in fade-in zoom-in duration-200">
                       <span className="text-[8px] px-1.5 py-0.5 bg-sky-500 text-zinc-950 font-black rounded shadow-lg shadow-sky-500/20">COMMAND:</span>
                       <div className="flex gap-0.5">
                         {['E','V','D','R','N'].map(k => (
                           <span key={k} className="text-[8px] px-1 bg-zinc-800 text-sky-400 font-bold border border-zinc-700 rounded">{k}</span>
                         ))}
                       </div>
                     </div>
                   )}
                </div>
                <span className="text-[10px] text-zinc-500 font-mono">Current: <span className="text-purple-400">{v.values[v.activeIndex]?.name}</span></span>
              </div>
              <div className="text-[10px] font-mono text-emerald-text truncate max-w-40">
                {v.values[v.activeIndex]?.value}
              </div>
            </div>
          ))}
        </div>
        
        <div className="flex flex-wrap items-center gap-x-6 gap-y-3 pt-4 border-t border-zinc-800 text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
          {!commandMode ? (
            <>
              <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">↑↓</kbd> Nav</div>
              <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">⇧↑↓</kbd> Move</div>
              <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">←→</kbd> Cycle</div>
              <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">Enter</kbd> Actions</div>
              <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">M</kbd> Full Edit</div>
            </>
          ) : (
            <>
              <div className="flex items-center gap-2 text-sky-400/80"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-sky-900/50 text-sky-400">E</kbd> Edit Value</div>
              <div className="flex items-center gap-2 text-sky-400/80"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-sky-900/50 text-sky-400">V</kbd> Add Variant</div>
              <div className="flex items-center gap-2 text-sky-400/80"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-sky-900/50 text-sky-400">D</kbd> Del Variant</div>
              <div className="flex items-center gap-2 text-sky-400/80"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-sky-900/50 text-sky-400">R</kbd> Rename</div>
              <div className="flex items-center gap-2 text-sky-400/80"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-sky-900/50 text-sky-400">N</kbd> New Var</div>
              <div className="flex items-center gap-2 text-rose-500/80"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-rose-900/50 text-rose-500">⇧D</kbd> Delete Var</div>
              <div className="flex items-center gap-2 text-zinc-500 ml-auto"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-400">ESC</kbd> Cancel</div>
            </>
          )}
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
      handler: () => setSelectedIndex(prev => (prev + 1) % environments.length),
    },
    {
      key: 'ArrowUp',
      enabled: isOpen,
      ignoreInputs: false,
      handler: () => setSelectedIndex(prev => (prev - 1 + environments.length) % environments.length),
    },
    {
      key: 'Enter',
      enabled: isOpen,
      ignoreInputs: false,
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
      enabled: isOpen,
      handler: () => {
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
        <div ref={listRef} className="space-y-1 max-h-96 overflow-y-auto outline-none">
          {environments.map((env, idx) => (
            <div 
              key={env.id} 
              className={`p-3 rounded flex items-center justify-between cursor-pointer border outline-none ${selectedIndex === idx ? 'bg-amber-500/10 border-amber-500/50' : 'bg-transparent border-transparent hover:bg-zinc-900'}`}
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

        <div className="flex flex-wrap items-center gap-x-6 gap-y-3 pt-4 border-t border-zinc-800 text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
          <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">↑↓</kbd> Navigate</div>
          <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">Enter</kbd> Select</div>
          <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">M</kbd> Manage All</div>
        </div>
      </div>
    </Modal>
  );
}
