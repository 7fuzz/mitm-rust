import { useState, useEffect, useRef } from 'react';
import { Modal } from '../ui/Modal';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { useHotkeys } from '@/hooks/ui/useHotkeys';

interface FilterRule {
  id: string;
  is_active: boolean;
  field: string; // 'url' | 'method' | 'status_code'
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

const METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'OPTIONS', 'HEAD', 'CONNECT', 'TRACE'];

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
    const newRule: FilterRule = { id: crypto.randomUUID(), is_active: true, field: 'url', rule_type: 'contains', mode: 'blacklist', pattern: '' };
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

  const handleFieldChange = (id: string, field: string) => {
    const updates: Partial<FilterRule> = { field };
    if (field === 'method') {
      updates.rule_type = 'exact';
      updates.pattern = 'GET';
    } else if (field === 'status_code') {
      updates.rule_type = 'starts_with';
      updates.pattern = '2'; // Success (2xx)
    } else {
      updates.rule_type = 'contains';
      updates.pattern = '';
    }
    updateRule(id, updates);
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
      stopPropagation: true,
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
      stopPropagation: true,
      handler: (e) => {
        if (!e.metaKey && !e.ctrlKey) {
          setSelectedIndex(prev => (prev - 1 + Math.max(1, localRules.length)) % Math.max(1, localRules.length));
        }
      }
    },
    {
      key: 'n',
      enabled: isOpen,
      stopPropagation: true,
      handler: (e) => {
        e.preventDefault();
        addRule();
      }
    },
    {
      key: 'd',
      enabled: isOpen && !!selectedRule,
      stopPropagation: true,
      handler: () => removeRule(selectedRule.id)
    },
    {
      key: ' ',
      enabled: isOpen && !!selectedRule,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        // Toggle active status
        if (document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'SELECT') {
          e.preventDefault();
          updateRule(selectedRule.id, { is_active: !selectedRule.is_active });
        }
      }
    },
    {
      key: 'Enter',
      enabled: isOpen,
      ignoreInputs: false,
      stopPropagation: true,
      handler: (e) => {
        if (e.metaKey || e.ctrlKey) {
          handleSave();
        }
      }
    }
  ], [isOpen, localRules, selectedIndex]);

  return (
    <Modal 
      isOpen={isOpen} 
      onClose={onClose} 
      title="Traffic Filter Engine" 
      maxWidth="3xl"
      footer={
        <div className="flex items-center justify-between w-full">
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[9px] text-zinc-500 font-bold uppercase tracking-widest font-mono">
            <div className="flex items-center gap-1"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-800 text-zinc-400">↑↓</kbd> Nav</div>
            <div className="flex items-center gap-1"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-800 text-zinc-400">N</kbd> New</div>
            <div className="flex items-center gap-1"><kbd className="bg-zinc-900 px-1 py-0.5 rounded border border-zinc-800 text-zinc-400">D</kbd> Del</div>
          </div>

          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={onClose} className="text-[10px] font-bold uppercase tracking-widest">Cancel</Button>
            <Button variant="purple" size="sm" onClick={handleSave} className="text-[10px] font-bold uppercase tracking-widest px-6 shadow-lg shadow-purple-500/10">Save Pipeline</Button>
          </div>
        </div>
      }
    >
      <div className="p-4 flex flex-col min-h-[400px]">
        <div className="flex items-center justify-between mb-4 shrink-0">
          <div className="space-y-0.5">
            <h4 className="text-[10px] font-black uppercase tracking-[0.2em] text-zinc-500">Filter Pipeline</h4>
            <p className="text-[10px] text-zinc-600 font-mono italic">Whitelist: ALLOW only matches. Blacklist: IGNORE matches.</p>
          </div>
          <Button variant="purple" size="sm" onClick={addRule} className="text-[10px] uppercase font-bold tracking-widest">+ Add Rule</Button>
        </div>

        <div ref={listRef} className="space-y-2">
          {localRules.length === 0 && (
            <div className="flex flex-col items-center justify-center py-20 border border-dashed border-zinc-800 rounded bg-zinc-950/20 opacity-50">
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
                className="w-4 h-4 rounded bg-zinc-950 border-zinc-800 text-purple-500 focus:ring-purple-500/20"
              />
              
              <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800 shrink-0">
                <button 
                  onClick={() => updateRule(rule.id, { mode: 'whitelist' })}
                  className={`px-3 py-1 text-[8px] font-black uppercase rounded transition-all ${rule.mode === 'whitelist' ? 'bg-emerald-500/20 text-emerald-400' : 'text-zinc-600 hover:text-zinc-400'}`}
                >
                  Allow
                </button>
                <button 
                  onClick={() => updateRule(rule.id, { mode: 'blacklist' })}
                  className={`px-3 py-1 text-[8px] font-black uppercase rounded transition-all ${rule.mode === 'blacklist' ? 'bg-rose-500/20 text-rose-400' : 'text-zinc-600 hover:text-zinc-400'}`}
                >
                  Block
                </button>
              </div>

              <div className="flex items-center gap-1 shrink-0">
                <Select
                  value={rule.field || 'url'}
                  onChange={(val) => handleFieldChange(rule.id, val)}
                  options={[
                    { value: 'url', label: 'URL' },
                    { value: 'method', label: 'Method' },
                    { value: 'status_code', label: 'Status' }
                  ]}
                  className="w-24 text-[9px] font-black"
                />
                
                {rule.field === 'url' ? (
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
                ) : rule.field === 'status_code' ? (
                  <Select
                    value={rule.rule_type === 'starts_with' ? `range_${rule.pattern}` : 'exact'}
                    onChange={(val) => {
                      if (val.startsWith('range_')) {
                        updateRule(rule.id, { rule_type: 'starts_with', pattern: val.replace('range_', '') });
                      } else {
                        updateRule(rule.id, { rule_type: 'exact', pattern: '' });
                      }
                    }}
                    options={[
                      { value: 'range_2', label: '2xx (Success)', color: 'text-emerald-400' },
                      { value: 'range_3', label: '3xx (Redirect)', color: 'text-sky-400' },
                      { value: 'range_4', label: '4xx (Client Err)', color: 'text-amber-400' },
                      { value: 'range_5', label: '5xx (Server Err)', color: 'text-rose-400' },
                      { value: 'exact', label: 'Precise Code' }
                    ]}
                    className="w-32 text-[9px] font-black"
                  />
                ) : (
                   <div className="w-24 text-[9px] font-black text-zinc-500 bg-zinc-950/50 border border-zinc-800 rounded px-3 py-1.5 uppercase tracking-widest flex items-center justify-center">EXACT</div>
                )}
              </div>

              {rule.field === 'method' ? (
                <Select
                  value={rule.pattern}
                  onChange={(val) => updateRule(rule.id, { pattern: val })}
                  options={METHODS.map(m => ({ value: m, label: m }))}
                  className="flex-1"
                />
              ) : rule.field === 'status_code' && rule.rule_type === 'starts_with' ? (
                <div className="flex-1 text-[11px] font-mono text-zinc-500 italic bg-zinc-900/20 px-3 py-1.5 border border-transparent">
                  Filtering {rule.pattern}xx status codes
                </div>
              ) : (
                <Input
                  value={rule.pattern}
                  onChange={(e) => updateRule(rule.id, { pattern: e.target.value })}
                  placeholder={rule.field === 'status_code' ? 'Enter code (e.g. 200)...' : 'https://api.example.com/...'}
                  className={`flex-1 text-[11px] font-mono !py-1 ${!rule.is_active ? 'opacity-30' : ''}`}
                  autoFocus={selectedIndex === idx && rule.pattern === ''}
                  type={rule.field === 'status_code' ? 'number' : 'text'}
                />
              )}

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
      </div>
    </Modal>
  );
}
