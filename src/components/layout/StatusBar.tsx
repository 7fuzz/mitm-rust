import React, { useState, useEffect, useRef } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { useProxyStore } from '../../stores/useProxyStore';
import { MingCuteIcon } from '../common/MingCuteIcon';

export const StatusBar: React.FC = () => {
  const { dbSizeMb, isSqliteConnected, setQuickVarModalOpen, setActiveModule } = useSettingsStore();
  const {
    activeWorkspaceId,
    workspaces,
    activeEnvironmentId,
    environmentsList,
    environments,
    selectWorkspace,
    createNewWorkspace,
    openImportModalForWorkspace,
  } = useWorkspaceStore();
  const { pendingQueue, traffic } = useProxyStore();

  const [isWorkspaceMenuOpen, setIsWorkspaceMenuOpen] = useState(false);
  const [workspaceSearch, setWorkspaceSearch] = useState('');
  const [isCreatingInline, setIsCreatingInline] = useState(false);
  const [newWsName, setNewWsName] = useState('');

  const popoverRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const activeEnvFromList = environmentsList.find((e) => e.id === activeEnvironmentId || e.isActive);
  const activeEnvFromLegacy = environments.find((e) => e.id === activeEnvironmentId);

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId);
  const workspaceName = activeWorkspace?.name || 'Default Workspace';

  const envName = activeEnvFromList?.name || activeEnvFromLegacy?.name || 'No Environment';
  const pendingCount = pendingQueue.length;

  const filteredWorkspaces = workspaces.filter((w) =>
    w.name.toLowerCase().includes(workspaceSearch.toLowerCase()) ||
    (w.description && w.description.toLowerCase().includes(workspaceSearch.toLowerCase()))
  );

  // Close combobox when clicking outside or pressing Escape
  useEffect(() => {
    if (!isWorkspaceMenuOpen) return;

    const handleMouseDown = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(e.target as Node)
      ) {
        setIsWorkspaceMenuOpen(false);
        setIsCreatingInline(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsWorkspaceMenuOpen(false);
        setIsCreatingInline(false);
      }
    };

    document.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isWorkspaceMenuOpen]);

  // Focus search input when popover opens
  useEffect(() => {
    if (isWorkspaceMenuOpen) {
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 50);
    } else {
      setWorkspaceSearch('');
      setIsCreatingInline(false);
      setNewWsName('');
    }
  }, [isWorkspaceMenuOpen]);

  const handleSelectWorkspace = async (id: string) => {
    await selectWorkspace(id);
    setIsWorkspaceMenuOpen(false);
  };

  const handleCreateWorkspace = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newWsName.trim()) return;
    const created = await createNewWorkspace(newWsName.trim());
    if (created) {
      await selectWorkspace(created.id);
    }
    setNewWsName('');
    setIsCreatingInline(false);
    setIsWorkspaceMenuOpen(false);
  };

  return (
    <footer className="relative h-6 bg-header border-t border-border flex items-center justify-between px-3 shrink-0 select-none text-2xs font-mono text-muted-foreground">
      {/* Left Info & Quick Env Trigger (Bottom Left Corner) */}
      <div className="flex items-center gap-3">
        {/* Quick Env Switcher Trigger Button (Corner Left Bottom) */}
        <button
          onClick={() => setQuickVarModalOpen(true)}
          className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-primary/10 hover:bg-primary/20 border border-primary/30 text-primary font-sans font-bold transition-colors cursor-pointer"
          title="Click or press 'V' to open Quick Environment Switcher"
        >
          <MingCuteIcon name="earth_line" size={13} />
          <span>Env: {envName}</span>
          <MingCuteIcon name="down_line" size={12} className="opacity-70" />
        </button>

        <span className="text-border">|</span>

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

      {/* Right Badges & Workspace Combobox Trigger */}
      <div className="flex items-center gap-3">
        {/* Pending Intercept Counter Badge */}
        {pendingCount > 0 && (
          <div className="flex items-center gap-1 text-rose-500 font-semibold animate-pulse">
            <MingCuteIcon name="shield_line" size={12} />
            <span>{pendingCount} Pending Intercept</span>
          </div>
        )}

        {/* Active Workspace Interactive Trigger Button (Bottom Right Corner) */}
        <button
          ref={triggerRef}
          type="button"
          onClick={() => setIsWorkspaceMenuOpen(!isWorkspaceMenuOpen)}
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded border transition-colors cursor-pointer font-sans text-2xs ${
            isWorkspaceMenuOpen
              ? 'bg-primary/15 border-primary/50 text-primary font-semibold'
              : 'bg-surface hover:bg-neutral-subtle border-border text-foreground font-medium'
          }`}
          title="Click to switch active workspace"
        >
          <MingCuteIcon name="folder_2_line" size={13} className="text-primary shrink-0" />
          <span className="text-muted-foreground uppercase text-3xs tracking-wider font-mono">Workspace:</span>
          <span className="font-semibold text-foreground truncate max-w-[140px]">{workspaceName}</span>
          <MingCuteIcon
            name={isWorkspaceMenuOpen ? 'down_line' : 'up_line'}
            size={12}
            className="opacity-70 shrink-0"
          />
        </button>
      </div>

      {/* Workspace Quick Switcher Combobox Popover */}
      {isWorkspaceMenuOpen && (
        <div
          ref={popoverRef}
          className="absolute right-2 bottom-7 z-50 w-80 bg-surface border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col font-sans animate-in fade-in zoom-in-95 duration-150"
        >
          {/* Header & Search */}
          <div className="p-2.5 border-b border-border bg-header flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 font-semibold text-foreground text-xs">
                <MingCuteIcon name="folder_2_line" size={14} className="text-primary" />
                <span>Switch Workspace</span>
              </div>
              <span className="text-3xs text-muted-foreground font-mono">
                {workspaces.length} {workspaces.length === 1 ? 'workspace' : 'workspaces'}
              </span>
            </div>

            {/* Search Input */}
            <div className="relative flex items-center">
              <MingCuteIcon
                name="search_line"
                size={13}
                className="absolute left-2 text-muted-foreground pointer-events-none"
              />
              <input
                ref={searchInputRef}
                type="text"
                value={workspaceSearch}
                onChange={(e) => setWorkspaceSearch(e.target.value)}
                placeholder="Search workspaces..."
                className="w-full pl-7 pr-6 py-1 bg-background border border-border rounded-lg text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary transition-colors"
              />
              {workspaceSearch && (
                <button
                  type="button"
                  onClick={() => setWorkspaceSearch('')}
                  className="absolute right-1.5 p-0.5 text-muted-foreground hover:text-foreground cursor-pointer"
                >
                  <MingCuteIcon name="close_line" size={12} />
                </button>
              )}
            </div>
          </div>

          {/* Workspaces List */}
          <div className="max-h-60 overflow-y-auto p-1.5 space-y-0.5 no-scrollbar">
            {filteredWorkspaces.length === 0 ? (
              <div className="py-6 text-center text-xs text-muted-foreground italic flex flex-col items-center gap-1">
                <MingCuteIcon name="search_line" size={20} className="opacity-40" />
                <span>No workspaces matching &ldquo;{workspaceSearch}&rdquo;</span>
              </div>
            ) : (
              filteredWorkspaces.map((ws) => {
                const isActive = ws.id === activeWorkspaceId;
                return (
                  <button
                    key={ws.id}
                    type="button"
                    onClick={() => handleSelectWorkspace(ws.id)}
                    className={`w-full text-left px-2.5 py-2 rounded-lg flex items-center justify-between gap-2 transition-colors cursor-pointer group ${
                      isActive
                        ? 'bg-primary/15 text-primary font-semibold'
                        : 'hover:bg-neutral-subtle text-foreground'
                    }`}
                  >
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <MingCuteIcon
                        name={isActive ? 'folder_open_line' : 'folder_line'}
                        size={15}
                        className={`shrink-0 ${isActive ? 'text-primary' : 'text-amber-500 group-hover:text-primary'}`}
                      />
                      <div className="flex flex-col min-w-0 truncate">
                        <span className="text-xs truncate">{ws.name}</span>
                        {ws.description && (
                          <span className="text-3xs text-muted-foreground truncate">{ws.description}</span>
                        )}
                      </div>
                    </div>

                    {isActive && (
                      <span className="flex items-center gap-1 text-primary text-3xs shrink-0 font-mono">
                        <MingCuteIcon name="check_line" size={14} />
                        Active
                      </span>
                    )}
                  </button>
                );
              })
            )}
          </div>

          {/* Inline New Workspace Form */}
          {isCreatingInline && (
            <form onSubmit={handleCreateWorkspace} className="p-2 border-t border-border bg-header flex items-center gap-1.5">
              <input
                type="text"
                autoFocus
                value={newWsName}
                onChange={(e) => setNewWsName(e.target.value)}
                placeholder="Workspace name..."
                className="flex-1 px-2 py-1 bg-background border border-primary rounded-md text-xs text-foreground focus:outline-none"
              />
              <button
                type="submit"
                className="px-2 py-1 bg-primary text-primary-foreground text-xs rounded-md font-semibold hover:opacity-90 cursor-pointer"
              >
                Create
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsCreatingInline(false);
                  setNewWsName('');
                }}
                className="p-1 text-muted-foreground hover:text-foreground cursor-pointer"
              >
                <MingCuteIcon name="close_line" size={13} />
              </button>
            </form>
          )}

          {/* Bottom Actions Footer */}
          <div className="p-1.5 border-t border-border bg-header flex items-center justify-between text-xs text-muted-foreground">
            <button
              type="button"
              onClick={() => setIsCreatingInline(true)}
              className="flex items-center gap-1 px-2 py-1 rounded hover:bg-neutral-subtle hover:text-foreground transition-colors cursor-pointer"
            >
              <MingCuteIcon name="plus_line" size={13} />
              <span>New</span>
            </button>

            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => {
                  setIsWorkspaceMenuOpen(false);
                  openImportModalForWorkspace();
                }}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-neutral-subtle hover:text-foreground transition-colors cursor-pointer"
                title="Import Workspace JSON"
              >
                <MingCuteIcon name="file_import_line" size={13} />
                <span>Import</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  setIsWorkspaceMenuOpen(false);
                  setActiveModule('workspace');
                }}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-neutral-subtle hover:text-primary transition-colors cursor-pointer font-medium"
                title="Open full Workspace Manager"
              >
                <MingCuteIcon name="external_link_line" size={13} />
                <span>Manage</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </footer>
  );
};
