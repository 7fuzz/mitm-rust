import { useState, useRef } from 'react';
import { NotificationProvider } from '@/components/ui/NotificationProvider';
import { TrafficProvider, useTraffic } from '@/hooks/traffic';
import { ThemeProvider, useTheme } from '@/hooks/ui/useTheme';
import { InterceptView } from '@/components/View/InterceptView';
import { RepeaterView } from '@/components/View/RepeaterView';
import { HistoryView } from '@/components/View/HistoryView';
import { OptionsView } from '@/components/View/OptionsView';
import { UtilitiesView } from '@/components/View/UtilitiesView';
import { WorkspaceView, WorkspaceViewHandle } from '@/components/View/WorkspaceView';
import { useKeyboardShortcuts } from '@/hooks/ui/useKeyboardShortcuts';
import { ShortcutHint } from '@/components/ui/ShortcutHint';
import { VariableQuickSwitcherModal, EnvironmentQuickSwitcherModal } from '@/components/Modals/QuickSwitcherModals';

function TrafficApp() {
  const { 
    traffic, repeaterRequests, simpleMode, prefs,
    variables, activeEnvId, updateVariable, reorderVariables,
    addVariable, deleteVariable,
    environments, setActiveEnvironment
  } = useTraffic();
  const { theme, toggleTheme } = useTheme();

  const [activeTab, setActiveTab] = useState<'history' | 'intercept' | 'repeater' | 'options' | 'utilities' | 'workspace'>('history');
  const workspaceRef = useRef<WorkspaceViewHandle>(null);

  const [isVarModalOpen, setIsVarModalOpen] = useState(false);
  const [isEnvModalOpen, setIsEnvModalOpen] = useState(false);

  const { isWaiting } = useKeyboardShortcuts({
    activeTab,
    onTabChange: setActiveTab,
    simpleMode,
    shortcuts: prefs.shortcuts,
    onOpenVariableSwitcher: () => {
      setActiveTab('workspace');
      setTimeout(() => workspaceRef.current?.switchTab('env'), 0);
    },
    onOpenEnvironmentSwitcher: () => {
      setActiveTab('workspace');
      setTimeout(() => workspaceRef.current?.switchTab('env'), 0);
    },
    onOpenVariableSwitcherInstant: () => setIsVarModalOpen(prev => !prev),
    onOpenEnvironmentSwitcherInstant: () => setIsEnvModalOpen(prev => !prev),
    isModalOpen: isVarModalOpen || isEnvModalOpen
  });

  const pendingCount = traffic.filter(t => t.is_intercepted).length;

  const handleQuickEdit = () => {
    setIsVarModalOpen(false);
    setIsEnvModalOpen(false);
    setActiveTab('workspace');
    setTimeout(() => workspaceRef.current?.switchTab('env'), 0);
  };

  return (
    <div className="flex flex-col h-screen bg-zinc-950 text-zinc-300 font-mono text-sm overflow-hidden selection:bg-emerald-500/30">

      {/* Top Navigation Bar */}
      <header className="flex items-center justify-between px-4 h-12 border-b border-zinc-800 bg-zinc-950 shrink-0">
        <div className="flex gap-1 h-full">
          <button
            onClick={() => setActiveTab('history')}
            className={`px-6 h-full flex items-center text-[11px] font-bold uppercase tracking-widest border-b-2 transition-all ${activeTab === 'history' ? 'border-emerald-500 text-emerald-400 bg-zinc-900/50' : 'border-transparent text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'}`}
          >
            HTTP_History
          </button>

          <button
            onClick={() => setActiveTab('intercept')}
            className={`px-6 h-full flex items-center gap-3 text-[11px] font-bold uppercase tracking-widest border-b-2 transition-all ${activeTab === 'intercept' ? 'border-rose-500 text-rose-400 bg-zinc-900/50' : 'border-transparent text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'}`}
          >
            Intercept
            {pendingCount > 0 && (
              <span className="flex items-center justify-center min-w-5 h-5 bg-rose-500/20 text-rose-400 border border-rose-500/30 rounded-full text-[9px] animate-pulse px-1 font-black">
                {pendingCount}
              </span>
            )}
          </button>

          <button
            onClick={() => setActiveTab('repeater')}
            className={`px-6 h-full flex items-center gap-3 text-[11px] font-bold uppercase tracking-widest border-b-2 transition-all ${activeTab === 'repeater' ? 'border-purple-500 text-purple-400 bg-zinc-900/50' : 'border-transparent text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'}`}
          >
            Repeater
            {repeaterRequests.length > 0 && (
              <span className="flex items-center justify-center min-w-5 h-5 bg-purple-500/20 text-purple-400 border border-purple-500/30 rounded-full text-[9px] px-1 font-black">
                {repeaterRequests.length}
              </span>
            )}
          </button>

          {!simpleMode && (
            <button
              onClick={() => setActiveTab('workspace')}
              className={`px-6 h-full flex items-center gap-2 text-[11px] font-bold uppercase tracking-widest border-b-2 transition-all ${activeTab === 'workspace' ? 'border-blue-500 text-blue-400 bg-zinc-900/50' : 'border-transparent text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'}`}
            >
              Workspace
            </button>
          )}

          {!simpleMode && (
            <button
              onClick={() => setActiveTab('utilities')}
              className={`px-6 h-full flex items-center gap-2 text-[11px] font-bold uppercase tracking-widest border-b-2 transition-all ${activeTab === 'utilities' ? 'border-fuchsia-500 text-fuchsia-400 bg-zinc-900/50' : 'border-transparent text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'}`}
            >
              Utilities
            </button>
          )}

          <button
            onClick={() => setActiveTab('options')}
            className={`px-6 h-full flex items-center gap-2 text-[11px] font-bold uppercase tracking-widest border-b-2 transition-all ${activeTab === 'options' ? 'border-orange-500 text-orange-400 bg-zinc-900/50' : 'border-transparent text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'}`}
          >
            Options
          </button>
        </div>

        <div className="flex items-center gap-4">
          <button
            onClick={toggleTheme}
            className="p-2 rounded-full border border-zinc-800 bg-zinc-900 text-zinc-400 hover:text-zinc-200 hover:border-zinc-700 transition-all cursor-pointer"
            title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
          >
            {theme === 'dark' ? (
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="18.36" x2="5.64" y2="16.92"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
            ) : (
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>
            )}
          </button>

          <div className="flex items-center gap-3 px-3 py-1.5 bg-emerald-500/5 border border-emerald-500/20 rounded-full group hover:border-emerald-500/40 transition-all cursor-default">
            <div className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
            </div>
            <span className="text-[10px] text-emerald-500/80 font-black uppercase tracking-[0.15em]">Proxy_Live</span>
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 flex overflow-hidden relative">
        {activeTab === 'history' && <HistoryView />}
        {activeTab === 'intercept' && <InterceptView />}
        {activeTab === 'repeater' && <RepeaterView />}
        {!simpleMode && activeTab === 'workspace' && <WorkspaceView ref={workspaceRef} />}
        {!simpleMode && activeTab === 'utilities' && <UtilitiesView />}
        {activeTab === 'options' && <OptionsView />}
      </main>

      <ShortcutHint isOpen={isWaiting} simpleMode={simpleMode} />

      <VariableQuickSwitcherModal 
        isOpen={isVarModalOpen} 
        onClose={() => setIsVarModalOpen(false)} 
        variables={variables}
        activeEnvId={activeEnvId}
        onUpdateVariable={updateVariable}
        onReorder={reorderVariables}
        onAddVariable={addVariable}
        onDeleteVariable={deleteVariable}
        onEdit={handleQuickEdit}
      />
      
      <EnvironmentQuickSwitcherModal
        isOpen={isEnvModalOpen}
        onClose={() => setIsEnvModalOpen(false)}
        environments={environments}
        activeEnvId={activeEnvId}
        onSetActive={setActiveEnvironment}
        onEdit={handleQuickEdit}
      />
    </div>
  );
}

export default function App() {
  return (
    <NotificationProvider>
      <TrafficProvider>
        <ThemeProvider>
          <TrafficApp />
        </ThemeProvider>
      </TrafficProvider>
    </NotificationProvider>
  );
}
