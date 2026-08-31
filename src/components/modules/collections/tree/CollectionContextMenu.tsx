import React from 'react';
import { MingCuteIcon } from '../../../common/MingCuteIcon';

export interface ContextMenuState {
  x: number;
  y: number;
  type: 'collection' | 'request' | 'root';
  id?: string;
  item?: any;
}

interface CollectionContextMenuProps {
  contextMenu: ContextMenuState | null;
  activeWorkspaceId: string | null;
  onClose: () => void;
  onAddRequest: (colId: string) => void;
  onAddSubfolder: (colId: string) => void;
  onOpenAutoExtract: (type: 'collection' | 'request', item: any) => void;
  onDuplicateFolder: (colId: string) => void;
  onRenameFolder: (colId: string, name: string) => void;
  onDeleteFolder: (colId: string) => void;
  onOpenRequestTab: (req: any) => void;
  onDuplicateRequest: (reqId: string) => void;
  onRenameRequest: (reqId: string, name: string) => void;
  onDeleteRequest: (reqId: string) => void;
  onAddRootFolder: () => void;
}

export const CollectionContextMenu: React.FC<CollectionContextMenuProps> = ({
  contextMenu,
  activeWorkspaceId,
  onClose,
  onAddRequest,
  onAddSubfolder,
  onOpenAutoExtract,
  onDuplicateFolder,
  onRenameFolder,
  onDeleteFolder,
  onOpenRequestTab,
  onDuplicateRequest,
  onRenameRequest,
  onDeleteRequest,
  onAddRootFolder,
}) => {
  if (!contextMenu) return null;

  return (
    <div
      style={{ top: `${contextMenu.y}px`, left: `${contextMenu.x}px` }}
      onClick={(e) => e.stopPropagation()}
      className="fixed z-50 bg-surface border border-border rounded-lg shadow-xl py-1 text-xs min-w-[170px] text-foreground font-sans flex flex-col"
    >
      {contextMenu.type === 'collection' && (
        <>
          <button
            type="button"
            onClick={() => {
              if (contextMenu.id) onAddRequest(contextMenu.id);
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="plus_line" size={14} className="text-primary" />
            <span>Add Request</span>
          </button>

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id) onAddSubfolder(contextMenu.id);
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="folder_add_line" size={14} className="text-amber-500" />
            <span>Add Subfolder</span>
          </button>

          <button
            type="button"
            onClick={() => {
              onOpenAutoExtract('collection', contextMenu.item);
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="lightning_line" size={14} className="text-amber-500" />
            <span>Auto-Extract Rules</span>
          </button>

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id && activeWorkspaceId) {
                onDuplicateFolder(contextMenu.id);
              }
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="copy_line" size={14} className="text-muted-foreground" />
            <span>Duplicate Folder</span>
          </button>

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id) {
                onRenameFolder(contextMenu.id, contextMenu.item?.name || '');
              }
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="edit_line" size={14} className="text-muted-foreground" />
            <span>Rename Folder</span>
          </button>

          <div className="my-1 border-t border-border" />

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id && activeWorkspaceId) {
                onDeleteFolder(contextMenu.id);
              }
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-rose-500/10 text-rose-500 text-left transition-colors cursor-pointer font-medium"
          >
            <MingCuteIcon name="close_line" size={14} />
            <span>Delete Folder</span>
          </button>
        </>
      )}

      {contextMenu.type === 'request' && (
        <>
          <button
            type="button"
            onClick={() => {
              onOpenRequestTab(contextMenu.item);
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="eye_line" size={14} className="text-primary" />
            <span>Open in Tab</span>
          </button>

          <button
            type="button"
            onClick={() => {
              onOpenAutoExtract('request', contextMenu.item);
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="lightning_line" size={14} className="text-amber-500" />
            <span>Auto-Extract Rules</span>
          </button>

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id && activeWorkspaceId) {
                onDuplicateRequest(contextMenu.id);
              }
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="copy_line" size={14} className="text-muted-foreground" />
            <span>Duplicate Request</span>
          </button>

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id) {
                onRenameRequest(contextMenu.id, contextMenu.item?.name || '');
              }
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
          >
            <MingCuteIcon name="edit_line" size={14} className="text-muted-foreground" />
            <span>Rename Request</span>
          </button>

          <div className="my-1 border-t border-border" />

          <button
            type="button"
            onClick={() => {
              if (contextMenu.id && activeWorkspaceId) {
                onDeleteRequest(contextMenu.id);
              }
              onClose();
            }}
            className="flex items-center gap-2 px-3 py-1.5 hover:bg-rose-500/10 text-rose-500 text-left transition-colors cursor-pointer font-medium"
          >
            <MingCuteIcon name="close_line" size={14} />
            <span>Delete Request</span>
          </button>
        </>
      )}

      {contextMenu.type === 'root' && (
        <button
          type="button"
          onClick={() => {
            onAddRootFolder();
            onClose();
          }}
          className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
        >
          <MingCuteIcon name="folder_add_line" size={14} className="text-amber-500" />
          <span>New Root Folder</span>
        </button>
      )}
    </div>
  );
};
