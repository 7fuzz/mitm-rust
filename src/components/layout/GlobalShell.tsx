import React, { useEffect } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { TopNav } from './TopNav';
import { StatusBar } from './StatusBar';
import { QuickVariableModal } from '../common/QuickVariableModal';

import { HistoryView } from '../modules/history/HistoryView';
import { InterceptView } from '../modules/intercept/InterceptView';
import { RepeaterView } from '../modules/repeater/RepeaterView';
import { CollectionsView } from '../modules/collections/CollectionsView';
import { WebSocketsView } from '../modules/websockets/WebSocketsView';
import { WebhooksView } from '../modules/webhooks/WebhooksView';
import { WorkspaceView } from '../modules/workspace/WorkspaceView';
import { UtilitiesView } from '../modules/utilities/UtilitiesView';
import { SettingsView } from '../modules/settings/SettingsView';
import type { NavModule } from '../../types';

export const GlobalShell: React.FC = () => {
  const { activeModule, setActiveModule, theme, fontSize } = useSettingsStore();

  // Keyboard shortcut listener for tabs 1-9
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger when inside inputs or textareas
      const target = e.target as HTMLElement;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
        return;
      }

      if (!e.metaKey && !e.ctrlKey && !e.altKey) {
        const keyMap: Record<string, NavModule> = {
          '1': 'http-history',
          '2': 'intercept',
          '3': 'repeater',
          '4': 'collections',
          '5': 'websockets',
          '6': 'webhooks',
          '7': 'workspace',
          '8': 'utilities',
          '9': 'settings',
        };
        if (keyMap[e.key]) {
          setActiveModule(keyMap[e.key]);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [setActiveModule]);

  // Ensure theme class applied on root HTML
  useEffect(() => {
    if (theme === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme]);

  const renderActiveModuleView = () => {
    switch (activeModule) {
      case 'http-history':
        return <HistoryView />;
      case 'intercept':
        return <InterceptView />;
      case 'repeater':
        return <RepeaterView />;
      case 'collections':
        return <CollectionsView />;
      case 'websockets':
        return <WebSocketsView />;
      case 'webhooks':
        return <WebhooksView />;
      case 'workspace':
        return <WorkspaceView />;
      case 'utilities':
        return <UtilitiesView />;
      case 'settings':
        return <SettingsView />;
      default:
        return <HistoryView />;
    }
  };

  return (
    <div
      className={`h-screen w-screen flex flex-col overflow-hidden bg-background text-foreground select-none font-sans ${
        fontSize === 'lg' ? 'text-sm' : fontSize === 'md' ? 'text-xs' : 'text-[11px]'
      }`}
    >
      {/* Top Navigation Bar */}
      <TopNav />

      {/* Main Module View */}
      <main className="flex-1 overflow-hidden relative bg-background">{renderActiveModuleView()}</main>

      {/* Bottom Status Bar */}
      <StatusBar />

      {/* Quick Variable Switcher Modal (Cmd+K) */}
      <QuickVariableModal />
    </div>
  );
};
