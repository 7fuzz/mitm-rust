import React, { useState, useEffect, useRef } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import type { Workspace } from '../../../services/tauri/bridge';

interface ContextMenuState {
  x: number;
  y: number;
  workspace: Workspace;
}

export const WorkspaceSelector: React.FC = () => {
  const {
    workspaces,
    activeWorkspaceId,
    selectWorkspace,
    createNewWorkspace,
    updateWorkspaceDetails,
    deleteWorkspaceById,
    setImportModalOpen,
    openImportModalForWorkspace,
  } = useWorkspaceStore();

  const [isCreating, setIsCreating] = useState(false);
  const [newWsName, setNewWsName] = useState('');
  const [deletingWsId, setDeletingWsId] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  // Inline renaming state
  const [renamingWsId, setRenamingWsId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');

  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      } else if (!menuRef.current) {
        setContextMenu(null);
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
  };

  const handleConfirmDelete = async () => {
    if (!deletingWsId) return;
    await deleteWorkspaceById(deletingWsId);
    setDeletingWsId(null);
  };

  const handleStartRename = (ws: Workspace) => {
    setRenamingWsId(ws.id);
    setRenameValue(ws.name);
    setContextMenu(null);
  };

  const handleSaveRename = (ws: Workspace) => {
    if (renameValue.trim() && renameValue.trim() !== ws.name) {
      updateWorkspaceDetails({
        ...ws,
        name: renameValue.trim(),
        updatedAtMs: Date.now(),
      });
    }
    setRenamingWsId(null);
  };

  const deletingWorkspace = workspaces.find((w) => w.id === deletingWsId);

  return (
    <div className="flex items-center justify-between w-full font-sans text-xs select-none gap-2">
      {/* Scrollable Workspace Tabs Bar */}
      <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar py-0.5 flex-1">
        {workspaces.map((ws) => {
          const isActive = ws.id === activeWorkspaceId;
          const canDelete = workspaces.length > 1;
          const isRenaming = renamingWsId === ws.id;

          return (
            <div
              key={ws.id}
              onClick={() => selectWorkspace(ws.id)}
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setContextMenu({ x: e.clientX, y: e.clientY, workspace: ws });
              }}
              className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all cursor-pointer shrink-0 ${
                isActive
                  ? 'bg-primary text-primary-foreground font-semibold border-primary shadow-xs'
                  : 'bg-surface hover:bg-neutral-subtle border-border text-foreground'
              }`}
              title="Click to activate, Right-click for options"
            >
              <MingCuteIcon
                name="folder_block_line"
                size={14}
                className={isActive ? 'text-primary-foreground' : 'text-primary'}
              />

              {isRenaming ? (
                <input
                  type="text"
                  autoFocus
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onBlur={() => handleSaveRename(ws)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSaveRename(ws);
                    if (e.key === 'Escape') setRenamingWsId(null);
                  }}
                  onClick={(e) => e.stopPropagation()}
                  className="w-28 bg-background border border-primary text-foreground rounded px-1.5 py-0.5 text-xs focus:outline-none"
                />
              ) : (
                <span className="truncate max-w-[130px]">
                  {typeof ws.name === 'string' ? ws.name : String(ws.name || 'Workspace')}
                </span>
              )}

              {isActive && !isRenaming && (
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shrink-0" />
              )}

              {canDelete && !isRenaming && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setDeletingWsId(ws.id);
                  }}
                  title="Delete workspace"
                  className={`p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-black/20 transition-opacity cursor-pointer ${
                    isActive ? 'text-primary-foreground' : 'text-rose-500'
                  }`}
                >
                  <MingCuteIcon name="close_line" size={13} />
                </button>
              )}
            </div>
          );
        })}

        {/* Inline Create New Workspace Tab */}
        {isCreating ? (
          <div className="flex items-center gap-1 bg-surface border border-primary rounded-lg p-1 shrink-0">
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
              className="w-32 bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none"
            />
            <button
              onClick={handleCreateWorkspace}
              className="px-2 py-0.5 bg-primary text-primary-foreground font-semibold rounded text-xs cursor-pointer shadow-2xs"
            >
              Add
            </button>
            <button
              onClick={() => setIsCreating(false)}
              className="p-0.5 text-muted-foreground hover:text-foreground"
            >
              <MingCuteIcon name="close_line" size={13} />
            </button>
          </div>
        ) : (
          <button
            onClick={() => setIsCreating(true)}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-surface/60 hover:bg-surface border border-dashed border-border text-muted-foreground hover:text-foreground font-medium transition-colors cursor-pointer shrink-0"
            title="Create new workspace"
          >
            <MingCuteIcon name="plus_line" size={14} className="text-primary" />
            <span>New Workspace</span>
          </button>
        )}
      </div>

      {/* Import Button */}
      <button
        onClick={() => setImportModalOpen(true)}
        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface hover:bg-neutral-subtle border border-border text-foreground font-semibold transition-colors cursor-pointer shadow-2xs shrink-0"
        title="Import Postman or JSON project file"
      >
        <MingCuteIcon name="file_import_line" size={15} className="text-amber-500" />
        <span>Import JSON</span>
      </button>

      {/* Right-Click Context Menu */}
      {contextMenu && (
        <div
          ref={menuRef}
          style={{ top: contextMenu.y + 4, left: contextMenu.x }}
          className="fixed z-50 bg-surface border border-border rounded-xl shadow-2xl p-1 w-48 text-foreground font-sans text-xs flex flex-col gap-0.5 animate-in fade-in zoom-in-95 duration-100"
        >
          <div className="px-2.5 py-1 text-[10px] font-bold text-muted-foreground uppercase tracking-wider border-b border-border/60 mb-0.5 truncate">
            {contextMenu.workspace.name}
          </div>

          <button
            onClick={() => handleStartRename(contextMenu.workspace)}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-neutral-subtle text-foreground text-left cursor-pointer transition-colors"
          >
            <MingCuteIcon name="edit_line" size={14} className="text-primary" />
            <span>Rename Workspace</span>
          </button>

          <button
            onClick={() => {
              selectWorkspace(contextMenu.workspace.id);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-neutral-subtle text-foreground text-left cursor-pointer transition-colors"
          >
            <MingCuteIcon name="settings_3_line" size={14} className="text-blue-500" />
            <span>View Workspace Details</span>
          </button>

          <button
            onClick={() => {
              openImportModalForWorkspace(contextMenu.workspace.id);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-neutral-subtle text-foreground text-left cursor-pointer transition-colors"
          >
            <MingCuteIcon name="file_import_line" size={14} className="text-amber-500" />
            <span>Import to this Workspace</span>
          </button>

          {workspaces.length > 1 && (
            <button
              onClick={() => {
                setDeletingWsId(contextMenu.workspace.id);
                setContextMenu(null);
              }}
              className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-rose-500/10 text-rose-500 text-left cursor-pointer transition-colors border-t border-border/50 mt-0.5 pt-1.5"
            >
              <MingCuteIcon name="delete_2_line" size={14} />
              <span>Delete Workspace</span>
            </button>
          )}
        </div>
      )}

      {/* Delete Confirmation Modal */}
      {deletingWsId && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-center justify-center p-4">
          <div className="bg-surface border border-border rounded-xl shadow-2xl p-5 w-full max-w-md text-foreground flex flex-col gap-4">
            <div className="flex items-center gap-2 text-rose-500 font-bold text-sm border-b border-border pb-3">
              <MingCuteIcon name="alert_line" size={20} />
              <span>Delete Workspace</span>
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              Are you sure you want to delete workspace <strong className="text-foreground">"{deletingWorkspace?.name}"</strong>?
              All collections, environment variables, and saved requests in this workspace will be permanently removed.
            </p>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-border">
              <button
                onClick={() => setDeletingWsId(null)}
                className="px-3.5 py-1.5 rounded bg-header hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground cursor-pointer font-medium"
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmDelete}
                className="px-4 py-1.5 rounded bg-rose-600 hover:bg-rose-700 text-white font-semibold cursor-pointer shadow-2xs"
              >
                Delete Workspace
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
