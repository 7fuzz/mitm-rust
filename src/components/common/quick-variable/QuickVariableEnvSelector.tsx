import React from 'react';
import { MingCuteIcon } from '../MingCuteIcon';
import type { Environment } from '../../../services/tauri/bridge';

interface QuickVariableEnvSelectorProps {
  environmentsList: Environment[];
  environments: Array<{ id: string; name: string; isActive?: boolean }>;
  activeEnvironmentId: string | null;
  onSelectEnvironment: (envId: string) => void;
}

export const QuickVariableEnvSelector: React.FC<QuickVariableEnvSelectorProps> = ({
  environmentsList,
  environments,
  activeEnvironmentId,
  onSelectEnvironment,
}) => {
  const envsToRender = environmentsList.length > 0 ? environmentsList : environments;

  return (
    <div className="p-2.5 bg-background border-b border-border flex flex-col gap-1.5 shrink-0">
      <div className="flex items-center justify-between">
        <label className="text-3xs font-semibold text-muted-foreground uppercase tracking-wider block font-mono">
          Active Workspace Environments (Press <kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">e</kbd>/<kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">E</kbd> to cycle)
        </label>
      </div>
      <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar py-0.5">
        {envsToRender.map((env) => {
          const isActive = env.id === activeEnvironmentId || ('isActive' in env && (env as any).isActive);
          return (
            <button
              key={env.id}
              type="button"
              onClick={() => onSelectEnvironment(env.id)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold cursor-pointer transition-all shrink-0 ${
                isActive
                  ? 'bg-primary text-primary-foreground border-primary shadow-2xs font-bold'
                  : 'bg-surface hover:bg-neutral-subtle border-border text-foreground'
              }`}
            >
              <MingCuteIcon name="earth_line" size={13} />
              <span>{env.name}</span>
              {isActive && (
                <span className="bg-emerald-500 text-white text-3xs px-1 py-0.2 rounded font-mono font-bold">
                  ACTIVE
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
};
