import React, { useState, useEffect } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { MingCuteIcon } from './MingCuteIcon';

export const QuickVariableModal: React.FC = () => {
  const { isQuickVarModalOpen, setQuickVarModalOpen } = useSettingsStore();
  const { variables, activeEnvironmentId, environments, addVar } = useWorkspaceStore();

  const [search, setSearch] = useState('');
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [isSecret, setIsSecret] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setQuickVarModalOpen(!isQuickVarModalOpen);
      }
      if (e.key === 'Escape' && isQuickVarModalOpen) {
        setQuickVarModalOpen(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isQuickVarModalOpen, setQuickVarModalOpen]);

  if (!isQuickVarModalOpen) return null;

  const currentEnv = environments.find((e) => e.id === activeEnvironmentId) || environments[0];
  const filteredVars = variables.filter(
    (v) =>
      v.key.toLowerCase().includes(search.toLowerCase()) ||
      v.value.toLowerCase().includes(search.toLowerCase())
  );

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKey.trim()) return;
    await addVar({
      key: newKey.trim().toUpperCase(),
      value: newValue,
      environmentId: activeEnvironmentId,
      isSecret,
      type: 'string',
    });
    setNewKey('');
    setNewValue('');
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/60 backdrop-blur-xs animate-in fade-in duration-150">
      <div className="w-full max-w-xl bg-surface border border-border rounded-lg shadow-2xl overflow-hidden flex flex-col max-h-[80vh]">
        {/* Modal Header */}
        <div className="p-3 border-b border-border flex items-center gap-2 bg-header">
          <MingCuteIcon name="search_line" className="text-muted-foreground" size={18} />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={`Search variables in ${currentEnv?.name} (e.g. {{AUTH_TOKEN}})...`}
            className="w-full bg-transparent text-sm focus:outline-none text-foreground placeholder:text-muted-foreground"
            autoFocus
          />
          <button
            onClick={() => setQuickVarModalOpen(false)}
            className="p-1 text-muted-foreground hover:text-foreground rounded"
          >
            <MingCuteIcon name="close_line" size={16} />
          </button>
        </div>

        {/* Variables List */}
        <div className="p-3 overflow-y-auto space-y-2 flex-1">
          {filteredVars.length === 0 ? (
            <div className="py-6 text-center text-xs text-muted-foreground italic">
              No matching variables found
            </div>
          ) : (
            filteredVars.map((v) => (
              <div
                key={v.id}
                className="p-2 border border-border/60 rounded bg-background/50 hover:border-primary/50 transition-colors flex items-center justify-between"
              >
                <div className="flex items-center gap-2 font-mono text-xs">
                  <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary font-semibold">
                    {`{{${v.key}}}`}
                  </span>
                  <span className="text-muted-foreground font-sans text-[11px] truncate max-w-[200px]">
                    {v.value ? (v.isSecret ? '••••••••' : v.value) : '<empty>'}
                  </span>
                </div>
                <span
                  className="px-1.5 py-0.5 rounded text-[10px] uppercase font-mono font-medium border"
                  style={{
                    backgroundColor: `${v.environmentId === 'env-global' ? '#94a3b8' : currentEnv?.color || '#38bdf8'}15`,
                    color: v.environmentId === 'env-global' ? '#94a3b8' : currentEnv?.color || '#38bdf8',
                    borderColor: `${v.environmentId === 'env-global' ? '#94a3b8' : currentEnv?.color || '#38bdf8'}30`,
                  }}
                >
                  {v.environmentId === 'env-global' ? 'Global' : currentEnv?.name}
                </span>
              </div>
            ))
          )}
        </div>

        {/* Create Variable Quick Form */}
        <form onSubmit={handleCreate} className="p-3 border-t border-border bg-header flex items-center gap-2">
          <input
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="KEY_NAME"
            className="w-1/3 bg-background border border-border rounded px-2 py-1 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
          />
          <input
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="Value..."
            className="w-1/2 bg-background border border-border rounded px-2 py-1 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
          />
          <label className="flex items-center gap-1 text-[11px] text-muted-foreground cursor-pointer select-none">
            <input
              type="checkbox"
              checked={isSecret}
              onChange={(e) => setIsSecret(e.target.checked)}
              className="rounded border-border text-primary"
            />
            Mask
          </label>
          <button
            type="submit"
            className="px-3 py-1 bg-primary text-primary-foreground text-xs font-medium rounded hover:bg-primary-hover transition-colors shrink-0"
          >
            Add
          </button>
        </form>
      </div>
    </div>
  );
};
