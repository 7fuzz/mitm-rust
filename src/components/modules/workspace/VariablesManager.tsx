import React, { useState } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const VariablesManager: React.FC = () => {
  const { variables, activeEnvironmentId, environments, addVar, updateVar, deleteVar } =
    useWorkspaceStore();

  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [isSecret, setIsSecret] = useState(false);
  const [revealedIds, setRevealedIds] = useState<Record<string, boolean>>({});

  const currentEnv = environments.find((e) => e.id === activeEnvironmentId) || environments[0];

  const envVariables = variables.filter(
    (v) => v.environmentId === activeEnvironmentId || v.environmentId === 'env-global'
  );

  const toggleReveal = (id: string) => {
    setRevealedIds((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKey.trim()) return;
    await addVar({
      key: newKey.trim(),
      value: newValue,
      environmentId: activeEnvironmentId,
      isSecret,
      type: 'string',
    });
    setNewKey('');
    setNewValue('');
  };

  return (
    <div className="bg-surface border border-border rounded-lg p-3 space-y-3 text-xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <MingCuteIcon name="key_line" size={16} className="text-primary" />
          <span className="font-semibold text-foreground text-sm">
            Environment Variables ({currentEnv?.name})
          </span>
        </div>
      </div>

      {/* Grid Editor Table */}
      <div className="border border-border rounded overflow-hidden">
        <table className="w-full text-left border-collapse font-mono text-xs">
          <thead>
            <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
              <th className="px-3 py-1.5 font-medium">Variable Key</th>
              <th className="px-3 py-1.5 font-medium">Current Value</th>
              <th className="px-3 py-1.5 font-medium">Scope</th>
              <th className="w-16 px-2 py-1.5 text-center">Mask</th>
              <th className="w-12 px-2 py-1.5 text-center">Action</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {envVariables.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-3 py-4 text-center text-muted-foreground italic font-sans">
                  No variables defined in this scope
                </td>
              </tr>
            ) : (
              envVariables.map((v) => {
                const isRevealed = revealedIds[v.id];
                return (
                  <tr key={v.id} className="hover:bg-neutral-subtle/50 transition-colors">
                    <td className="px-3 py-1.5 font-bold text-primary">{`{{${v.key}}}`}</td>
                    <td className="px-3 py-1.5">
                      <input
                        type={v.isSecret && !isRevealed ? 'password' : 'text'}
                        value={v.value}
                        onChange={(e) => updateVar({ ...v, value: e.target.value })}
                        className="w-full bg-transparent px-1 py-0.5 text-foreground focus:outline-none focus:bg-background border border-transparent focus:border-border rounded"
                      />
                    </td>
                    <td className="px-3 py-1.5">
                      <span
                        className="px-1.5 py-0.2 rounded text-[10px] uppercase font-bold border"
                        style={{
                          backgroundColor: `${v.environmentId === 'env-global' ? '#94a3b8' : currentEnv?.color || '#38bdf8'}15`,
                          color: v.environmentId === 'env-global' ? '#94a3b8' : currentEnv?.color || '#38bdf8',
                          borderColor: `${v.environmentId === 'env-global' ? '#94a3b8' : currentEnv?.color || '#38bdf8'}30`,
                        }}
                      >
                        {v.environmentId === 'env-global' ? 'Global' : currentEnv?.name}
                      </span>
                    </td>
                    <td className="px-2 py-1.5 text-center">
                      {v.isSecret && (
                        <button
                          onClick={() => toggleReveal(v.id)}
                          className="p-1 text-muted-foreground hover:text-foreground rounded"
                          title={isRevealed ? 'Mask secret value' : 'Show secret value'}
                        >
                          <MingCuteIcon name={isRevealed ? 'eye_line' : 'eye_close_line'} size={14} />
                        </button>
                      )}
                    </td>
                    <td className="px-2 py-1.5 text-center">
                      <button
                        onClick={() => deleteVar(v.id)}
                        className="p-1 text-muted-foreground hover:text-rose-500 rounded"
                      >
                        <MingCuteIcon name="delete_2_line" size={14} />
                      </button>
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* Quick Add Form */}
      <form onSubmit={handleAdd} className="flex items-center gap-2 pt-1 font-mono">
        <input
          type="text"
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          placeholder="VARIABLE_KEY"
          className="w-1/3 bg-background border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <input
          type="text"
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          placeholder="Variable Value..."
          className="w-1/2 bg-background border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <label className="flex items-center gap-1 text-[11px] text-muted-foreground cursor-pointer select-none font-sans">
          <input
            type="checkbox"
            checked={isSecret}
            onChange={(e) => setIsSecret(e.target.checked)}
            className="rounded border-border text-primary"
          />
          Secret
        </label>
        <button
          type="submit"
          className="px-3 py-1 bg-primary text-primary-foreground text-xs font-sans font-medium rounded hover:bg-primary-hover transition-colors shrink-0"
        >
          Add Variable
        </button>
      </form>
    </div>
  );
};
