import React, { useState, useEffect } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MethodBadge } from '../../common/MethodBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Input, Button } from '../../common/ui';
import { CollectionExtractRulesModal } from './CollectionExtractRulesModal';
import type { CollectionTreeItem, RequestItem } from '../../../services/tauri/bridge';

interface CollectionTreeSidebarProps {
  widthPx?: number;
}

interface ContextMenuState {
  x: number;
  y: number;
  type: 'collection' | 'request' | 'root';
  id?: string;
  item?: any;
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

  // Recursive tree filter supporting name, URL, method, and description
  const filterTree = (items: CollectionTreeItem[], query: string): CollectionTreeItem[] => {
    const q = query.trim().toLowerCase();
    if (!q) return items;

    const terms = q.split(/\s+/).filter(Boolean);

    const matchesAllTerms = (text: string) => {
      const lower = text.toLowerCase();
      return terms.every((t) => lower.includes(t));
    };

    return items
      .map((item) => {
        const matchingChildren = filterTree(item.children || [], query);
        const matchingRequests = (item.requests || []).filter((r) => {
          const combined = `${r.name} ${r.url || ''} ${r.method || ''} ${r.description || ''}`;
          return matchesAllTerms(combined);
        });

        const folderCombined = `${item.name} ${item.description || ''}`;
        const folderMatches = matchesAllTerms(folderCombined);

        if (folderMatches) {
          return item;
        }

        if (matchingChildren.length > 0 || matchingRequests.length > 0) {
          return {
            ...item,
            children: matchingChildren,
            requests: matchingRequests,
          };
        }

        return null;
      })
      .filter((item): item is CollectionTreeItem => item !== null);
  };

  const isSearching = !!debouncedQuery.trim();
  const visibleTree = filterTree(collectionsTree, debouncedQuery);

  // Close context menu on outside click
  useEffect(() => {
    const handleOutsideClick = () => setContextMenu(null);
    window.addEventListener('click', handleOutsideClick);
    window.addEventListener('contextmenu', handleOutsideClick);
    return () => {
      window.removeEventListener('click', handleOutsideClick);
      window.removeEventListener('contextmenu', handleOutsideClick);
    };
  }, []);

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

  const renderTreeItem = (item: CollectionTreeItem, depth: number = 0) => {
    const isExpanded = isSearching ? true : expandedFolders[item.id] !== false; // Auto-expand when searching, default expanded otherwise
    const isAddingSubfolder = addingFolderParentId === item.id;
    const isAddingSubReq = addingReqFolderId === item.id;
    const isDropTarget = dragTargetFolderId === item.id;
    const isRenaming = editingId === item.id && editingType === 'collection';

    return (
      <div key={item.id} className="flex flex-col select-none text-xs">
        {/* Folder Node Header */}
        <div
          draggable
          onDragStart={(e) => handleDragStartFolder(e, item)}
          onDragOver={(e) => handleDragOverFolder(e, item.id)}
          onDragLeave={(e) => handleDragLeaveFolder(e, item.id)}
          onDrop={(e) => handleDropOnFolder(e, item.id)}
          onClick={(e) => toggleFolder(item.id, e)}
          onContextMenu={(e) => handleContextMenu(e, 'collection', item.id, item)}
          className={`group flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-neutral-subtle/80 cursor-pointer font-sans transition-all ${
            isDropTarget ? 'ring-2 ring-primary bg-primary/10' : ''
          }`}
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
        >
          <div className="flex items-center gap-1.5 min-w-0 font-medium text-foreground flex-1">
            <MingCuteIcon
              name={isExpanded ? 'folder_open_line' : 'folder_line'}
              size={15}
              className="text-amber-500 shrink-0"
            />
            {isRenaming ? (
              <input
                type="text"
                autoFocus
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleSaveRename();
                  if (e.key === 'Escape') setEditingId(null);
                }}
                onBlur={handleSaveRename}
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
                if (activeWorkspaceId) deleteCollectionById(item.id, activeWorkspaceId);
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
    const isRenaming = editingId === req.id && editingType === 'request';

    return (
      <div
        key={req.id}
        draggable
        onDragStart={(e) => handleDragStartRequest(e, req)}
        onClick={() => openRequestTab(req)}
        onContextMenu={(e) => handleContextMenu(e, 'request', req.id, req)}
        className="group flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-neutral-subtle/80 cursor-pointer font-sans text-xs transition-colors"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        <div className="flex items-center gap-2 truncate min-w-0 flex-1">
          <MethodBadge method={req.method} />
          {isRenaming ? (
            <input
              type="text"
              autoFocus
              value={editingName}
              onChange={(e) => setEditingName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleSaveRename();
                if (e.key === 'Escape') setEditingId(null);
              }}
              onBlur={handleSaveRename}
              onClick={(e) => e.stopPropagation()}
              className="bg-background border border-primary rounded px-1 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
            />
          ) : (
            <div className="flex flex-col min-w-0 truncate">
              <span className="truncate text-foreground font-medium">{req.name}</span>
              {isSearching && req.url && (
                <span className="truncate text-[10px] text-muted-foreground font-mono" title={req.url}>
                  {req.url}
                </span>
              )}
            </div>
          )}
        </div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            if (activeWorkspaceId) deleteRequestById(req.id, activeWorkspaceId);
          }}
          className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-opacity shrink-0"
          title="Delete Request"
        >
          <MingCuteIcon name="close_line" size={13} />
        </button>
      </div>
    );
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
          visibleTree.map((item) => renderTreeItem(item, 0))
        )}
      </div>

      {/* Floating Right-Click Context Menu */}
      {contextMenu && (
        <div
          style={{ top: `${contextMenu.y}px`, left: `${contextMenu.x}px` }}
          onClick={(e) => e.stopPropagation()}
          className="fixed z-50 bg-surface border border-border rounded-lg shadow-xl py-1 text-xs min-w-[170px] text-foreground font-sans flex flex-col"
        >
          {contextMenu.type === 'collection' && (
            <>
              <button
                onClick={() => {
                  setAddingReqFolderId(contextMenu.id!);
                  setReqName('');
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="plus_line" size={14} className="text-primary" />
                <span>Add Request</span>
              </button>

              <button
                onClick={() => {
                  setAddingFolderParentId(contextMenu.id!);
                  setFolderName('');
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="folder_add_line" size={14} className="text-amber-500" />
                <span>Add Subfolder</span>
              </button>

              <button
                onClick={() => {
                  setExtractModalCol(contextMenu.item);
                  setExtractModalReq(null);
                  setIsExtractModalOpen(true);
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="lightning_line" size={14} className="text-amber-500" />
                <span>Auto-Extract Rules</span>
              </button>

              <button
                onClick={() => {
                  if (contextMenu.id && activeWorkspaceId) {
                    duplicateCollectionItem(contextMenu.id, activeWorkspaceId);
                  }
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="copy_line" size={14} className="text-muted-foreground" />
                <span>Duplicate Folder</span>
              </button>

              <button
                onClick={() => {
                  setEditingId(contextMenu.id!);
                  setEditingType('collection');
                  setEditingName(contextMenu.item?.name || '');
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="edit_line" size={14} className="text-muted-foreground" />
                <span>Rename Folder</span>
              </button>

              <div className="my-1 border-t border-border" />

              <button
                onClick={() => {
                  if (contextMenu.id && activeWorkspaceId) {
                    deleteCollectionById(contextMenu.id, activeWorkspaceId);
                  }
                  setContextMenu(null);
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
                onClick={() => {
                  openRequestTab(contextMenu.item);
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="eye_line" size={14} className="text-primary" />
                <span>Open in Tab</span>
              </button>

              <button
                onClick={() => {
                  setExtractModalCol(null);
                  setExtractModalReq(contextMenu.item);
                  setIsExtractModalOpen(true);
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="lightning_line" size={14} className="text-amber-500" />
                <span>Auto-Extract Rules</span>
              </button>

              <button
                onClick={() => {
                  if (contextMenu.id && activeWorkspaceId) {
                    duplicateRequestItem(contextMenu.id, activeWorkspaceId);
                  }
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="copy_line" size={14} className="text-muted-foreground" />
                <span>Duplicate Request</span>
              </button>

              <button
                onClick={() => {
                  setEditingId(contextMenu.id!);
                  setEditingType('request');
                  setEditingName(contextMenu.item?.name || '');
                  setContextMenu(null);
                }}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="edit_line" size={14} className="text-muted-foreground" />
                <span>Rename Request</span>
              </button>

              <div className="my-1 border-t border-border" />

              <button
                onClick={() => {
                  if (contextMenu.id && activeWorkspaceId) {
                    deleteRequestById(contextMenu.id, activeWorkspaceId);
                  }
                  setContextMenu(null);
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
              onClick={() => {
                setAddingFolderParentId('root');
                setFolderName('');
                setContextMenu(null);
              }}
              className="flex items-center gap-2 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
            >
              <MingCuteIcon name="folder_add_line" size={14} className="text-amber-500" />
              <span>New Root Folder</span>
            </button>
          )}
        </div>
      )}

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
