import React from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { useProxyStore } from '../../stores/useProxyStore';
import { MingCuteIcon } from '../common/MingCuteIcon';

export const StatusBar: React.FC = () => {
  const { dbSizeMb, isSqliteConnected, setQuickVarModalOpen } = useSettingsStore();
  const { activeWorkspaceId, workspaces, activeEnvironmentId, environmentsList, environments } = useWorkspaceStore();
  const { pendingQueue, traffic } = useProxyStore();

  const activeEnvFromList = environmentsList.find((e) => e.id === activeEnvironmentId || e.isActive);
  const activeEnvFromLegacy = environments.find((e) => e.id === activeEnvironmentId);

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId);
  const workspaceName = activeWorkspace?.name || 'Default Workspace';

  const envName = activeEnvFromList?.name || activeEnvFromLegacy?.name || 'No Environment';
  const pendingCount = pendingQueue.length;

  return (
    <footer className="h-6 bg-header border-t border-border flex items-center justify-between px-3 shrink-0 select-none text-[11px] font-mono text-muted-foreground">
      {/* Left Info & Quick Env Trigger (Bottom Left Corner) */}
      <div className="flex items-center gap-3">
        {/* Quick Env Switcher Trigger Button (Corner Left Bottom) */}
        <button
          onClick={() => setQuickVarModalOpen(true)}
          className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-primary/10 hover:bg-primary/20 border border-primary/30 text-primary font-sans font-bold transition-colors cursor-pointer"
          title="Click or press 'V' to open Quick Environment Switcher"
        >
          <MingCuteIcon name="earth_line" size={13} />
          <span>Env: {envName}</span>
          <MingCuteIcon name="down_line" size={12} className="opacity-70" />
        </button>

        <span className="text-border">|</span>

        {/* SQLite Connection */}
        <div className="flex items-center gap-1">
          <span className={`w-1.5 h-1.5 rounded-full ${isSqliteConnected ? 'bg-emerald-500' : 'bg-rose-500'}`} />
          <span>SQLite: {isSqliteConnected ? 'CONNECTED' : 'DISCONNECTED'}</span>
        </div>

        <span className="text-border">|</span>

        {/* Database Size */}
        <div className="flex items-center gap-1">
          <MingCuteIcon name="storage_line" size={12} />
          <span>mitm.db ({dbSizeMb})</span>
        </div>

        <span className="text-border">|</span>

        {/* Traffic Count */}
        <div>Total Captured: {traffic.length} items</div>
      </div>

      {/* Right Badges */}
      <div className="flex items-center gap-3">
        {/* Pending Intercept Counter Badge */}
        {pendingCount > 0 && (
          <div className="flex items-center gap-1 text-rose-500 font-semibold animate-pulse">
            <MingCuteIcon name="shield_line" size={12} />
            <span>{pendingCount} Pending Intercept</span>
          </div>
        )}

        {/* Active Workspace Indicator (Bottom Right Corner) */}
        <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-surface border border-border text-foreground font-sans font-medium text-[11px]">
          <MingCuteIcon name="folder_2_line" size={13} className="text-primary shrink-0" />
          <span className="text-muted-foreground uppercase text-[10px] tracking-wider font-mono">Workspace:</span>
          <span className="font-semibold text-foreground truncate max-w-[160px]">{workspaceName}</span>
        </div>
      </div>
    </footer>
  );
};
