import React, { useState, useRef, useEffect } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const WorkspaceSelector: React.FC = () => {
  const {
    workspaces,
    activeWorkspaceId,
    selectWorkspace,
    createNewWorkspace,
    setImportModalOpen,
  } = useWorkspaceStore();

  const [isOpen, setIsOpen] = useState(false);
  const [newWsName, setNewWsName] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) || workspaces[0] || null;

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleCreateWorkspace = async () => {
    if (!newWsName.trim()) return;
    await createNewWorkspace(newWsName.trim());
    setNewWsName('');
    setIsCreating(false);
    setIsOpen(false);
  };

  return (
    <div className="relative font-sans text-xs select-none" ref={dropdownRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-surface hover:bg-neutral-subtle border border-border text-foreground font-semibold transition-colors cursor-pointer shadow-2xs"
        title="Switch active workspace"
      >
        <MingCuteIcon name="folder_block_line" size={16} className="text-primary" />
        <span className="max-w-[140px] truncate">{activeWorkspace?.name || 'Workspace'}</span>
        <MingCuteIcon name="down_line" size={14} className="opacity-60" />
      </button>

      {isOpen && (
        <div className="absolute left-0 mt-2 w-64 bg-surface border border-border rounded-xl shadow-2xl p-2 z-50 flex flex-col gap-1">
          <div className="px-2 py-1 text-[11px] font-bold text-muted-foreground uppercase tracking-wider">
            Workspaces ({workspaces.length})
          </div>

          <div className="max-h-56 overflow-y-auto space-y-0.5 no-scrollbar">
            {workspaces.map((ws) => {
              const isActive = ws.id === activeWorkspaceId;
              return (
                <button
                  key={ws.id}
                  onClick={() => {
                    selectWorkspace(ws.id);
                    setIsOpen(false);
                  }}
                  className={`w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg transition-colors cursor-pointer text-left ${
                    isActive
                      ? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
                      : 'hover:bg-neutral-subtle text-foreground'
                  }`}
                >
                  <div className="flex flex-col truncate">
                    <span className="truncate text-xs font-semibold">{ws.name}</span>
                    {ws.description && (
                      <span className="truncate text-[10px] opacity-75">{ws.description}</span>
                    )}
                  </div>
                  {isActive && <MingCuteIcon name="check_line" size={14} />}
                </button>
              );
            })}
          </div>

          <div className="border-t border-border/80 pt-1 mt-1 flex flex-col gap-1">
            {isCreating ? (
              <div className="p-1 flex items-center gap-1">
                <input
                  type="text"
                  autoFocus
                  placeholder="Workspace Name..."
                  value={newWsName}
                  onChange={(e) => setNewWsName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleCreateWorkspace();
                    if (e.key === 'Escape') setIsCreating(false);
                  }}
                  className="w-full bg-background border border-primary rounded px-2 py-1 text-xs text-foreground focus:outline-none"
                />
                <button
                  onClick={handleCreateWorkspace}
                  className="px-2 py-1 rounded bg-primary text-primary-foreground font-semibold text-xs cursor-pointer"
                >
                  Add
                </button>
              </div>
            ) : (
              <button
                onClick={() => setIsCreating(true)}
                className="w-full px-2.5 py-1 rounded-lg text-left text-primary hover:bg-neutral-subtle font-medium flex items-center gap-1.5 cursor-pointer text-xs"
              >
                <MingCuteIcon name="plus_line" size={14} />
                <span>New Workspace</span>
              </button>
            )}

            <button
              onClick={() => {
                setIsOpen(false);
                setImportModalOpen(true);
              }}
              className="w-full px-2.5 py-1 rounded-lg text-left text-foreground hover:bg-neutral-subtle font-medium flex items-center gap-1.5 cursor-pointer text-xs border-t border-border/50 pt-1.5"
            >
              <MingCuteIcon name="file_import_line" size={14} className="text-amber-500" />
              <span>Import JSON Project</span>
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
