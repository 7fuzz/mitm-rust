import React from 'react';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { CollectionRequestNode } from './CollectionRequestNode';
import { TreeGuideLines } from './TreeGuideLines';
import type { CollectionTreeItem, RequestItem } from '../../../../services/tauri/bridge';

interface CollectionFolderNodeProps {
  item: CollectionTreeItem;
  depth: number;
  isLast?: boolean;
  ancestorIsLast?: boolean[];
  isSearching: boolean;
  expandedFolders: Record<string, boolean>;
  addingFolderParentId: string | null | 'root';
  folderName: string;
  addingReqFolderId: string | null;
  reqName: string;
  dragTargetFolderId: string | null;
  editingId: string | null;
  editingType: 'collection' | 'request' | null;
  editingName: string;
  onSetFolderName: (name: string) => void;
  onSetReqName: (name: string) => void;
  onSetEditingName: (name: string) => void;
  onSetAddingFolderParentId: (id: string | null | 'root') => void;
  onSetAddingReqFolderId: (id: string | null) => void;
  onToggleFolder: (id: string, e: React.MouseEvent) => void;
  onAddFolder: (parentId: string | null) => void;
  onAddRequest: (colId: string) => void;
  onSaveRename: () => void;
  onCancelRename: () => void;
  onContextMenu: (e: React.MouseEvent, type: 'collection' | 'request' | 'root', id?: string, item?: any) => void;
  onDragStartFolder: (e: React.DragEvent, item: CollectionTreeItem) => void;
  onDragStartRequest: (e: React.DragEvent, req: RequestItem) => void;
  onDragOverFolder: (e: React.DragEvent, targetFolderId: string) => void;
  onDragLeaveFolder: (e: React.DragEvent, targetFolderId: string) => void;
  onDropOnFolder: (e: React.DragEvent, targetFolderId: string) => void;
  onOpenRequestTab: (req: RequestItem) => void;
  onDeleteCollection: (id: string) => void;
  onDeleteRequest: (id: string) => void;
}

export const CollectionFolderNode: React.FC<CollectionFolderNodeProps> = ({
  item,
  depth,
  isLast = false,
  ancestorIsLast = [],
  isSearching,
  expandedFolders,
  addingFolderParentId,
  folderName,
  addingReqFolderId,
  reqName,
  dragTargetFolderId,
  editingId,
  editingType,
  editingName,
  onSetFolderName,
  onSetReqName,
  onSetEditingName,
  onSetAddingFolderParentId,
  onSetAddingReqFolderId,
  onToggleFolder,
  onAddFolder,
  onAddRequest,
  onSaveRename,
  onCancelRename,
  onContextMenu,
  onDragStartFolder,
  onDragStartRequest,
  onDragOverFolder,
  onDragLeaveFolder,
  onDropOnFolder,
  onOpenRequestTab,
  onDeleteCollection,
  onDeleteRequest,
}) => {
  const isExpanded = isSearching ? true : expandedFolders[item.id] !== false;
  const isAddingSubfolder = addingFolderParentId === item.id;
  const isAddingSubReq = addingReqFolderId === item.id;
  const isDropTarget = dragTargetFolderId === item.id;
  const isRenaming = editingId === item.id && editingType === 'collection';

  const childrenCount = item.children?.length || 0;
  const requestsCount = item.requests?.length || 0;
  const nextAncestorIsLast = depth === 0 ? [] : [...ancestorIsLast, isLast];

  return (
    <div key={item.id} className="flex flex-col select-none text-xs">
      {/* Folder Node Header */}
      <div
        draggable
        onDragStart={(e) => onDragStartFolder(e, item)}
        onDragOver={(e) => onDragOverFolder(e, item.id)}
        onDragLeave={(e) => onDragLeaveFolder(e, item.id)}
        onDrop={(e) => onDropOnFolder(e, item.id)}
        onClick={(e) => onToggleFolder(item.id, e)}
        onContextMenu={(e) => onContextMenu(e, 'collection', item.id, item)}
        className={`group flex items-center justify-between pr-2 rounded-md hover:bg-neutral-subtle/80 cursor-pointer font-sans transition-all h-7 ${
          isDropTarget ? 'ring-2 ring-primary bg-primary/10' : ''
        }`}
        style={{ paddingLeft: depth === 0 ? '6px' : '0px' }}
      >
        <div className="flex items-center gap-1.5 min-w-0 font-medium text-foreground flex-1 self-stretch h-full">
          {/* Tree Structure Guide Lines */}
          <TreeGuideLines depth={depth} isLast={isLast} ancestorIsLast={ancestorIsLast} />

          <MingCuteIcon
            name={isExpanded ? 'folder_open_line' : 'folder_line'}
            size={15}
            className="text-amber-500 shrink-0 ml-0.5"
          />
          {isRenaming ? (
            <input
              type="text"
              autoFocus
              value={editingName}
              onChange={(e) => onSetEditingName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') onSaveRename();
                if (e.key === 'Escape') onCancelRename();
              }}
              onBlur={onSaveRename}
              onClick={(e) => e.stopPropagation()}
              className="bg-background border border-primary rounded px-1 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
            />
          ) : (
            <span className="truncate">{item.name}</span>
          )}
        </div>

        {/* Actions: Add Subfolder / Add Request / Delete */}
        <div className="opacity-0 group-hover:opacity-100 flex items-center gap-1 shrink-0 transition-opacity">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSetAddingFolderParentId(item.id);
              onSetFolderName('');
            }}
            className="text-muted-foreground hover:text-primary p-0.5 rounded transition-colors cursor-pointer"
            title="Add Subfolder"
          >
            <MingCuteIcon name="folder_add_line" size={13} />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSetAddingReqFolderId(item.id);
              onSetReqName('');
            }}
            className="text-muted-foreground hover:text-primary p-0.5 rounded transition-colors cursor-pointer"
            title="Add Request"
          >
            <MingCuteIcon name="plus_line" size={13} />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDeleteCollection(item.id);
            }}
            className="text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-colors cursor-pointer"
            title="Delete Folder"
          >
            <MingCuteIcon name="close_line" size={13} />
          </button>
        </div>
      </div>

      {/* Folder Children & Sub-Requests */}
      {isExpanded && (
        <div className="flex flex-col">
          {/* Inline Subfolder Input */}
          {isAddingSubfolder && (
            <div className="pr-2 flex items-center gap-1 h-7">
              <TreeGuideLines
                depth={depth + 1}
                isLast={childrenCount === 0 && requestsCount === 0}
                ancestorIsLast={nextAncestorIsLast}
              />
              <input
                type="text"
                autoFocus
                placeholder="Subfolder name..."
                value={folderName}
                onChange={(e) => onSetFolderName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') onAddFolder(item.id);
                  if (e.key === 'Escape') onSetAddingFolderParentId(null);
                }}
                className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
              />
              <button
                type="button"
                onClick={() => onAddFolder(item.id)}
                className="p-1 text-emerald-400 hover:bg-emerald-500/10 rounded cursor-pointer shrink-0"
                title="Save Subfolder"
              >
                <MingCuteIcon name="check_line" size={13} />
              </button>
              <button
                type="button"
                onClick={() => onSetAddingFolderParentId(null)}
                className="p-1 text-muted-foreground hover:text-rose-400 hover:bg-rose-500/10 rounded cursor-pointer shrink-0"
                title="Cancel"
              >
                <MingCuteIcon name="close_line" size={13} />
              </button>
            </div>
          )}

          {/* Inline Sub-Request Input */}
          {isAddingSubReq && (
            <div className="pr-2 flex items-center gap-1 h-7">
              <TreeGuideLines
                depth={depth + 1}
                isLast={requestsCount === 0}
                ancestorIsLast={nextAncestorIsLast}
              />
              <input
                type="text"
                autoFocus
                placeholder="Request name..."
                value={reqName}
                onChange={(e) => onSetReqName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') onAddRequest(item.id);
                  if (e.key === 'Escape') onSetAddingReqFolderId(null);
                }}
                className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
              />
              <button
                type="button"
                onClick={() => onAddRequest(item.id)}
                className="p-1 text-emerald-400 hover:bg-emerald-500/10 rounded cursor-pointer shrink-0"
                title="Save Request"
              >
                <MingCuteIcon name="check_line" size={13} />
              </button>
              <button
                type="button"
                onClick={() => onSetAddingReqFolderId(null)}
                className="p-1 text-muted-foreground hover:text-rose-400 hover:bg-rose-500/10 rounded cursor-pointer shrink-0"
                title="Cancel"
              >
                <MingCuteIcon name="close_line" size={13} />
              </button>
            </div>
          )}

          {/* Subfolders (Recursive) */}
          {item.children?.map((child, idx) => {
            const hasRequests = requestsCount > 0;
            const isChildLast = idx === childrenCount - 1 && !hasRequests;
            return (
              <CollectionFolderNode
                key={child.id}
                item={child}
                depth={depth + 1}
                isLast={isChildLast}
                ancestorIsLast={nextAncestorIsLast}
                isSearching={isSearching}
                expandedFolders={expandedFolders}
                addingFolderParentId={addingFolderParentId}
                folderName={folderName}
                addingReqFolderId={addingReqFolderId}
                reqName={reqName}
                dragTargetFolderId={dragTargetFolderId}
                editingId={editingId}
                editingType={editingType}
                editingName={editingName}
                onSetFolderName={onSetFolderName}
                onSetReqName={onSetReqName}
                onSetEditingName={onSetEditingName}
                onSetAddingFolderParentId={onSetAddingFolderParentId}
                onSetAddingReqFolderId={onSetAddingReqFolderId}
                onToggleFolder={onToggleFolder}
                onAddFolder={onAddFolder}
                onAddRequest={onAddRequest}
                onSaveRename={onSaveRename}
                onCancelRename={onCancelRename}
                onContextMenu={onContextMenu}
                onDragStartFolder={onDragStartFolder}
                onDragStartRequest={onDragStartRequest}
                onDragOverFolder={onDragOverFolder}
                onDragLeaveFolder={onDragLeaveFolder}
                onDropOnFolder={onDropOnFolder}
                onOpenRequestTab={onOpenRequestTab}
                onDeleteCollection={onDeleteCollection}
                onDeleteRequest={onDeleteRequest}
              />
            );
          })}

          {/* Folder Requests */}
          {item.requests?.map((req, idx) => {
            const isReqLast = idx === requestsCount - 1;
            return (
              <CollectionRequestNode
                key={req.id}
                request={req}
                depth={depth + 1}
                isLast={isReqLast}
                ancestorIsLast={nextAncestorIsLast}
                isSearching={isSearching}
                editingId={editingId}
                editingType={editingType}
                editingName={editingName}
                onSetEditingName={onSetEditingName}
                onSaveRename={onSaveRename}
                onCancelRename={onCancelRename}
                onOpenRequestTab={onOpenRequestTab}
                onContextMenu={onContextMenu}
                onDragStart={onDragStartRequest}
                onDeleteRequest={onDeleteRequest}
              />
            );
          })}
        </div>
      )}
    </div>
  );
};
