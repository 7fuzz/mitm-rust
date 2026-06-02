import { useRef, useState } from 'react';
import { KeyboardShortcuts, TabType } from '../traffic/types';
import { useHotkeys } from './useHotkeys';

interface ShortcutOptions {
  activeTab: TabType;
  onTabChange: (tab: TabType) => void;
  simpleMode: boolean;
  shortcuts?: KeyboardShortcuts;
  onOpenVariableSwitcher?: () => void;
  onOpenEnvironmentSwitcher?: () => void;
  onOpenVariableSwitcherInstant?: () => void;
  onOpenEnvironmentSwitcherInstant?: () => void;
  isModalOpen?: boolean;
}

export function useKeyboardShortcuts({ 
  activeTab, 
  onTabChange, 
  simpleMode, 
  shortcuts,
  onOpenVariableSwitcher,
  onOpenEnvironmentSwitcher,
  onOpenVariableSwitcherInstant,
  onOpenEnvironmentSwitcherInstant,
  isModalOpen
}: ShortcutOptions) {
  const waitingForSecondKey = useRef<boolean>(false);
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const [isWaiting, setIsWaiting] = useState(false);

  const parseKey = (str: string) => {
    if (!str) return { key: '', ctrl: false, shift: false, alt: false, meta: false };
    const parts = str.toLowerCase().split('+');
    return {
      key: parts[parts.length - 1],
      ctrl: parts.includes('ctrl'),
      shift: parts.includes('shift'),
      alt: parts.includes('alt'),
      meta: parts.includes('meta')
    };
  };

  const prefix = parseKey(shortcuts?.prefix_key || '');

  const resetWaiting = () => {
    waitingForSecondKey.current = false;
    setIsWaiting(false);
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  useHotkeys([
    // 1. Modal Toggle Keys (Always enabled even if modal is open)
    {
      key: [shortcuts?.instant_variable_switcher || '', shortcuts?.instant_environment_switcher || ''],
      enabled: !!shortcuts && isModalOpen,
      handler: (e) => {
        const key = e.key.toLowerCase();
        if (key === shortcuts?.instant_variable_switcher) onOpenVariableSwitcherInstant?.();
        if (key === shortcuts?.instant_environment_switcher) onOpenEnvironmentSwitcherInstant?.();
      }
    },
    // 2. Global Instant Shortcuts (only if modal is closed)
    {
      key: shortcuts?.instant_variable_switcher || '',
      enabled: !!shortcuts && !isModalOpen && !waitingForSecondKey.current,
      handler: () => onOpenVariableSwitcherInstant?.(),
    },
    {
      key: shortcuts?.instant_environment_switcher || '',
      enabled: !!shortcuts && !isModalOpen && !waitingForSecondKey.current,
      handler: () => onOpenEnvironmentSwitcherInstant?.(),
    },
    // 3. Tab Cycling
    {
      key: [shortcuts?.cycle_prev || '', shortcuts?.cycle_next || ''],
      enabled: !!shortcuts && !isModalOpen && !waitingForSecondKey.current,
      handler: (e) => {
        const tabs: TabType[] = ['history', 'intercept', 'repeater'];
        if (!simpleMode) tabs.push('workspace', 'utilities');
        tabs.push('options');

        const currentIndex = tabs.indexOf(activeTab);
        if (currentIndex === -1) return;

        const nextIndex = e.key.toLowerCase() === shortcuts?.cycle_prev 
          ? (currentIndex - 1 + tabs.length) % tabs.length
          : (currentIndex + 1) % tabs.length;
        
        onTabChange(tabs[nextIndex]);
      }
    },
    // 4. View-specific Action Shortcuts
    {
      key: shortcuts?.clear_history || 'd',
      ctrl: true,
      enabled: !!shortcuts && !isModalOpen && activeTab === 'history',
      handler: () => window.dispatchEvent(new CustomEvent('shortcut-clear-history')),
    },
    {
      key: shortcuts?.execute_request || 'Enter',
      ctrl: true,
      enabled: !!shortcuts && !isModalOpen && activeTab === 'repeater',
      handler: () => window.dispatchEvent(new CustomEvent('shortcut-execute-request')),
    },
    {
      key: shortcuts?.forward_intercept || 'f',
      ctrl: true,
      enabled: !!shortcuts && !isModalOpen && activeTab === 'intercept',
      handler: () => window.dispatchEvent(new CustomEvent('shortcut-forward-intercept')),
    },
    {
      key: shortcuts?.drop_intercept || 'd',
      ctrl: true,
      enabled: !!shortcuts && !isModalOpen && activeTab === 'intercept',
      handler: () => window.dispatchEvent(new CustomEvent('shortcut-drop-intercept')),
    },
    {
      key: shortcuts?.focus_search || 's',
      enabled: !!shortcuts && !isModalOpen && (activeTab === 'history' || activeTab === 'repeater' || activeTab === 'intercept'),
      handler: () => window.dispatchEvent(new CustomEvent('shortcut-focus-search')),
    },
    // 5. Prefix Key
    {
      key: prefix.key,
      ctrl: prefix.ctrl,
      shift: prefix.shift,
      alt: prefix.alt,
      meta: prefix.meta,
      enabled: !!shortcuts && !isModalOpen && !waitingForSecondKey.current,
      handler: () => {
        waitingForSecondKey.current = true;
        setIsWaiting(true);
        timerRef.current = setTimeout(resetWaiting, 1500);
      }
    }
  ], [activeTab, shortcuts, isModalOpen, simpleMode, prefix]);

  // Handle sequences (the second key)
  useHotkeys([
    {
      key: [
        shortcuts?.goto_history || '',
        shortcuts?.goto_intercept || '',
        shortcuts?.goto_repeater || '',
        shortcuts?.goto_options || '',
        shortcuts?.goto_workspace || '',
        shortcuts?.goto_utilities || '',
        shortcuts?.open_variable_switcher || '',
        shortcuts?.open_environment_switcher || ''
      ],
      enabled: !!shortcuts && waitingForSecondKey.current,
      handler: (e) => {
        const key = e.key.toLowerCase();
        let targetTab: TabType | null = null;

        if (key === shortcuts?.goto_history) targetTab = 'history';
        else if (key === shortcuts?.goto_intercept) targetTab = 'intercept';
        else if (key === shortcuts?.goto_repeater) targetTab = 'repeater';
        else if (key === shortcuts?.goto_options) targetTab = 'options';
        else if (key === shortcuts?.goto_workspace && !simpleMode) targetTab = 'workspace';
        else if (key === shortcuts?.goto_utilities && !simpleMode) targetTab = 'utilities';
        else if (key === shortcuts?.open_variable_switcher) onOpenVariableSwitcher?.();
        else if (key === shortcuts?.open_environment_switcher) onOpenEnvironmentSwitcher?.();

        if (targetTab) onTabChange(targetTab);
        resetWaiting();
      }
    }
  ], [shortcuts, waitingForSecondKey.current]);

  return { isWaiting };
}
