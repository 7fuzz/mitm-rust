import React, { useState, useEffect } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Input, Button } from '../../common/ui';
import { CollectionExtractRulesModal } from './CollectionExtractRulesModal';
import { CollectionFolderNode } from './tree/CollectionFolderNode';
import { CollectionContextMenu, type ContextMenuState } from './tree/CollectionContextMenu';
import { filterTree } from './tree/collectionTreeFilter';
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
    moveCollectionItem,
    moveRequestItem,
    duplicateCollectionItem,
    duplicateRequestItem,
    updateCollectionDetails,
    updateRequestDetails,
    openRequestTab,
    searchQuery,
    setSearchQuery,
  } = useCollectionStore();

  const [localSearch, setLocalSearch] = useState(searchQuery);
  const [debouncedQuery, setDebouncedQuery] = useState(searchQuery);

  // Extract Rules Modal state
  const [isExtractModalOpen, setIsExtractModalOpen] = useState(false);
  const [extractModalCol, setExtractModalCol] = useState<CollectionTreeItem | null>(null);
  const [extractModalReq, setExtractModalReq] = useState<RequestItem | null>(null);

  // Drag over root indicator
  const [isDragOverRoot, setIsDragOverRoot] = useState(false);

  // Debounce search query updates by 250ms
  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedQuery(localSearch);
      setSearchQuery(localSearch);
    }, 250);
    return () => clearTimeout(handler);
  }, [localSearch, setSearchQuery]);

  // Keep local search in sync if external store changes
  useEffect(() => {
    setLocalSearch(searchQuery);
    setDebouncedQuery(searchQuery);
  }, [searchQuery]);

  const [expandedFolders, setExpandedFolders] = useState<Record<string, boolean>>({});
  const [addingFolderParentId, setAddingFolderParentId] = useState<string | null | 'root'>(null);
  const [folderName, setFolderName] = useState('');
  const [addingReqFolderId, setAddingReqFolderId] = useState<string | null>(null);
  const [reqName, setReqName] = useState('');

  // Context Menu state
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  // Drag & Drop target highlight state
  const [dragTargetFolderId, setDragTargetFolderId] = useState<string | null>(null);

  // Inline Rename state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingType, setEditingType] = useState<'collection' | 'request' | null>(null);
  const [editingName, setEditingName] = useState('');

  const isSearching = !!debouncedQuery.trim();
  const visibleTree = filterTree(collectionsTree, debouncedQuery);

  // Close context menu on outside click
  useEffect(() => {
    const handleOutsideClick = () => setContextMenu(null);
    window.addEventListener('click', handleOutsideClick);
    return () => window.removeEventListener('click', handleOutsideClick);
  }, []);

  const toggleFolder = (id: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setExpandedFolders((prev) => ({
      ...prev,
      [id]: prev[id] === undefined ? false : !prev[id],
    }));
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

  // Inline Rename submit
  const handleSaveRename = async () => {
    if (!editingId || !editingName.trim()) {
      setEditingId(null);
      setEditingType(null);
      return;
    }

    if (editingType === 'collection') {
      await updateCollectionDetails({
        id: editingId,
        workspaceId: activeWorkspaceId || '',
        name: editingName.trim(),
        orderIndex: 0,
        createdAtMs: Date.now(),
        updatedAtMs: Date.now(),
      });
    } else if (editingType === 'request') {
      const targetReq = collectionsTree
        .flatMap((c) => [c, ...(c.children || [])])
        .flatMap((c) => c.requests || [])
        .find((r) => r.id === editingId);

      if (targetReq) {
        await updateRequestDetails({
          ...targetReq,
          name: editingName.trim(),
        });
      }
    }

    setEditingId(null);
    setEditingType(null);
  };

  // Context Menu Trigger
  const handleContextMenu = (
    e: React.MouseEvent,
    type: 'collection' | 'request' | 'root',
    id?: string,
    item?: any
  ) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      type,
      id,
      item,
    });
  };

  // Drag Handlers
  const handleDragStartRequest = (e: React.DragEvent, req: RequestItem) => {
    e.stopPropagation();
    e.dataTransfer.setData('application/json', JSON.stringify({ type: 'request', id: req.id, collectionId: req.collectionId }));
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragStartFolder = (e: React.DragEvent, item: CollectionTreeItem) => {
    e.stopPropagation();
    e.dataTransfer.setData('application/json', JSON.stringify({ type: 'collection', id: item.id, parentId: item.parentId }));
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOverFolder = (e: React.DragEvent, targetFolderId: string) => {
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = 'move';
    if (dragTargetFolderId !== targetFolderId) {
      setDragTargetFolderId(targetFolderId);
    }
  };

  const handleDragLeaveFolder = (e: React.DragEvent, targetFolderId: string) => {
    e.preventDefault();
    e.stopPropagation();
    if (dragTargetFolderId === targetFolderId) {
      setDragTargetFolderId(null);
    }
  };

  const handleDropOnFolder = async (e: React.DragEvent, targetFolderId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setDragTargetFolderId(null);

    const rawData = e.dataTransfer.getData('application/json');
    if (!rawData || !activeWorkspaceId) return;

    try {
      const payload = JSON.parse(rawData);
      if (payload.type === 'request' && payload.id) {
        if (payload.collectionId !== targetFolderId) {
          await moveRequestItem(payload.id, targetFolderId, activeWorkspaceId);
          setExpandedFolders((prev) => ({ ...prev, [targetFolderId]: true }));
        }
      } else if (payload.type === 'collection' && payload.id) {
        if (payload.id !== targetFolderId) {
          await moveCollectionItem(payload.id, targetFolderId, activeWorkspaceId);
          setExpandedFolders((prev) => ({ ...prev, [targetFolderId]: true }));
        }
      }
    } catch (err) {
      console.error('Failed to parse drag drop payload:', err);
    }
  };

  const handleDragOverRoot = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = 'move';
    if (!isDragOverRoot) {
      setIsDragOverRoot(true);
    }
  };

  const handleDragLeaveRoot = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (isDragOverRoot) {
      setIsDragOverRoot(false);
    }
  };

  const handleDropOnRoot = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOverRoot(false);
    setDragTargetFolderId(null);

    const rawData = e.dataTransfer.getData('application/json');
    if (!rawData || !activeWorkspaceId) return;

    try {
      const payload = JSON.parse(rawData);
      if (payload.type === 'collection' && payload.id) {
        // Move collection folder to root
        await moveCollectionItem(payload.id, null, activeWorkspaceId);
      } else if (payload.type === 'request' && payload.id) {
        // Move request to root / unassigned folder
        let targetColId = collectionsTree.find((c) => !c.parentId)?.id;
        if (!targetColId) {
          const created = await createNewCollection(activeWorkspaceId, null, 'Unassigned Requests');
          targetColId = created?.id;
        }
        if (targetColId && targetColId !== payload.collectionId) {
          await moveRequestItem(payload.id, targetColId, activeWorkspaceId);
          setExpandedFolders((prev) => ({ ...prev, [targetColId]: true }));
        }
      }
    } catch (err) {
      console.error('Failed to drop on root:', err);
    }
  };

  return (
    <div
      onContextMenu={(e) => handleContextMenu(e, 'root')}
      className="bg-surface border-r border-border h-full flex flex-col overflow-hidden text-xs shrink-0 select-none relative"
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
          value={localSearch}
          onChange={(e) => setLocalSearch(e.target.value)}
          placeholder="Search collections & requests..."
          leftIcon="search_line"
          rightIcon={localSearch ? 'close_line' : undefined}
          onRightIconClick={() => {
            setLocalSearch('');
            setDebouncedQuery('');
            setSearchQuery('');
          }}
        />
      </div>

      {/* Root Collection Tree */}
      <div
        onDragOver={handleDragOverRoot}
        onDragLeave={handleDragLeaveRoot}
        onDrop={handleDropOnRoot}
        className={`flex-1 p-2 overflow-y-auto space-y-1 no-scrollbar transition-colors ${
          isDragOverRoot ? 'bg-primary/10 ring-2 ring-dashed ring-primary/60 rounded-lg m-1' : ''
        }`}
      >
        {isDragOverRoot && (
          <div className="p-2 mb-2 rounded border border-dashed border-primary bg-primary/20 text-center text-xs text-primary font-semibold flex items-center justify-center gap-1.5 animate-pulse">
            <MingCuteIcon name="folder_line" size={14} />
            <span>Drop here to move to Root Level / Unassigned</span>
          </div>
        )}

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

        {visibleTree.length === 0 && isSearching ? (
          <div className="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2">
            <MingCuteIcon name="search_line" size={32} className="opacity-30" />
            <span>No collections or requests match &ldquo;{debouncedQuery}&rdquo;</span>
            <button
              type="button"
              onClick={() => {
                setLocalSearch('');
                setDebouncedQuery('');
                setSearchQuery('');
              }}
              className="text-xs text-primary hover:underline cursor-pointer"
            >
              Clear search
            </button>
          </div>
        ) : collectionsTree.length === 0 && addingFolderParentId !== 'root' ? (
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
          visibleTree.map((item) => (
            <CollectionFolderNode
              key={item.id}
              item={item}
              depth={0}
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
              onSetFolderName={setFolderName}
              onSetReqName={setReqName}
              onSetEditingName={setEditingName}
              onSetAddingFolderParentId={setAddingFolderParentId}
              onSetAddingReqFolderId={setAddingReqFolderId}
              onToggleFolder={toggleFolder}
              onAddFolder={handleAddFolder}
              onAddRequest={handleAddRequest}
              onSaveRename={handleSaveRename}
              onCancelRename={() => setEditingId(null)}
              onContextMenu={handleContextMenu}
              onDragStartFolder={handleDragStartFolder}
              onDragStartRequest={handleDragStartRequest}
              onDragOverFolder={handleDragOverFolder}
              onDragLeaveFolder={handleDragLeaveFolder}
              onDropOnFolder={handleDropOnFolder}
              onOpenRequestTab={openRequestTab}
              onDeleteCollection={(id) => {
                if (activeWorkspaceId) deleteCollectionById(id, activeWorkspaceId);
              }}
              onDeleteRequest={(id) => {
                if (activeWorkspaceId) deleteRequestById(id, activeWorkspaceId);
              }}
            />
          ))
        )}
      </div>

      {/* Floating Right-Click Context Menu */}
      <CollectionContextMenu
        contextMenu={contextMenu}
        activeWorkspaceId={activeWorkspaceId}
        onClose={() => setContextMenu(null)}
        onAddRequest={(colId) => {
          setAddingReqFolderId(colId);
          setReqName('');
        }}
        onAddSubfolder={(colId) => {
          setAddingFolderParentId(colId);
          setFolderName('');
        }}
        onOpenAutoExtract={(type, item) => {
          if (type === 'collection') {
            setExtractModalCol(item);
            setExtractModalReq(null);
          } else {
            setExtractModalCol(null);
            setExtractModalReq(item);
          }
          setIsExtractModalOpen(true);
        }}
        onDuplicateFolder={(colId) => {
          if (activeWorkspaceId) duplicateCollectionItem(colId, activeWorkspaceId);
        }}
        onRenameFolder={(colId, name) => {
          setEditingId(colId);
          setEditingType('collection');
          setEditingName(name);
        }}
        onDeleteFolder={(colId) => {
          if (activeWorkspaceId) deleteCollectionById(colId, activeWorkspaceId);
        }}
        onOpenRequestTab={openRequestTab}
        onDuplicateRequest={(reqId) => {
          if (activeWorkspaceId) duplicateRequestItem(reqId, activeWorkspaceId);
        }}
        onRenameRequest={(reqId, name) => {
          setEditingId(reqId);
          setEditingType('request');
          setEditingName(name);
        }}
        onDeleteRequest={(reqId) => {
          if (activeWorkspaceId) deleteRequestById(reqId, activeWorkspaceId);
        }}
        onAddRootFolder={() => {
          setAddingFolderParentId('root');
          setFolderName('');
        }}
      />

      {/* Auto-Extract Rules Modal */}
      <CollectionExtractRulesModal
        isOpen={isExtractModalOpen}
        onClose={() => {
          setIsExtractModalOpen(false);
          setExtractModalCol(null);
          setExtractModalReq(null);
        }}
        collection={extractModalCol}
        request={extractModalReq}
      />
    </div>
  );
};
