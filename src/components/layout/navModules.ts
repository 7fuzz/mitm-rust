import type { NavModule } from '../../types';
import type { MingCuteIconName } from '../common/MingCuteIcon';

export interface NavItem {
  id: NavModule;
  label: string;
  icon: MingCuteIconName;
}

/** Tabs that are always open and can't be closed */
export const PINNED_ITEMS: NavItem[] = [
  { id: 'http-history', label: 'History', icon: 'history_line' },
  { id: 'intercept', label: 'Intercept', icon: 'shield_line' },
  { id: 'repeater', label: 'Repeater', icon: 'repeat_line' },
];

/** Tools in the "More" menu, one inner array per divider-separated group */
export const MORE_GROUPS: NavItem[][] = [
  [
    { id: 'collections', label: 'Collections', icon: 'folder_line' },
    { id: 'workspace', label: 'Workspace', icon: 'grid_line' },
  ],
  [
    { id: 'fuzzer', label: 'Fuzzer', icon: 'fast_forward_line' },
    { id: 'rewrite', label: 'Rewrite', icon: 'transfer_line' },
    { id: 'websockets', label: 'WebSockets', icon: 'websocket_line' },
    { id: 'webhooks', label: 'Webhooks', icon: 'link_line' },
  ],
  [{ id: 'utilities', label: 'Utilities', icon: 'tool_line' }],
];

export const SETTINGS_ITEM: NavItem = { id: 'settings', label: 'Settings', icon: 'settings_3_line' };
export const SETTINGS_SHORTCUT = '0';

export const MORE_ITEMS = MORE_GROUPS.flat();

const TAB_ITEMS = [...PINNED_ITEMS, ...MORE_ITEMS];

export const tabItem = (id: NavModule) => TAB_ITEMS.find((item) => item.id === id);

export const isPinned = (id: NavModule) => PINNED_ITEMS.some((item) => item.id === id);

/** Number keys 1–9 jump to the tab at that position */
export const tabShortcut = (index: number) => (index < 9 ? String(index + 1) : undefined);
