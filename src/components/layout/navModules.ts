import type { NavModule } from '../../types';
import type { MingCuteIconName } from '../common/MingCuteIcon';

export interface NavItem {
  id: NavModule;
  label: string;
  /** Number key that opens it; there are more modules than digits, so some have none */
  shortcut?: string;
  icon: MingCuteIconName;
}

/** Always-visible tabs, one inner array per divider-separated group */
export const PRIMARY_GROUPS: NavItem[][] = [
  [
    { id: 'http-history', label: 'History', shortcut: '1', icon: 'history_line' },
    { id: 'intercept', label: 'Intercept', shortcut: '2', icon: 'shield_line' },
    { id: 'repeater', label: 'Repeater', shortcut: '3', icon: 'repeat_line' },
  ],
  [
    { id: 'collections', label: 'Collections', shortcut: '4', icon: 'folder_line' },
    { id: 'workspace', label: 'Workspace', shortcut: '5', icon: 'grid_line' },
  ],
];

/** Tools in the "More" menu, one inner array per divider-separated group */
export const MORE_GROUPS: NavItem[][] = [
  [
    { id: 'fuzzer', label: 'Fuzzer', shortcut: '6', icon: 'fast_forward_line' },
    { id: 'rewrite', label: 'Rewrite', shortcut: '7', icon: 'transfer_line' },
    { id: 'websockets', label: 'WebSockets', shortcut: '8', icon: 'websocket_line' },
    { id: 'webhooks', label: 'Webhooks', shortcut: '9', icon: 'link_line' },
  ],
  [{ id: 'utilities', label: 'Utilities', icon: 'tool_line' }],
];

export const SETTINGS_ITEM: NavItem = { id: 'settings', label: 'Settings', shortcut: '0', icon: 'settings_3_line' };

const ALL_ITEMS = [...PRIMARY_GROUPS.flat(), ...MORE_GROUPS.flat(), SETTINGS_ITEM];

export const MORE_ITEMS = MORE_GROUPS.flat();

export const MODULE_BY_SHORTCUT: Record<string, NavModule> = Object.fromEntries(
  ALL_ITEMS.filter((item) => item.shortcut).map((item) => [item.shortcut as string, item.id])
);
