import React from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { useProxyStore } from '../../stores/useProxyStore';
import { MingCuteIcon } from '../common/MingCuteIcon';

export const StatusBar: React.FC = () => {
  const { dbSizeMb, isSqliteConnected } = useSettingsStore();
  const { activeEnvironmentId, environments } = useWorkspaceStore();
  const { pendingQueue, traffic } = useProxyStore();

  const currentEnv = environments.find((e) => e.id === activeEnvironmentId) || environments[0];
  const pendingCount = pendingQueue.length;

  return (
    <footer className="h-6 bg-header border-t border-border flex items-center justify-between px-3 shrink-0 select-none text-[11px] font-mono text-muted-foreground">
      {/* Left Info */}
      <div className="flex items-center gap-3">
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

        {/* Current Environment Badge */}
        <div className="flex items-center gap-1">
          <span className="text-[10px] text-muted-foreground uppercase">Env:</span>
          <span
            className="px-1.5 py-0.2 rounded text-[10px] font-semibold uppercase border"
            style={{
              backgroundColor: `${currentEnv?.color || '#38bdf8'}15`,
              color: currentEnv?.color || '#38bdf8',
              borderColor: `${currentEnv?.color || '#38bdf8'}30`,
            }}
          >
            {currentEnv?.name}
          </span>
        </div>
      </div>
    </footer>
  );
};
