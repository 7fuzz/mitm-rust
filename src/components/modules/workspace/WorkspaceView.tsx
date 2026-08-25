import React, { useState } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { VariablesManager } from './VariablesManager';
import { ReplacementRules } from './ReplacementRules';
import { CollectionLinking } from './CollectionLinking';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const WorkspaceView: React.FC = () => {
  const { environments, activeEnvironmentId, setActiveEnv, createEnv, deleteEnv } = useWorkspaceStore();

  const [isAddingEnv, setIsAddingEnv] = useState(false);
  const [newEnvName, setNewEnvName] = useState('');
  const [newEnvColor, setNewEnvColor] = useState('#38bdf8');

  const handleCreateEnv = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newEnvName.trim()) return;
    await createEnv(newEnvName.trim(), newEnvColor);
    setNewEnvName('');
    setIsAddingEnv(false);
  };

  return (
    <div className="h-full flex flex-col bg-background p-4 overflow-y-auto space-y-4">
      {/* Environment Switcher Bar */}
      <div className="bg-surface border border-border rounded-lg p-3 flex items-center justify-between gap-3 shadow-2xs text-xs">
        <div className="flex items-center gap-3">
          <span className="font-semibold text-foreground text-sm">Active Environment:</span>

          <div className="flex items-center gap-1.5 flex-wrap">
            {environments.map((env) => {
              const isActive = activeEnvironmentId === env.id;
              return (
                <button
                  key={env.id}
                  onClick={() => setActiveEnv(env.id)}
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md font-mono text-xs transition-all font-medium border ${
                    isActive
                      ? 'bg-background shadow-xs font-bold border-border'
                      : 'bg-neutral-subtle/50 border-transparent text-muted-foreground hover:text-foreground'
                  }`}
                >
                  <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: env.color || '#38bdf8' }} />
                  <span>{env.name}</span>
                </button>
              );
            })}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {!isAddingEnv ? (
            <button
              onClick={() => setIsAddingEnv(true)}
              className="flex items-center gap-1 px-3 py-1.5 rounded bg-primary text-primary-foreground font-medium hover:bg-primary-hover transition-colors shadow-xs"
            >
              <MingCuteIcon name="plus_line" size={14} />
              <span>New Environment</span>
            </button>
          ) : (
            <form onSubmit={handleCreateEnv} className="flex items-center gap-2">
              <input
                type="text"
                value={newEnvName}
                onChange={(e) => setNewEnvName(e.target.value)}
                placeholder="Env Name..."
                className="bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
                autoFocus
              />
              <input
                type="color"
                value={newEnvColor}
                onChange={(e) => setNewEnvColor(e.target.value)}
                className="w-6 h-6 rounded cursor-pointer border border-border bg-transparent p-0"
              />
              <button type="submit" className="p-1.5 bg-emerald-600 text-white rounded hover:bg-emerald-500">
                <MingCuteIcon name="check_line" size={14} />
              </button>
              <button type="button" onClick={() => setIsAddingEnv(false)} className="p-1.5 bg-neutral-subtle text-muted-foreground rounded hover:text-foreground">
                <MingCuteIcon name="close_line" size={14} />
              </button>
            </form>
          )}

          {activeEnvironmentId !== 'env-global' && (
            <button
              onClick={() => deleteEnv(activeEnvironmentId)}
              className="p-1.5 rounded bg-rose-500/10 border border-rose-500/20 text-rose-500 hover:bg-rose-500/20 transition-colors"
              title="Delete Active Environment"
            >
              <MingCuteIcon name="delete_2_line" size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Variables Table Manager */}
      <VariablesManager />

      {/* Automated Replacement Rules */}
      <ReplacementRules />

      {/* Collection Linking UI */}
      <CollectionLinking />
    </div>
  );
};
