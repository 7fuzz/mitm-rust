import React, { useEffect } from 'react';
import { useSettingsStore, quickVarScopeFor } from '../../stores/useSettingsStore';
import { TopNav } from './TopNav';
import { WorkspaceBar } from './WorkspaceBar';
import { QuickVariableModal } from '../common/QuickVariableModal';

import { HistoryView } from '../modules/history/HistoryView';
import { InterceptView } from '../modules/intercept/InterceptView';
import { RewriteView } from '../modules/rewrite/RewriteView';
import { RepeaterView } from '../modules/repeater/RepeaterView';
import { FuzzerView } from '../modules/fuzzer/FuzzerView';
import { CollectionsView } from '../modules/collections/CollectionsView';
import { WebSocketsView } from '../modules/websockets/WebSocketsView';
import { WebhooksView } from '../modules/webhooks/WebhooksView';
import { WorkspaceView } from '../modules/workspace/WorkspaceView';
import { UtilitiesView } from '../modules/utilities/UtilitiesView';
import { SettingsView } from '../modules/settings/SettingsView';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { useInterceptStore } from '../../stores/useInterceptStore';
import { MODULE_BY_SHORTCUT } from './navModules';
import { useUiPref } from '../../stores/useUiPrefsStore';
import { applyUiZoom, stepZoom } from '../../utils/uiZoom';

export const GlobalShell: React.FC = () => {
  const { activeModule, setActiveModule, theme } = useSettingsStore();
  const [zoom, setZoom] = useUiPref('appearance.zoom');

  const { isQuickVarModalOpen, setQuickVarModalOpen, openQuickVarModal } = useSettingsStore();
  const quickVarScope = quickVarScopeFor(activeModule);
  const initWorkspaceStore = useWorkspaceStore((state) => state.initStore);
  const initInterceptStore = useInterceptStore((state) => state.initInterceptStore);

  // Initialize workspace, active environment, and intercept listener on app startup
  useEffect(() => {
    initWorkspaceStore();
    initInterceptStore();
  }, [initWorkspaceStore, initInterceptStore]);

  useEffect(() => {
    setQuickVarModalOpen(false);
  }, [activeModule, setQuickVarModalOpen]);

  // Number keys switch modules (see navModules) and V opens the variable switcher on pages that send requests
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger when inside inputs or textareas
      const target = e.target as HTMLElement;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
        return;
      }

      if (!e.metaKey && !e.ctrlKey && !e.altKey) {
        if ((e.key === 'v' || e.key === 'V') && quickVarScope) {
          e.preventDefault();
          if (isQuickVarModalOpen) setQuickVarModalOpen(false);
          else openQuickVarModal(quickVarScope);
          return;
        }

        const module = MODULE_BY_SHORTCUT[e.key];
        if (module) {
          setActiveModule(module);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [setActiveModule, isQuickVarModalOpen, setQuickVarModalOpen, openQuickVarModal, quickVarScope]);

  // Saved zoom, applied on startup and whenever it changes
  useEffect(() => {
    applyUiZoom(zoom);
  }, [zoom]);

  // Ctrl/Cmd + = / - / 0 zoom the UI in, out, and back to 100% (works inside inputs and editors too)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      if (e.key === '=' || e.key === '+') {
        e.preventDefault();
        setZoom(stepZoom(zoom, 1));
      } else if (e.key === '-' || e.key === '_') {
        e.preventDefault();
        setZoom(stepZoom(zoom, -1));
      } else if (e.key === '0') {
        e.preventDefault();
        setZoom(1);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [zoom, setZoom]);

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
      case 'rewrite':
        return <RewriteView />;
      case 'repeater':
        return <RepeaterView />;
      case 'fuzzer':
        return <FuzzerView />;
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
      className="h-screen w-screen flex flex-col overflow-hidden bg-background text-foreground font-sans text-2xs"
    >
      {/* Top Navigation Bar */}
      <TopNav />

      {/* Main Module View */}
      <main className="flex-1 overflow-hidden relative bg-background">{renderActiveModuleView()}</main>

      {quickVarScope === 'workspace' && <WorkspaceBar />}

      <QuickVariableModal />
    </div>
  );
};
