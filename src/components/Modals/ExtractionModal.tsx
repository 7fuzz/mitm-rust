import { useState } from 'react';
import { GlobalVariable } from '@/hooks/traffic';
import { Button, Input, Modal } from '../ui';

interface ExtractionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (rules: Record<string, string>) => void;
  initialRules: Record<string, string>;
  availableVariables: GlobalVariable[];
}

export function ExtractionModal({ isOpen, onClose, onSave, initialRules, availableVariables }: ExtractionModalProps) {
  const [rules, setRules] = useState<{ id: string; varName: string; path: string }[]>([]);
  const [prevOpen, setPrevOpen] = useState(isOpen);
  const [prevInitialRules, setPrevInitialRules] = useState(initialRules);

  if (isOpen && (!prevOpen || initialRules !== prevInitialRules)) {
    setPrevOpen(true);
    setPrevInitialRules(initialRules);
    const initial = Object.entries(initialRules).map(([varName, path]) => ({
      id: crypto.randomUUID(),
      varName,
      path
    }));
    setRules(initial.length > 0 ? initial : []);
  } else if (!isOpen && prevOpen) {
    setPrevOpen(false);
  }

  const addRule = () => {
    setRules([...rules, { id: crypto.randomUUID(), varName: '', path: '' }]);
  };

  const updateRule = (id: string, field: 'varName' | 'path', value: string) => {
    setRules(rules.map(r => r.id === id ? { ...r, [field]: value } : r));
  };

  const removeRule = (id: string) => {
    setRules(rules.filter(r => r.id !== id));
  };

  const handleSave = () => {
    const finalRules: Record<string, string> = {};
    rules.forEach(r => {
      if (r.varName.trim() && r.path.trim()) {
        finalRules[r.varName.trim()] = r.path.trim();
      }
    });
    onSave(finalRules);
    onClose();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Extraction Rules Configuration"
      maxWidth="lg"
      footer={
        <div className="flex justify-end gap-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
          >
            Cancel
          </Button>
          <Button
            variant="amber"
            size="md"
            onClick={handleSave}
            className="px-6"
          >
            Apply Rules
          </Button>
        </div>
      }
    >
      <div className="p-6 flex flex-col gap-4">
        <p className="text-[10px] text-zinc-500 font-mono leading-relaxed">
          Define rules to automatically extract data from JSON responses into your environment variables. 
          Paths use dot notation (e.g., <span className="text-zinc-300 font-bold">data.user.id</span>).
        </p>

        <div className="space-y-3 max-h-80 overflow-y-auto pr-2 scrollbar-thin scrollbar-thumb-zinc-800">
          {rules.length === 0 ? (
            <div className="text-center py-10 border-2 border-dashed border-zinc-900 rounded-lg">
              <span className="text-zinc-600 text-[10px] uppercase font-bold tracking-widest">No rules defined yet</span>
            </div>
          ) : (
            rules.map((rule) => (
              <div key={rule.id} className="flex items-center gap-2 group">
                <div className="flex-1">
                  <select
                    value={rule.varName}
                    onChange={(e) => updateRule(rule.id, 'varName', e.target.value)}
                    className="w-full bg-zinc-900 border border-zinc-800 p-2 rounded text-emerald-text text-xs font-mono outline-none focus:border-amber-500/50 transition-colors"
                  >
                    <option value="">Select Variable...</option>
                    {availableVariables.map(v => (
                      <option key={v.id} value={v.name}>{v.name}</option>
                    ))}
                  </select>
                </div>
                <span className="text-zinc-700 text-xs">←</span>
                <div className="flex-[1.5]">
                  <Input
                    type="text"
                    value={rule.path}
                    onChange={(e) => updateRule(rule.id, 'path', e.target.value)}
                    placeholder="Response Path (e.g. result.token)"
                    className="bg-zinc-900"
                  />
                </div>
                <button
                  onClick={() => removeRule(rule.id)}
                  className="p-2 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded transition-colors"
                >
                  ✕
                </button>
              </div>
            ))
          )}
        </div>

        <button
          onClick={addRule}
          className="w-full py-2 border border-dashed border-zinc-800 hover:border-zinc-700 hover:bg-zinc-900/50 text-zinc-500 hover:text-zinc-300 text-[10px] font-bold uppercase tracking-widest rounded transition-all mt-2"
        >
          + Add New Extraction Rule
        </button>
      </div>
    </Modal>
  );
}
