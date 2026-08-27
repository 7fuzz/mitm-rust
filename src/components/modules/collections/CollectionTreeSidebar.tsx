import React, { useState } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MethodBadge } from '../../common/MethodBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Input, Button } from '../../common/ui';
import type { CollectionTreeItem, RequestItem } from '../../../services/tauri/bridge';

interface CollectionTreeSidebarProps {
  widthPx?: number;
}

export const CollectionTreeSidebar: React.FC<CollectionTreeSidebarProps> = ({ widthPx = 280 }) => {
  const { activeWorkspaceId } = useWorkspaceStore();
  const {
    collectionsTree,
    createNewCollection,
    createNewRequest,
    deleteCollectionById,
    deleteRequestById,
    openRequestTab,
    searchQuery,
    setSearchQuery,
  } = useCollectionStore();

  const [expandedFolders, setExpandedFolders] = useState<Record<string, boolean>>({});
  const [addingFolderParentId, setAddingFolderParentId] = useState<string | null | 'root'>(null);
  const [folderName, setFolderName] = useState('');
  const [addingReqFolderId, setAddingReqFolderId] = useState<string | null>(null);
  const [reqName, setReqName] = useState('');

  const toggleFolder = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setExpandedFolders((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const handleAddFolder = async (parentId: string | null) => {
    if (!folderName.trim() || !activeWorkspaceId) return;
    await createNewCollection(activeWorkspaceId, parentId, folderName.trim());
    setFolderName('');
    setAddingFolderParentId(null);
    if (parentId) {
      setExpandedFolders((prev) => ({ ...prev, [parentId]: true }));
    }
  };

  const handleAddRequest = async (colId: string) => {
    if (!reqName.trim()) return;
    await createNewRequest(colId, reqName.trim());
    setReqName('');
    setAddingReqFolderId(null);
    setExpandedFolders((prev) => ({ ...prev, [colId]: true }));
  };

  const renderTreeItem = (item: CollectionTreeItem, depth: number = 0) => {
    const isExpanded = expandedFolders[item.id] !== false; // Default expanded
    const isAddingSubfolder = addingFolderParentId === item.id;
    const isAddingSubReq = addingReqFolderId === item.id;

    return (
      <div key={item.id} className="flex flex-col select-none text-xs">
        {/* Folder Node Header */}
        <div
          onClick={(e) => toggleFolder(item.id, e)}
          className="group flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-neutral-subtle/80 cursor-pointer font-sans transition-colors"
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
        >
          <div className="flex items-center gap-1.5 min-w-0 font-medium text-foreground">
            <MingCuteIcon
              name={isExpanded ? 'folder_open_line' : 'folder_line'}
              size={15}
              className="text-amber-500 shrink-0"
            />
            <span className="truncate">{item.name}</span>
          </div>

          {/* Actions: Add Subfolder / Add Request / Delete */}
          <div className="opacity-0 group-hover:opacity-100 flex items-center gap-1 shrink-0 transition-opacity">
            <button
              onClick={(e) => {
                e.stopPropagation();
                setAddingFolderParentId(item.id);
                setFolderName('');
              }}
              className="text-muted-foreground hover:text-primary p-0.5 rounded transition-colors"
              title="Add Subfolder"
            >
              <MingCuteIcon name="folder_add_line" size={13} />
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                setAddingReqFolderId(item.id);
                setReqName('');
              }}
              className="text-muted-foreground hover:text-primary p-0.5 rounded transition-colors"
              title="Add Request"
            >
              <MingCuteIcon name="plus_line" size={13} />
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                deleteCollectionById(item.id);
              }}
              className="text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-colors"
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
              <div className="px-2 py-1 flex items-center gap-1" style={{ paddingLeft: `${(depth + 1) * 12 + 8}px` }}>
                <input
                  type="text"
                  autoFocus
                  placeholder="Subfolder name..."
                  value={folderName}
                  onChange={(e) => setFolderName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleAddFolder(item.id);
                    if (e.key === 'Escape') setAddingFolderParentId(null);
                  }}
                  className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
                />
                <button
                  type="button"
                  onClick={() => handleAddFolder(item.id)}
                  className="p-1 text-emerald-400 hover:bg-emerald-500/10 rounded cursor-pointer shrink-0"
                  title="Save Subfolder"
                >
                  <MingCuteIcon name="check_line" size={13} />
                </button>
                <button
                  type="button"
                  onClick={() => setAddingFolderParentId(null)}
                  className="p-1 text-muted-foreground hover:text-rose-400 hover:bg-rose-500/10 rounded cursor-pointer shrink-0"
                  title="Cancel"
                >
                  <MingCuteIcon name="close_line" size={13} />
                </button>
              </div>
            )}

            {/* Inline Sub-Request Input */}
            {isAddingSubReq && (
              <div className="px-2 py-1 flex items-center gap-1" style={{ paddingLeft: `${(depth + 1) * 12 + 8}px` }}>
                <input
                  type="text"
                  autoFocus
                  placeholder="Request name..."
                  value={reqName}
                  onChange={(e) => setReqName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleAddRequest(item.id);
                    if (e.key === 'Escape') setAddingReqFolderId(null);
                  }}
                  className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
                />
                <button
                  type="button"
                  onClick={() => handleAddRequest(item.id)}
                  className="p-1 text-emerald-400 hover:bg-emerald-500/10 rounded cursor-pointer shrink-0"
                  title="Save Request"
                >
                  <MingCuteIcon name="check_line" size={13} />
                </button>
                <button
                  type="button"
                  onClick={() => setAddingReqFolderId(null)}
                  className="p-1 text-muted-foreground hover:text-rose-400 hover:bg-rose-500/10 rounded cursor-pointer shrink-0"
                  title="Cancel"
                >
                  <MingCuteIcon name="close_line" size={13} />
                </button>
              </div>
            )}

            {/* Subfolders */}
            {item.children?.map((child) => renderTreeItem(child, depth + 1))}

            {/* Folder Requests */}
            {item.requests?.map((req) => renderRequestNode(req, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  const renderRequestNode = (req: RequestItem, depth: number) => {
    return (
      <div
        key={req.id}
        onClick={() => openRequestTab(req)}
        className="group flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-neutral-subtle/80 cursor-pointer font-sans text-xs transition-colors"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        <div className="flex items-center gap-2 truncate min-w-0">
          <MethodBadge method={req.method} />
          <span className="truncate text-foreground font-medium">{req.name}</span>
        </div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            deleteRequestById(req.id);
          }}
          className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-opacity"
          title="Delete Request"
        >
          <MingCuteIcon name="close_line" size={13} />
        </button>
      </div>
    );
  };

  return (
    <div
      className="bg-surface border-r border-border h-full flex flex-col overflow-hidden text-xs shrink-0 select-none"
      style={{ width: `${widthPx}px` }}
    >
      {/* Top Header */}
      <div className="p-2.5 bg-header border-b border-border flex flex-col gap-2 shrink-0">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-foreground flex items-center gap-1.5">
            <MingCuteIcon name="folder_line" size={16} className="text-amber-500" />
            <span>Collections</span>
          </span>

          <Button
            variant="primary"
            sizeVariant="xs"
            icon="plus_line"
            onClick={() => {
              setAddingFolderParentId('root');
              setFolderName('');
            }}
            title="Create Root Collection Folder"
          >
            + Folder
          </Button>
        </div>

        <Input
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search collections..."
          leftIcon="search_line"
          rightIcon={searchQuery ? 'close_line' : undefined}
          onRightIconClick={() => setSearchQuery('')}
        />
      </div>

      {/* Root Collection Tree */}
      <div className="flex-1 p-2 overflow-y-auto space-y-1 no-scrollbar">
        {addingFolderParentId === 'root' && (
          <div className="p-1 flex items-center gap-1">
            <input
              type="text"
              autoFocus
              placeholder="Root Folder Name..."
              value={folderName}
              onChange={(e) => setFolderName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddFolder(null);
                if (e.key === 'Escape') setAddingFolderParentId(null);
              }}
              className="bg-background border border-primary rounded px-2 py-1 text-xs text-foreground font-sans w-full focus:outline-none"
            />
            <button
              type="button"
              onClick={() => handleAddFolder(null)}
              className="p-1 text-emerald-400 hover:bg-emerald-500/10 rounded cursor-pointer shrink-0"
              title="Save Root Folder"
            >
              <MingCuteIcon name="check_line" size={13} />
            </button>
            <button
              type="button"
              onClick={() => setAddingFolderParentId(null)}
              className="p-1 text-muted-foreground hover:text-rose-400 hover:bg-rose-500/10 rounded cursor-pointer shrink-0"
              title="Cancel"
            >
              <MingCuteIcon name="close_line" size={13} />
            </button>
          </div>
        )}

        {collectionsTree.length === 0 && addingFolderParentId !== 'root' ? (
          <div className="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2">
            <MingCuteIcon name="folder_open_line" size={32} className="opacity-30" />
            <span>No collections created yet</span>
            <Button
              variant="outline"
              sizeVariant="xs"
              icon="plus_line"
              onClick={() => {
                setAddingFolderParentId('root');
                setFolderName('');
              }}
            >
              Create Root Folder
            </Button>
          </div>
        ) : (
          collectionsTree.map((item) => renderTreeItem(item, 0))
        )}
      </div>
    </div>
  );
};
