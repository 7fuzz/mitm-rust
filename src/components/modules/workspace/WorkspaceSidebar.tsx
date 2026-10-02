import React, { useMemo, useState } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';
import type { Workspace } from '../../../services/tauri/bridge';

interface WorkspaceSidebarProps {
  onRequestDelete: (workspace: Workspace) => void;
  onExport: (workspaceId: string) => void;
  onHide: () => void;
}

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({ onRequestDelete, onExport, onHide }) => {
  const {
    workspaces,
    activeWorkspaceId,
    selectWorkspace,
    createNewWorkspace,
    updateWorkspaceDetails,
    setImportModalOpen,
    openImportModalForWorkspace,
  } = useWorkspaceStore();

  const [filter, setFilter] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; workspace: Workspace } | null>(null);

  const shown = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return q ? workspaces.filter((w) => w.name.toLowerCase().includes(q)) : workspaces;
  }, [workspaces, filter]);

  const handleCreate = async () => {
    if (!newName.trim()) return;
    await createNewWorkspace(newName.trim());
    setNewName('');
    setIsCreating(false);
  };

  const saveRename = (ws: Workspace) => {
    const name = renameValue.trim();
    if (name && name !== ws.name) {
      updateWorkspaceDetails({ ...ws, name, updatedAtMs: Date.now() });
    }
    setRenamingId(null);
  };

  const menuItems = (ws: Workspace): ContextMenuItem[] => [
    {
      label: 'Rename',
      icon: 'edit_line',
      action: () => {
        setRenamingId(ws.id);
        setRenameValue(ws.name);
      },
    },
    { label: 'Export to JSON', icon: 'download_line', action: () => onExport(ws.id) },
    { label: 'Import into this workspace', icon: 'file_import_line', action: () => openImportModalForWorkspace(ws.id) },
    {
      label: 'Delete',
      icon: 'delete_2_line',
      danger: true,
      disabled: workspaces.length <= 1,
      action: () => onRequestDelete(ws),
    },
  ];

  return (
    <div className="h-full flex flex-col bg-surface border-r border-border select-none">
      <div className="px-3 pt-3 pb-2 flex items-center gap-2">
        <span className="text-3xs font-semibold uppercase tracking-wider text-muted-foreground flex-1">
          Workspaces <span className="font-mono">{workspaces.length}</span>
        </span>
        <button
          onClick={() => setIsCreating(true)}
          className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
          title="New workspace"
        >
          <MingCuteIcon name="plus_line" size={14} />
        </button>
        <button
          onClick={onHide}
          className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
          title="Hide workspace list"
        >
          <MingCuteIcon name="chevron_left_line" size={14} />
        </button>
      </div>

      <div className="px-2 pb-2">
        <div className="relative">
          <MingCuteIcon name="search_line" size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter workspaces..."
            className="w-full bg-background border border-border rounded pl-7 pr-2 py-1 text-2xs text-foreground focus:outline-none focus:border-primary"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-2 space-y-0.5">
        {isCreating && (
          <input
            type="text"
            autoFocus
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleCreate();
              if (e.key === 'Escape') setIsCreating(false);
            }}
            onBlur={() => !newName.trim() && setIsCreating(false)}
            placeholder="Workspace name, then Enter"
            className="w-full bg-background border border-primary rounded px-2 py-1.5 text-xs text-foreground focus:outline-none"
          />
        )}

        {shown.map((ws) => {
          const isActive = ws.id === activeWorkspaceId;
          return renamingId === ws.id ? (
            <input
              key={ws.id}
              type="text"
              autoFocus
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={() => saveRename(ws)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') saveRename(ws);
                if (e.key === 'Escape') setRenamingId(null);
              }}
              className="w-full bg-background border border-primary rounded px-2 py-1.5 text-xs text-foreground focus:outline-none"
            />
          ) : (
            <button
              key={ws.id}
              onClick={() => selectWorkspace(ws.id)}
              onDoubleClick={() => {
                setRenamingId(ws.id);
                setRenameValue(ws.name);
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, workspace: ws });
              }}
              className={`w-full flex items-start gap-2 px-2 py-1.5 rounded text-left cursor-pointer transition-colors ${
                isActive ? 'bg-primary/10 text-foreground' : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
              }`}
              title={`${ws.name}\nDouble-click to rename, right-click for more`}
            >
              <MingCuteIcon
                name="folder_block_line"
                size={14}
                className={`mt-0.5 shrink-0 ${isActive ? 'text-primary' : ''}`}
              />
              <span className="min-w-0 flex-1">
                <span className={`block truncate text-xs ${isActive ? 'font-semibold' : 'font-medium'}`}>{ws.name}</span>
                {ws.description && <span className="block truncate text-3xs text-muted-foreground">{ws.description}</span>}
              </span>
              {isActive && <span className="mt-1.5 w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0" title="Active workspace" />}
            </button>
          );
        })}

        {shown.length === 0 && !isCreating && (
          <div className="px-2 py-4 text-center text-2xs text-muted-foreground italic">No workspaces match</div>
        )}
      </div>

      <div className="p-2 border-t border-border">
        <button
          onClick={() => setImportModalOpen(true)}
          className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded border border-dashed border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle text-xs cursor-pointer transition-colors"
          title="Import a Postman collection or exported workspace JSON"
        >
          <MingCuteIcon name="file_import_line" size={14} />
          <span>Import workspace</span>
        </button>
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={menuItems(contextMenu.workspace)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};
