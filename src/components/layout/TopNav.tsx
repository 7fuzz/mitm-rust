import React, { useEffect, useRef, useState } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useHistoryStore } from '../../stores/useHistoryStore';
import { useInterceptStore } from '../../stores/useInterceptStore';
import { useRewriteStore } from '../../stores/useRewriteStore';
import { useWebSocketStore } from '../../stores/useWebSocketStore';
import { useWebhookStore } from '../../stores/useWebhookStore';
import { MingCuteIcon } from '../common/MingCuteIcon';
import { ProxyPowerButton } from '../common/ProxyPowerButton';
import type { NavModule } from '../../types';
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core';
import { SortableContext, horizontalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { MORE_GROUPS, SETTINGS_ITEM, SETTINGS_SHORTCUT, isPinned, tabItem, tabShortcut, type NavItem } from './navModules';

const ActiveDot: React.FC = () => <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" title="Active" />;

const NavTab: React.FC<{
  item: NavItem;
  shortcut?: string;
  isActive: boolean;
  isRunning: boolean;
  pendingCount: number;
  onSelect: () => void;
  onClose?: () => void;
}> = ({ item, shortcut, isActive, isRunning, pendingCount, onSelect, onClose }) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: item.id });

  return (
    <button
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      style={{
        transform: transform ? CSS.Translate.toString({ ...transform, y: 0 }) : undefined,
        transition,
      }}
      onClick={onSelect}
      onMouseDown={(e) => {
        // Stops the middle-click autoscroll cursor
        if (e.button === 1) e.preventDefault();
      }}
      onAuxClick={(e) => {
        if (e.button === 1) onClose?.();
      }}
      className={`relative flex items-center gap-1.5 px-2.5 py-1 rounded-md text-2xs whitespace-nowrap shrink-0 cursor-pointer transition-colors outline-none ${
        isActive
          ? 'bg-surface text-primary shadow-xs border border-border/80 font-semibold'
          : 'border border-transparent text-muted-foreground font-medium hover:text-foreground hover:bg-surface/50'
      } ${isDragging ? 'z-10' : ''}`}
      title={shortcut ? `${item.label} (${shortcut})` : item.label}
    >
      <MingCuteIcon name={item.icon} size={14} />
      <span>{item.label}</span>
      {isRunning && <ActiveDot />}
      {shortcut && <span className="text-3xs opacity-50 font-mono">{shortcut}</span>}
      {pendingCount > 0 && (
        <span className="ml-0.5 px-1.5 bg-amber-500 text-zinc-950 rounded-full text-3xs font-mono font-bold animate-pulse">{pendingCount}</span>
      )}
      {onClose && (
        <span
          role="button"
          onPointerDown={(e) => e.stopPropagation()}
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

export const TopNav: React.FC = () => {
  const { activeModule, setActiveModule, openTabs, closeTab, moveTab, theme, toggleTheme } = useSettingsStore();
  const { fetchProxyStatus } = useHistoryStore();
  const { pendingFlows } = useInterceptStore();
  const rewriteActive = useRewriteStore((s) => s.isRewriteEnabled && s.rules.some((r) => r.enabled));
  const wsActive = useWebSocketStore((s) => s.wsMitmEnabled);
  const webhooksActive = useWebhookStore((s) => s.listenerConfig.is_running);
  const isModuleActive = (id: NavModule) => (id === 'rewrite' && rewriteActive) || (id === 'websockets' && wsActive) || (id === 'webhooks' && webhooksActive);

  const [moreMenuPos, setMoreMenuPos] = useState<{
    left: number;
    top: number;
  } | null>(null);
  const isMoreOpen = moreMenuPos !== null;
  const moreRef = useRef<HTMLDivElement>(null);
  const moreButtonRef = useRef<HTMLButtonElement>(null);
  const closeMore = () => setMoreMenuPos(null);
  const dndSensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  useEffect(() => {
    fetchProxyStatus();
    useRewriteStore.getState().fetchStatus();
    useWebSocketStore.getState().fetchStatus();
    useWebhookStore.getState().fetchStatus();
  }, []);

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

  const handleTabDragEnd = ({ active, over }: DragEndEvent) => {
    if (over && active.id !== over.id) moveTab(active.id as NavModule, over.id as NavModule);
  };

  const closedMoreGroups = MORE_GROUPS.map((group) => group.filter((item) => !openTabs.includes(item.id))).filter((group) => group.length > 0);

  const divider = (key: string) => <div key={key} className="h-4 w-px bg-border mx-1 shrink-0" />;

  return (
    <header className="h-10 bg-header border-b border-border flex items-center gap-1 px-2 shrink-0 select-none text-xs relative z-40">
      <nav className="flex items-center gap-1 flex-1 min-w-0 overflow-x-auto no-scrollbar py-0.5">
        <DndContext sensors={dndSensors} collisionDetection={closestCenter} onDragEnd={handleTabDragEnd}>
          <SortableContext items={openTabs} strategy={horizontalListSortingStrategy}>
            {openTabs.map((id, index) => {
              const item = tabItem(id);
              if (!item) return null;
              return (
                <NavTab
                  key={id}
                  item={item}
                  shortcut={tabShortcut(index)}
                  isActive={activeModule === id}
                  isRunning={isModuleActive(id)}
                  pendingCount={id === 'intercept' ? pendingFlows.length : 0}
                  onSelect={() => setActiveModule(id)}
                  onClose={isPinned(id) ? undefined : () => closeTab(id)}
                />
              );
            })}
          </SortableContext>
        </DndContext>
        {closedMoreGroups.length > 0 && divider('more')}

        {closedMoreGroups.length > 0 && (
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
            {closedMoreGroups.flat().some((item) => isModuleActive(item.id)) && <ActiveDot />}
            <MingCuteIcon name="down_line" size={12} className="opacity-60" />
          </button>
        )}

        {moreMenuPos && closedMoreGroups.length > 0 && (
          <div
            ref={moreRef}
            style={{ left: moreMenuPos.left, top: moreMenuPos.top }}
            className="fixed w-52 bg-surface border border-border rounded-lg shadow-xl py-1 z-50"
          >
            {closedMoreGroups.map((group, i) => (
              <div key={i} className={i > 0 ? 'border-t border-border mt-1 pt-1' : ''}>
                {group.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => {
                      setActiveModule(item.id);
                      closeMore();
                    }}
                    className="w-full flex items-center gap-2 px-3 py-1.5 text-left text-xs cursor-pointer text-foreground hover:bg-neutral-subtle"
                  >
                    <MingCuteIcon name={item.icon} size={14} />
                    <span className="flex-1 flex items-center gap-1.5">
                      {item.label}
                      {isModuleActive(item.id) && <ActiveDot />}
                    </span>
                  </button>
                ))}
              </div>
            ))}
          </div>
        )}
      </nav>

      <div className="flex items-center gap-2 shrink-0 pl-2">
        <button
          onClick={() => setActiveModule(SETTINGS_ITEM.id)}
          className={`p-1.5 rounded border transition-colors cursor-pointer ${
            activeModule === SETTINGS_ITEM.id
              ? 'bg-surface border-border/80 text-primary'
              : 'bg-surface border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
          title={`Settings (${SETTINGS_SHORTCUT})`}
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
