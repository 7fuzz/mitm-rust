import React, { useState } from 'react';
import type { Environment } from '../../../services/tauri/bridge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ContextMenu, type ContextMenuItem } from '../../common/ContextMenu';

interface EnvironmentListProps {
  environments: Environment[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onCreate: (name: string) => void;
  onRename: (id: string, name: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (env: Environment) => void;
  onSetActive: (env: Environment) => void;
}

export const EnvironmentList: React.FC<EnvironmentListProps> = ({
  environments,
  selectedId,
  onSelect,
  onCreate,
  onRename,
  onDuplicate,
  onDelete,
  onSetActive,
}) => {
  const [isCreating, setIsCreating] = useState(false);
  const [draftName, setDraftName] = useState('');
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; env: Environment } | null>(null);

  const submitCreate = () => {
    if (draftName.trim()) onCreate(draftName.trim());
    setDraftName('');
    setIsCreating(false);
  };

  const submitRename = (id: string) => {
    onRename(id, draftName);
    setRenamingId(null);
  };

  const startRename = (env: Environment) => {
    setRenamingId(env.id);
    setDraftName(env.name);
  };

  const menuItems = (env: Environment): ContextMenuItem[] => [
    { label: 'Set as active', icon: 'check_line', disabled: env.isActive, action: () => onSetActive(env) },
    { label: 'Rename', icon: 'edit_line', action: () => startRename(env) },
    { label: 'Duplicate', icon: 'copy_line', action: () => onDuplicate(env.id) },
    { label: 'Delete', icon: 'delete_2_line', danger: true, action: () => onDelete(env) },
  ];

  const nameInput = (onSubmit: () => void, onCancel: () => void, placeholder: string) => (
    <input
      type="text"
      autoFocus
      value={draftName}
      onChange={(e) => setDraftName(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') onSubmit();
        if (e.key === 'Escape') onCancel();
      }}
      onBlur={onSubmit}
      placeholder={placeholder}
      className="w-full bg-background border border-primary rounded px-2 py-1 text-xs text-foreground focus:outline-none"
    />
  );

  return (
    <div className="h-full flex flex-col select-none">
      <div className="h-10 px-3 bg-header border-b border-border flex items-center gap-2 shrink-0">
        <span className="flex-1 text-3xs font-semibold uppercase tracking-wider text-muted-foreground">
          Environments <span className="font-mono">{environments.length}</span>
        </span>
        <button
          onClick={() => {
            setDraftName('');
            setIsCreating(true);
          }}
          className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
          title="New environment"
        >
          <MingCuteIcon name="plus_line" size={14} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-1.5 space-y-0.5">
        {environments.map((env) =>
          renamingId === env.id ? (
            <div key={env.id}>{nameInput(() => submitRename(env.id), () => setRenamingId(null), 'Environment name')}</div>
          ) : (
            <div
              key={env.id}
              onClick={() => onSelect(env.id)}
              onDoubleClick={() => startRename(env)}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, env });
              }}
              className={`group flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors ${
                env.id === selectedId
                  ? 'bg-primary/10 text-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
              }`}
              title="Double-click to rename, right-click for more"
            >
              <span
                className={`w-1.5 h-1.5 rounded-full shrink-0 ${env.isActive ? 'bg-emerald-400' : 'bg-muted-foreground/30'}`}
                title={env.isActive ? 'Active environment' : undefined}
              />
              <span className={`flex-1 truncate text-xs ${env.id === selectedId ? 'font-semibold' : 'font-medium'}`}>
                {env.name}
              </span>
              <span className="text-3xs font-mono text-muted-foreground group-hover:hidden">{env.variables?.length ?? 0}</span>
              <span className="hidden group-hover:flex items-center">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDuplicate(env.id);
                  }}
                  className="p-0.5 rounded text-muted-foreground hover:text-foreground cursor-pointer"
                  title="Duplicate"
                >
                  <MingCuteIcon name="copy_line" size={12} />
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(env);
                  }}
                  className="p-0.5 rounded text-muted-foreground hover:text-rose-500 cursor-pointer"
                  title="Delete"
                >
                  <MingCuteIcon name="delete_2_line" size={12} />
                </button>
              </span>
            </div>
          )
        )}

        {isCreating && nameInput(submitCreate, () => setIsCreating(false), 'New environment name')}

        {environments.length === 0 && !isCreating && (
          <div className="px-2 py-4 text-center text-2xs text-muted-foreground italic">No environments yet</div>
        )}
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={menuItems(contextMenu.env)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};
