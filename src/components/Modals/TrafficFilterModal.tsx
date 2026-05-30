import { useState, useEffect, useRef } from 'react';
import { Modal } from '../ui/Modal';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { useHotkeys } from '@/hooks/ui/useHotkeys';

interface FilterRule {
  id: string;
  is_active: boolean;
  rule_type: string;
  mode: 'whitelist' | 'blacklist';
  pattern: string;
}

interface FilterConfig {
  rules: FilterRule[];
}

interface Props {
  isOpen: boolean;
  onClose: () => void;
  config: FilterConfig;
  onSave: (config: FilterConfig) => void;
}

export function TrafficFilterModal({ isOpen, onClose, config, onSave }: Props) {
  const [localRules, setLocalRules] = useState<FilterRule[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isOpen) {
      setLocalRules([...config.rules]);
      setSelectedIndex(0);
    }
  }, [isOpen, config.rules]);

  useEffect(() => {
    if (listRef.current && localRules.length > 0) {
      const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedEl) {
        selectedEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }, [selectedIndex, localRules.length]);

  const addRule = () => {
    const newRule: FilterRule = { id: crypto.randomUUID(), is_active: true, rule_type: 'contains', mode: 'blacklist', pattern: '' };
    const next = [...localRules, newRule];
    setLocalRules(next);
    setSelectedIndex(next.length - 1);
  };

  const removeRule = (id: string) => {
    const next = localRules.filter(r => r.id !== id);
    setLocalRules(next);
    setSelectedIndex(prev => Math.min(prev, Math.max(0, next.length - 1)));
  };

  const updateRule = (id: string, updates: Partial<FilterRule>) => {
    setLocalRules(localRules.map(r => r.id === id ? { ...r, ...updates } : r));
  };

  const handleSave = () => {
    onSave({ rules: localRules });
    onClose();
  };

  const selectedRule = localRules[selectedIndex];

  useHotkeys([
    {
      key: 'ArrowDown',
      enabled: isOpen,
      ignoreInputs: false,
      handler: (e) => {
        if (!e.metaKey && !e.ctrlKey) {
          setSelectedIndex(prev => (prev + 1) % Math.max(1, localRules.length));
        }
      }
    },
    {
      key: 'ArrowUp',
      enabled: isOpen,
      ignoreInputs: false,
      handler: (e) => {
        if (!e.metaKey && !e.ctrlKey) {
          setSelectedIndex(prev => (prev - 1 + Math.max(1, localRules.length)) % Math.max(1, localRules.length));
        }
      }
    },
    {
      key: 'n',
      enabled: isOpen,
      handler: (e) => {
        e.preventDefault();
        addRule();
      }
    },
    {
      key: 'd',
      enabled: isOpen && !!selectedRule,
      handler: () => removeRule(selectedRule.id)
    },
    {
      key: ' ',
      enabled: isOpen && !!selectedRule,
      ignoreInputs: false,
      handler: (e) => {
        // Toggle active status
        if (document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'SELECT') {
          e.preventDefault();
          updateRule(selectedRule.id, { is_active: !selectedRule.is_active });
        }
      }
    },
    {
      key: 'w',
      enabled: isOpen && !!selectedRule,
      handler: () => updateRule(selectedRule.id, { mode: 'whitelist' })
    },
    {
      key: 'b',
      enabled: isOpen && !!selectedRule,
      handler: () => updateRule(selectedRule.id, { mode: 'blacklist' })
    },
    {
      key: 'Enter',
      enabled: isOpen,
      ignoreInputs: false,
      handler: (e) => {
        if (e.metaKey || e.ctrlKey) {
          handleSave();
        }
      }
    }
  ], [isOpen, localRules, selectedIndex]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Traffic Filter Engine" maxWidth="lg">
      <div className="p-4 space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <h4 className="text-[10px] font-black uppercase tracking-[0.2em] text-zinc-500">Filter Pipeline</h4>
            <p className="text-[10px] text-zinc-600 font-mono italic">Whitelist: ALLOW only matches. Blacklist: IGNORE matches.</p>
          </div>
          <Button variant="secondary" size="sm" onClick={addRule} className="text-[10px] uppercase font-bold tracking-widest">+ Add Rule</Button>
        </div>

        <div ref={listRef} className="space-y-1.5 max-h-[450px] overflow-y-auto pr-1 custom-scrollbar min-h-[200px]">
          {localRules.length === 0 && (
            <div className="flex flex-col items-center justify-center py-12 border border-dashed border-zinc-800 rounded bg-zinc-950/20 opacity-50">
              <span className="text-[10px] font-black uppercase tracking-[0.3em] text-zinc-700">Pipeline Empty</span>
              <span className="text-[9px] text-zinc-800 mt-1 uppercase font-bold">Capturing all traffic by default</span>
            </div>
          )}
          {localRules.map((rule, idx) => (
            <div 
              key={rule.id} 
              className={`p-2 rounded flex items-center gap-3 border transition-all ${selectedIndex === idx ? 'bg-zinc-900/50 border-zinc-700 ring-1 ring-zinc-800' : 'bg-transparent border-transparent hover:bg-zinc-900/30'}`}
              onClick={() => setSelectedIndex(idx)}
            >
              <input
                type="checkbox"
                checked={rule.is_active}
                onChange={(e) => updateRule(rule.id, { is_active: e.target.checked })}
                className="w-3.5 h-3.4 rounded bg-zinc-950 border-zinc-800 text-emerald-500 focus:ring-emerald-500/20"
              />
              
              <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800 shrink-0">
                <button 
                  onClick={() => updateRule(rule.id, { mode: 'whitelist' })}
                  className={`px-2 py-0.5 text-[8px] font-black uppercase rounded transition-all ${rule.mode === 'whitelist' ? 'bg-emerald-highlight-bg text-emerald-text' : 'text-zinc-600 hover:text-zinc-400'}`}
                >
                  Allow
                </button>
                <button 
                  onClick={() => updateRule(rule.id, { mode: 'blacklist' })}
                  className={`px-2 py-0.5 text-[8px] font-black uppercase rounded transition-all ${rule.mode === 'blacklist' ? 'bg-rose-highlight-bg text-rose-text' : 'text-zinc-600 hover:text-zinc-400'}`}
                >
                  Block
                </button>
              </div>

              <Select
                value={rule.rule_type}
                onChange={(val) => updateRule(rule.id, { rule_type: val })}
                options={[
                  { value: 'contains', label: 'CONTAINS' },
                  { value: 'starts_with', label: 'STARTS' },
                  { value: 'ends_with', label: 'ENDS' },
                  { value: 'exact', label: 'EXACT' }
                ]}
                className="w-24 text-[9px] font-black"
              />

              <Input
                value={rule.pattern}
                onChange={(e) => updateRule(rule.id, { pattern: e.target.value })}
                placeholder="https://api.example.com/..."
                className={`flex-1 text-[11px] font-mono !py-1 ${!rule.is_active ? 'opacity-30' : ''}`}
                autoFocus={selectedIndex === idx && rule.pattern === ''}
              />

              <button
                onClick={() => removeRule(rule.id)}
                className="p-1 text-zinc-700 hover:text-rose-500 transition-colors"
                title="Remove Rule"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
              </button>
            </div>
          ))}
        </div>

        <div className="flex flex-wrap items-center gap-x-5 gap-y-2 pt-4 border-t border-zinc-800 text-[9px] text-zinc-500 font-bold uppercase tracking-widest font-mono">
          <div className="flex items-center gap-1.5"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-700 text-zinc-300">↑↓</kbd> Nav</div>
          <div className="flex items-center gap-1.5"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-700 text-zinc-300">SPACE</kbd> Toggle</div>
          <div className="flex items-center gap-1.5"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-700 text-zinc-300">W</kbd> Whitelist</div>
          <div className="flex items-center gap-1.5"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-700 text-zinc-300">B</kbd> Blacklist</div>
          <div className="flex items-center gap-1.5"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-700 text-zinc-300">N</kbd> New</div>
          <div className="flex items-center gap-1.5"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-700 text-zinc-300">D</kbd> Del</div>
          <div className="flex items-center gap-1.5 ml-auto text-sky-400/80 font-black"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-sky-900/50 text-sky-400">⌘↵</kbd> Save & Close</div>
        </div>
      </div>
    </Modal>
  );
}
