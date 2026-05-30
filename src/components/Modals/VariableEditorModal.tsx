import React from 'react';
import { GlobalVariable } from '@/hooks/traffic/types';
import { Button, Modal, DebouncedInput } from '../ui';

interface VariableEditorModalProps {
  isOpen: boolean;
  variable: GlobalVariable | null;
  onClose: () => void;
  onUpdate: (id: string, updates: Partial<GlobalVariable>) => void;
  onDelete: (id: string) => void;
}

export function VariableEditorModal({ isOpen, variable, onClose, onUpdate, onDelete }: VariableEditorModalProps) {
  if (!variable) return null;

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Edit Variable: ${variable.name || 'Unnamed'}`}
      maxWidth="lg"
      footer={
        <div className="flex items-center justify-between">
          <Button 
            variant="destructive" 
            size="sm" 
            onClick={() => {
              if (confirm(`Delete variable "${variable.name}"?`)) {
                onDelete(variable.id);
                onClose();
              }
            }}
          >
            Delete Variable
          </Button>
          <Button variant="primary" size="md" onClick={onClose} className="px-8">
            Done
          </Button>
        </div>
      }
    >
      <div className="p-6 flex flex-col gap-6">
        <div className="space-y-2">
          <label className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">Variable Name</label>
          <DebouncedInput
            value={variable.name}
            onChange={(val) => onUpdate(variable.id, { name: val })}
            className="bg-zinc-900 border-zinc-800 rounded w-full"
            inputClassName="text-amber-400 font-bold"
            placeholder="variable_name"
            showIcon={false}
          />
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">Variants & Values</label>
            <button 
              onClick={() => onUpdate(variable.id, { values: [...variable.values, { id: crypto.randomUUID(), name: `Variant ${variable.values.length + 1}`, value: '' }] })}
              className="text-[9px] text-emerald-text font-black uppercase tracking-widest hover:text-emerald-400 transition-colors"
            >
              + Add Variant
            </button>
          </div>
          
          <div className="space-y-3">
            {variable.values.map((val, idx) => (
              <div key={val.id} className="flex items-center gap-2 p-2 bg-zinc-900/30 rounded border border-zinc-800/50">
                <div className="flex flex-col gap-1 w-24">
                  <input
                    value={val.name}
                    disabled={val.name === '(auto)'}
                    onChange={(e) => {
                      const newVals = variable.values.map(vv => vv.id === val.id ? { ...vv, name: e.target.value } : vv);
                      onUpdate(variable.id, { values: newVals });
                    }}
                    className={`bg-transparent border-none text-[10px] font-bold outline-none ${val.name === '(auto)' ? 'text-purple-500' : 'text-sky-400'}`}
                    placeholder="Name"
                  />
                </div>
                <input
                  value={val.value}
                  onChange={(e) => {
                    const newVals = variable.values.map(vv => vv.id === val.id ? { ...vv, value: e.target.value } : vv);
                    onUpdate(variable.id, { values: newVals });
                  }}
                  className="flex-1 bg-transparent border-none text-[11px] text-zinc-300 outline-none font-mono"
                  placeholder="Value..."
                />
                <div className="flex items-center gap-1">
                  <button 
                    onClick={() => onUpdate(variable.id, { activeIndex: idx })}
                    className={`px-2 py-1 rounded text-[8px] font-black tracking-tighter border transition-all ${variable.activeIndex === idx ? 'bg-emerald-500 border-emerald-400 text-zinc-950' : 'bg-transparent border-zinc-800 text-zinc-600 hover:text-zinc-300'}`}
                  >
                    Active
                  </button>
                  {variable.values.length > 1 && val.name !== '(auto)' && (
                    <button 
                      onClick={() => onUpdate(variable.id, { values: variable.values.filter(vv => vv.id !== val.id), activeIndex: 0 })}
                      className="p-1 text-zinc-600 hover:text-rose-500 transition-colors"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </Modal>
  );
}
