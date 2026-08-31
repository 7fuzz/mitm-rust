import React, { useEffect, useRef } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useHistoryStore } from '../../stores/useHistoryStore';
import { useInterceptStore } from '../../stores/useInterceptStore';
import { MingCuteIcon, MingCuteIconName } from '../common/MingCuteIcon';
import { ProxyPowerButton } from '../common/ProxyPowerButton';
import type { NavModule } from '../../types';

interface NavTabItem {
  id: NavModule;
  label: string;
  shortcut: string;
  icon: MingCuteIconName;
}

const NAV_TABS: NavTabItem[] = [
  { id: 'http-history', label: 'HTTP History', shortcut: '1', icon: 'history_line' },
  { id: 'intercept', label: 'Intercept', shortcut: '2', icon: 'shield_line' },
  { id: 'rewrite', label: 'Rewrite', shortcut: '3', icon: 'transfer_line' },
  { id: 'repeater', label: 'Repeater', shortcut: '4', icon: 'repeat_line' },
  { id: 'collections', label: 'Collections', shortcut: '5', icon: 'folder_line' },
  { id: 'websockets', label: 'WebSockets', shortcut: '6', icon: 'websocket_line' },
  { id: 'webhooks', label: 'Webhooks', shortcut: '7', icon: 'link_line' },
  { id: 'workspace', label: 'Workspace', shortcut: '8', icon: 'grid_line' },
  { id: 'utilities', label: 'Utilities', shortcut: '9', icon: 'tool_line' },
  { id: 'settings', label: 'Settings', shortcut: '0', icon: 'settings_3_line' },
];

export const TopNav: React.FC = () => {
  const { activeModule, setActiveModule, theme, toggleTheme } = useSettingsStore();
  const { fetchProxyStatus } = useHistoryStore();
  const { pendingFlows } = useInterceptStore();
  const navRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchProxyStatus();
  }, []);

  const handleWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (navRef.current) {
      navRef.current.scrollLeft += e.deltaY || e.deltaX;
    }
  };

  return (
    <header className="h-10 bg-header border-b border-border flex items-center justify-between px-2 shrink-0 select-none text-xs relative z-40">

      {/* Left: Navigation Tabs */}
      <div
        ref={navRef}
        onWheel={handleWheel}
        className="flex items-center gap-1 flex-1 overflow-x-auto no-scrollbar py-0.5"
      >
        {NAV_TABS.map((tab) => {
          const isActive = activeModule === tab.id;
          const isIntercept = tab.id === 'intercept';
          const pendingCount = pendingFlows.length;

          return (
            <button
              key={tab.id}
              onClick={() => setActiveModule(tab.id)}
              className={`relative flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium text-[11px] whitespace-nowrap shrink-0 ${
                isActive
                  ? 'bg-surface text-primary shadow-xs border border-border/80 font-semibold'
                  : 'text-muted-foreground hover:text-foreground hover:bg-surface/50'
              }`}
            >
              <MingCuteIcon name={tab.icon} size={14} />
              <span>{tab.label}</span>
              <span className="text-[9px] opacity-60 font-mono">[{tab.shortcut}]</span>

              {isIntercept && pendingCount > 0 && (
                <span className="ml-1 px-1.5 py-0.2 bg-amber-500 text-zinc-950 rounded-full text-[9px] font-mono font-bold animate-pulse">
                  {pendingCount}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Right Controls: Master Power Switch & Theme Toggle */}
      <div className="flex items-center gap-2 shrink-0 pl-2">
        {/* Proxy Power Button with Mode Selection Popover Overlay */}
        <ProxyPowerButton />

        {/* Theme Switch */}
        <button
          onClick={toggleTheme}
          className="p-1.5 rounded bg-surface border border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle transition-colors"
          title={`Switch to ${theme === 'dark' ? 'Light' : 'Dark'} mode`}
        >
          <MingCuteIcon name={theme === 'dark' ? 'sun_line' : 'moon_line'} size={15} />
        </button>
      </div>
    </header>
  );
};
