import { useState, useEffect, useRef } from 'react';
import { GlobalVariable, Environment } from '@/hooks/traffic/types';
import { Modal, useDialog } from '../ui';
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
  const listRef = useRef<HTMLDivElement>(null);
  const { prompt, confirm, alert } = useDialog();
  
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
    // 1. Edit Value (Enter)
    {
      key: 'Enter',
      enabled: isOpen && !!v,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        const currentVariant = v.values[v.activeIndex];
        const newValue = await prompt(`Update Variable`, `Value for "${v.name}" [${currentVariant.name}]:`, currentVariant.value);
        if (newValue !== null) {
          const newValues = v.values.map((val, idx) => 
            idx === v.activeIndex ? { ...val, value: newValue } : val
          );
          onUpdateVariable(v.id, { values: newValues }, true);
        }
      }
    },
    // 2. New Variant (Ctrl + N)
    {
      key: 'n',
      ctrl: true,
      shift: false,
      enabled: isOpen && !!v,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        const varName = await prompt(`New Variant`, `Add new variant name for "${v.name}":`, 'New Variant');
        if (varName) {
          const varValue = await prompt(`Variant Value`, `Value for "${varName}":`, '');
          const newValues = [...v.values, { id: crypto.randomUUID(), name: varName, value: varValue || '' }];
          onUpdateVariable(v.id, { values: newValues, activeIndex: newValues.length - 1 }, true);
        }
      }
    },
    // 3. New Variable (Ctrl + Shift + N)
    {
      key: 'n',
      ctrl: true,
      shift: true,
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        const newName = await prompt(`New Variable`, 'Enter variable name:', '');
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
            orderIndex: envVars.length
          });
          setFilter(''); // Clear filter to see the new variable
        }
      }
    },
    // 4. Delete Variant (Ctrl + D)
    {
      key: 'd',
      ctrl: true,
      shift: false,
      enabled: isOpen && !!v,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        const currentVariant = v.values[v.activeIndex];
        if (v.values.length > 1) {
          if (await confirm(`Delete Variant`, `Are you sure you want to delete variant "${currentVariant.name}"?`, true)) {
            const newValues = v.values.filter((_, idx) => idx !== v.activeIndex);
            onUpdateVariable(v.id, { values: newValues, activeIndex: 0 }, true);
          }
        } else {
          await alert("Cannot Delete", "Cannot delete the last variant. Delete the variable instead.");
        }
      }
    },
    // 5. Delete Variable (Ctrl + Shift + D)
    {
      key: 'd',
      ctrl: true,
      shift: true,
      enabled: isOpen && !!v,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        if (await confirm(`Delete Variable`, `Permanently delete variable "${v.name}"?`, true)) {
          onDeleteVariable(v.id);
        }
      }
    },
    // 6. Rename Variant (Ctrl + R)
    {
      key: 'r',
      ctrl: true,
      shift: false,
      enabled: isOpen && !!v,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        const currentVariant = v.values[v.activeIndex];
        if (currentVariant.name === '(auto)') {
          await alert("Protected Variant", "Cannot rename the (auto) variant.");
          return;
        }
        const newName = await prompt(`Rename Variant`, `Rename variant "${currentVariant.name}" to:`, currentVariant.name);
        if (newName && newName !== currentVariant.name) {
          const newValues = v.values.map((val, idx) => 
            idx === v.activeIndex ? { ...val, name: newName } : val
          );
          onUpdateVariable(v.id, { values: newValues }, true);
        }
      }
    },
    // 7. Rename Variable (Ctrl + Shift + R)
    {
      key: 'r',
      ctrl: true,
      shift: true,
      enabled: isOpen && !!v,
      ignoreInputs: false,
      stopPropagation: true,
      handler: async (e) => {
        e.preventDefault();
        const newName = await prompt(`Rename Variable`, `Rename variable "${v.name}" to:`, v.name);
        if (newName && newName !== v.name) {
          onUpdateVariable(v.id, { name: newName }, true);
        }
      }
    },
    // Navigation
    {
      key: 'ArrowDown',
      enabled: isOpen,
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
      enabled: isOpen,
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
      enabled: isOpen && !!v,
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
      enabled: isOpen && !!v,
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
  ], [isOpen, selectedIndex, filtered, filter, envVars, v]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Quick Variable Switcher" maxWidth="lg">
      <div className="p-4 space-y-4">
        <input 
          autoFocus
          className="w-full bg-zinc-950 border border-zinc-800 p-3 rounded text-amber-400 font-bold outline-none focus:border-amber-500 transition-colors text-sm font-mono shadow-inner"
          placeholder="Type to filter variables..."
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        
        <div ref={listRef} className="space-y-1 max-h-96 overflow-y-auto pr-1 custom-scrollbar">
          {filtered.map((v, idx) => (
            <div 
              key={v.id} 
              className={`p-3 rounded flex items-center justify-between cursor-pointer border transition-all ${selectedIndex === idx ? 'bg-amber-500/10 border-amber-500/50 shadow-lg shadow-amber-500/5' : 'bg-transparent border-transparent hover:bg-zinc-900/50'}`}
              onClick={() => setSelectedIndex(idx)}
            >
              <div className="flex flex-col">
                <div className="flex items-center gap-2">
                   <span className={`text-xs font-bold ${selectedIndex === idx ? 'text-amber-400' : 'text-zinc-200'}`}>{v.name}</span>
                </div>
                <span className="text-[10px] text-zinc-500 font-mono">
                  Variant: <span className="text-purple-400 font-bold">{v.values[v.activeIndex]?.name}</span>
                </span>
              </div>
              <div className="text-[10px] font-mono text-emerald-text truncate max-w-40 bg-zinc-950/50 px-2 py-1 rounded border border-zinc-800/50">
                {v.values[v.activeIndex]?.value || <span className="opacity-30 italic">empty</span>}
              </div>
            </div>
          ))}
          {filtered.length === 0 && (
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
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">ENTER</kbd> Edit Value</div>
          </div>
          <div className="space-y-2">
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">^N</kbd> New Variant <span className="text-[7px] opacity-40 ml-auto">^⇧N for New Var</span></div>
            <div className="flex items-center gap-2"><kbd className="bg-zinc-900 px-1.5 py-0.5 rounded border border-zinc-700 text-zinc-300">^D</kbd> Del Variant <span className="text-[7px] opacity-40 ml-auto">^⇧D for Del Var</span></div>
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
