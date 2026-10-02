import React, { useEffect, useRef, useState } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useHistoryStore } from '../../stores/useHistoryStore';
import { useInterceptStore } from '../../stores/useInterceptStore';
import { MingCuteIcon } from '../common/MingCuteIcon';
import { ProxyPowerButton } from '../common/ProxyPowerButton';
import type { NavModule } from '../../types';
import { MORE_GROUPS, MORE_ITEMS, PRIMARY_GROUPS, SETTINGS_ITEM, type NavItem } from './navModules';

const isPrimary = (id: NavModule) => PRIMARY_GROUPS.some((group) => group.some((item) => item.id === id));

export const TopNav: React.FC = () => {
  const { activeModule, setActiveModule, theme, toggleTheme } = useSettingsStore();
  const { fetchProxyStatus } = useHistoryStore();
  const { pendingFlows } = useInterceptStore();

  const [moreMenuPos, setMoreMenuPos] = useState<{ left: number; top: number } | null>(null);
  const isMoreOpen = moreMenuPos !== null;
  const moreRef = useRef<HTMLDivElement>(null);
  const moreButtonRef = useRef<HTMLButtonElement>(null);
  const closeMore = () => setMoreMenuPos(null);
  const lastPrimary = useRef<NavModule>('http-history');

  useEffect(() => {
    fetchProxyStatus();
  }, []);

  useEffect(() => {
    if (isPrimary(activeModule)) lastPrimary.current = activeModule;
  }, [activeModule]);

  useEffect(() => {
    if (!isMoreOpen) return;
    const handleMouseDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (!moreRef.current?.contains(target) && !moreButtonRef.current?.contains(target)) closeMore();
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeMore();
    };
    document.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isMoreOpen]);

  const openedMoreItem = MORE_ITEMS.find((item) => item.id === activeModule);

  const tab = (item: NavItem, onClose?: () => void) => {
    const isActive = activeModule === item.id;
    const pendingCount = item.id === 'intercept' ? pendingFlows.length : 0;
    return (
      <button
        key={item.id}
        onClick={() => setActiveModule(item.id)}
        className={`relative flex items-center gap-1.5 px-2.5 py-1 rounded-md text-2xs whitespace-nowrap shrink-0 cursor-pointer transition-colors ${
          isActive
            ? 'bg-surface text-primary shadow-xs border border-border/80 font-semibold'
            : 'border border-transparent text-muted-foreground font-medium hover:text-foreground hover:bg-surface/50'
        }`}
        title={item.shortcut ? `${item.label} (${item.shortcut})` : item.label}
      >
        <MingCuteIcon name={item.icon} size={14} />
        <span>{item.label}</span>
        {item.shortcut && <span className="text-3xs opacity-50 font-mono">{item.shortcut}</span>}
        {pendingCount > 0 && (
          <span className="ml-0.5 px-1.5 bg-amber-500 text-zinc-950 rounded-full text-3xs font-mono font-bold animate-pulse">
            {pendingCount}
          </span>
        )}
        {onClose && (
          <span
            role="button"
            onClick={(e) => {
              e.stopPropagation();
              onClose();
            }}
            className="ml-0.5 -mr-1 p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
            title="Close"
          >
            <MingCuteIcon name="close_line" size={11} />
          </span>
        )}
      </button>
    );
  };

  const divider = (key: string) => <div key={key} className="h-4 w-px bg-border mx-1 shrink-0" />;

  return (
    <header className="h-10 bg-header border-b border-border flex items-center gap-1 px-2 shrink-0 select-none text-xs relative z-40">
      <nav className="flex items-center gap-1 flex-1 min-w-0 overflow-x-auto no-scrollbar py-0.5">
        {PRIMARY_GROUPS.flatMap((group, i) => [...(i > 0 ? [divider(`d${i}`)] : []), ...group.map((item) => tab(item))])}
        {divider('more')}

        <button
          ref={moreButtonRef}
          onClick={() => {
            if (isMoreOpen) return closeMore();
            const rect = moreButtonRef.current?.getBoundingClientRect();
            if (rect) setMoreMenuPos({ left: rect.left, top: rect.bottom + 4 });
          }}
          className={`flex items-center gap-1 px-2.5 py-1 rounded-md text-2xs font-medium cursor-pointer transition-colors border shrink-0 ${
            isMoreOpen ? 'bg-surface border-border/80 text-foreground' : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-surface/50'
          }`}
        >
          <span>More</span>
          <MingCuteIcon name="down_line" size={12} className="opacity-60" />
        </button>

        {moreMenuPos && (
          <div
            ref={moreRef}
            style={{ left: moreMenuPos.left, top: moreMenuPos.top }}
            className="fixed w-52 bg-surface border border-border rounded-lg shadow-xl py-1 z-50"
          >
            {MORE_GROUPS.map((group, i) => (
              <div key={i} className={i > 0 ? 'border-t border-border mt-1 pt-1' : ''}>
                {group.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => {
                      setActiveModule(item.id);
                      closeMore();
                    }}
                    className={`w-full flex items-center gap-2 px-3 py-1.5 text-left text-xs cursor-pointer ${
                      activeModule === item.id ? 'text-primary font-semibold' : 'text-foreground hover:bg-neutral-subtle'
                    }`}
                  >
                    <MingCuteIcon name={item.icon} size={14} />
                    <span className="flex-1">{item.label}</span>
                    {item.shortcut && <span className="text-3xs font-mono text-muted-foreground">{item.shortcut}</span>}
                  </button>
                ))}
              </div>
            ))}
          </div>
        )}

        {openedMoreItem && tab(openedMoreItem, () => setActiveModule(lastPrimary.current))}
      </nav>

      <div className="flex items-center gap-2 shrink-0 pl-2">
        <button
          onClick={() => setActiveModule(SETTINGS_ITEM.id)}
          className={`p-1.5 rounded border transition-colors cursor-pointer ${
            activeModule === SETTINGS_ITEM.id
              ? 'bg-surface border-border/80 text-primary'
              : 'bg-surface border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
          title={`Settings (${SETTINGS_ITEM.shortcut})`}
        >
          <MingCuteIcon name={SETTINGS_ITEM.icon} size={15} />
        </button>

        <ProxyPowerButton />

        <button
          onClick={toggleTheme}
          className="p-1.5 rounded bg-surface border border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle transition-colors cursor-pointer"
          title={`Switch to ${theme === 'dark' ? 'Light' : 'Dark'} mode`}
        >
          <MingCuteIcon name={theme === 'dark' ? 'sun_line' : 'moon_line'} size={15} />
        </button>
      </div>
    </header>
  );
};
