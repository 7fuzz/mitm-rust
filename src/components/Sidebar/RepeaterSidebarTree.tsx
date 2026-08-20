import React, { useState, useMemo, useEffect, useRef } from 'react';
import { RepeaterGroup, RepeaterRequest } from '@/hooks/traffic';
import { DebouncedInput } from '../ui/DebouncedInput';

interface RepeaterSidebarTreeProps {
  repeaterGroups: RepeaterGroup[];
  repeaterRequests: RepeaterRequest[];
  activeId: string | null;
  onSelectRequest: (id: string) => void;
  onDeleteRequest: (id: string) => void;
  onCreateRequest: (groupId: string | null) => void;
  onCreateGroup: (name: string, parentId?: string | null) => void;
  onRenameGroup: (group: RepeaterGroup) => void;
  onDeleteGroup: (group: RepeaterGroup) => void;
  onOpenDocModal: (group: RepeaterGroup) => void;
  onOpenExtractionModal: (group: RepeaterGroup) => void;
  openPrompt: (title: string, initialValue: string, action: (val: string) => void) => void;
}

const getMethodColor = (m: string) => {
  if (m === 'GET') return 'text-sky-400 bg-sky-500/10 border-sky-500/30';
  if (m === 'POST') return 'text-emerald-400 bg-emerald-500/10 border-emerald-500/30';
  if (m === 'DELETE') return 'text-rose-400 bg-rose-500/10 border-rose-500/30';
  if (m === 'PUT' || m === 'PATCH') return 'text-amber-400 bg-amber-500/10 border-amber-500/30';
  return 'text-purple-400 bg-purple-500/10 border-purple-500/30';
};

interface ContextMenuState {
  x: number;
  y: number;
  type: 'folder' | 'request' | 'uncategorized';
  targetFolder?: RepeaterGroup;
  targetRequest?: RepeaterRequest;
}

interface TreeNodeProps {
  group: RepeaterGroup;
  level: number;
  repeaterGroups: RepeaterGroup[];
  repeaterRequests: RepeaterRequest[];
  activeId: string | null;
  expandedMap: Record<string, boolean>;
  toggleExpand: (id: string) => void;
  onSelectRequest: (id: string) => void;
  onContextMenuFolder: (e: React.MouseEvent, group: RepeaterGroup) => void;
  onContextMenuRequest: (e: React.MouseEvent, request: RepeaterRequest) => void;
}

function FolderTreeNode({
  group,
  level,
  repeaterGroups,
  repeaterRequests,
  activeId,
  expandedMap,
  toggleExpand,
  onSelectRequest,
  onContextMenuFolder,
  onContextMenuRequest,
}: TreeNodeProps) {
  const isExpanded = expandedMap[group.id] ?? false;

  const childGroups = useMemo(
    () => repeaterGroups.filter((g) => g.parentId === group.id),
    [repeaterGroups, group.id]
  );
  const childRequests = useMemo(
    () => repeaterRequests.filter((r) => r.groupId === group.id),
    [repeaterRequests, group.id]
  );

  const totalItemsCount = childGroups.length + childRequests.length;

  return (
    <div className="select-none">
      {/* Folder Header Row */}
      <div
        style={{ paddingLeft: `${level * 12 + 8}px` }}
        className="group/folder flex items-center justify-between py-1.5 pr-2 hover:bg-zinc-900/80 rounded cursor-pointer transition-colors text-zinc-300"
        onClick={() => toggleExpand(group.id)}
        onContextMenu={(e) => onContextMenuFolder(e, group)}
      >
        <div className="flex items-center gap-1.5 min-w-0 flex-1">
          <span className="text-[10px] text-zinc-500 w-4 h-4 flex items-center justify-center shrink-0">
            {isExpanded ? '▼' : '▶'}
          </span>
          <span className="text-purple-400 shrink-0 text-xs">📁</span>
          <span className="text-xs font-bold truncate text-zinc-200">{group.name}</span>
          <span className="text-[9px] text-zinc-500 font-mono shrink-0 bg-zinc-950 px-1 py-0.5 rounded border border-zinc-800">
            {totalItemsCount}
          </span>
          {group.description && (
            <span className="text-[8px] bg-purple-500/20 text-purple-300 px-1 rounded border border-purple-500/30 font-mono shrink-0">
              Docs
            </span>
          )}
        </div>
      </div>

      {/* Expanded Children */}
      {isExpanded && (
        <div className="space-y-0.5">
          {/* Sub-Folders */}
          {childGroups.map((cg) => (
            <FolderTreeNode
              key={cg.id}
              group={cg}
              level={level + 1}
              repeaterGroups={repeaterGroups}
              repeaterRequests={repeaterRequests}
              activeId={activeId}
              expandedMap={expandedMap}
              toggleExpand={toggleExpand}
              onSelectRequest={onSelectRequest}
              onContextMenuFolder={onContextMenuFolder}
              onContextMenuRequest={onContextMenuRequest}
            />
          ))}

          {/* Child Requests */}
          {childRequests.map((req) => {
            const isActive = activeId === req.id;
            return (
              <div
                key={req.id}
                style={{ paddingLeft: `${(level + 1) * 12 + 16}px` }}
                onClick={() => onSelectRequest(req.id)}
                onContextMenu={(e) => onContextMenuRequest(e, req)}
                className={`group/req flex items-center justify-between py-1.5 pr-2 rounded cursor-pointer transition-all border-l-2 ${
                  isActive
                    ? 'bg-purple-500/10 border-l-purple-500 text-purple-300 font-semibold'
                    : 'bg-transparent border-l-transparent hover:bg-zinc-900/60 hover:border-l-zinc-700 text-zinc-300'
                }`}
              >
                <div className="flex items-center gap-2 min-w-0 flex-1">
                  <span
                    className={`text-[9px] font-black uppercase tracking-wider px-1 py-0.5 rounded border shrink-0 ${getMethodColor(
                      req.method
                    )}`}
                  >
                    {req.method}
                  </span>
                  <span className="text-xs truncate">{req.name}</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

const STORAGE_KEY = 'repeater_sidebar_tree_expanded_map';

export function RepeaterSidebarTree({
  repeaterGroups,
  repeaterRequests,
  activeId,
  onSelectRequest,
  onDeleteRequest,
  onCreateRequest,
  onCreateGroup,
  onRenameGroup,
  onDeleteGroup,
  onOpenDocModal,
  onOpenExtractionModal,
  openPrompt,
}: RepeaterSidebarTreeProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [expandedMap, setExpandedMap] = useState<Record<string, boolean>>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      return saved ? JSON.parse(saved) : {};
    } catch (e) {
      return {};
    }
  });
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(expandedMap));
    } catch (e) {
      console.error('Failed to save tree expansion state:', e);
    }
  }, [expandedMap]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setContextMenu(null);
    };
    window.addEventListener('mousedown', handleClickOutside);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('mousedown', handleClickOutside);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  const toggleExpand = (id: string) => {
    setExpandedMap((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const rootGroups = useMemo(() => {
    return repeaterGroups.filter((g) => !g.parentId);
  }, [repeaterGroups]);

  const uncategorizedRequests = useMemo(() => {
    return repeaterRequests.filter((r) => !r.groupId);
  }, [repeaterRequests]);

  const filteredRequests = useMemo(() => {
    if (!searchTerm.trim()) return null;
    const term = searchTerm.toLowerCase();
    return repeaterRequests.filter(
      (r) => r.name.toLowerCase().includes(term) || r.url.toLowerCase().includes(term)
    );
  }, [repeaterRequests, searchTerm]);

  const handleContextMenuFolder = (e: React.MouseEvent, group: RepeaterGroup) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      type: 'folder',
      targetFolder: group,
    });
  };

  const handleContextMenuRequest = (e: React.MouseEvent, request: RepeaterRequest) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      type: 'request',
      targetRequest: request,
    });
  };

  const handleContextMenuUncategorized = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      type: 'uncategorized',
    });
  };

  return (
    <div className="flex flex-col h-full bg-zinc-950 border-r border-zinc-800/80 text-zinc-300 select-none relative">
      {/* Sidebar Header */}
      <div className="p-3 border-b border-zinc-800/80 space-y-2 shrink-0">
        <div className="flex items-center justify-between">
          <span className="text-[10px] font-black uppercase tracking-widest text-purple-400 flex items-center gap-1.5">
            <span>🌿</span> Collections & Requests
          </span>
          <button
            onClick={() =>
              openPrompt('New Collection Name', '', (val) => onCreateGroup(val, null))
            }
            className="px-2 py-1 bg-purple-600/10 border border-purple-600/30 text-purple-400 hover:bg-purple-600/20 rounded text-[9px] font-black uppercase tracking-widest transition-all"
            title="Create top-level root collection"
          >
            + Collection
          </button>
        </div>

        {/* Search Input */}
        <DebouncedInput
          value={searchTerm}
          onChange={setSearchTerm}
          placeholder="Filter collections & requests..."
          className="w-full text-xs !bg-zinc-900 border-zinc-800"
        />
      </div>

      {/* Tree / List Content */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1 scrollbar-thin">
        {filteredRequests ? (
          /* Search Results Mode */
          <div className="space-y-1">
            <div className="text-[10px] uppercase font-bold text-zinc-500 px-2 py-1">
              Search Results ({filteredRequests.length})
            </div>
            {filteredRequests.length === 0 ? (
              <div className="text-xs text-zinc-600 px-2 py-4 italic">No matching requests found</div>
            ) : (
              filteredRequests.map((req) => (
                <div
                  key={req.id}
                  onClick={() => onSelectRequest(req.id)}
                  onContextMenu={(e) => handleContextMenuRequest(e, req)}
                  className={`flex items-center justify-between p-2 rounded cursor-pointer transition-all border-l-2 ${
                    activeId === req.id
                      ? 'bg-purple-500/10 border-l-purple-500 text-purple-300 font-semibold'
                      : 'bg-zinc-900/40 border-l-transparent hover:bg-zinc-900 text-zinc-300'
                  }`}
                >
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    <span
                      className={`text-[9px] font-black uppercase tracking-wider px-1.5 py-0.5 rounded border shrink-0 ${getMethodColor(
                        req.method
                      )}`}
                    >
                      {req.method}
                    </span>
                    <div className="flex flex-col min-w-0">
                      <span className="text-xs truncate font-medium">{req.name}</span>
                      <span className="text-[9px] text-zinc-500 truncate">{req.url}</span>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        ) : (
          /* Normal Tree View Mode */
          <>
            {/* Top-level (Uncategorized) Folder Node */}
            <div className="select-none">
              <div
                className="group/folder flex items-center justify-between py-1.5 px-2 hover:bg-zinc-900/80 rounded cursor-pointer transition-colors text-zinc-300"
                onClick={() => toggleExpand('uncategorized')}
                onContextMenu={handleContextMenuUncategorized}
              >
                <div className="flex items-center gap-1.5 min-w-0 flex-1">
                  <span className="text-[10px] text-zinc-500 w-4 h-4 flex items-center justify-center shrink-0">
                    {expandedMap['uncategorized'] ? '▼' : '▶'}
                  </span>
                  <span className="text-amber-400 shrink-0 text-xs">📦</span>
                  <span className="text-xs font-bold truncate text-zinc-300">(Uncategorized)</span>
                  <span className="text-[9px] text-zinc-500 font-mono shrink-0 bg-zinc-950 px-1 py-0.5 rounded border border-zinc-800">
                    {uncategorizedRequests.length}
                  </span>
                </div>
              </div>

              {/* Uncategorized Requests nested inside folder */}
              {expandedMap['uncategorized'] && (
                <div className="space-y-0.5">
                  {uncategorizedRequests.length === 0 ? (
                    <div className="text-[10px] text-zinc-600 pl-8 py-1 italic">No uncategorized requests</div>
                  ) : (
                    uncategorizedRequests.map((req) => {
                      const isActive = activeId === req.id;
                      return (
                        <div
                          key={req.id}
                          style={{ paddingLeft: '28px' }}
                          onClick={() => onSelectRequest(req.id)}
                          onContextMenu={(e) => handleContextMenuRequest(e, req)}
                          className={`group/req flex items-center justify-between py-1.5 pr-2 rounded cursor-pointer transition-all border-l-2 ${
                            isActive
                              ? 'bg-purple-500/10 border-l-purple-500 text-purple-300 font-semibold'
                              : 'bg-transparent border-l-transparent hover:bg-zinc-900/60 hover:border-l-zinc-700 text-zinc-300'
                          }`}
                        >
                          <div className="flex items-center gap-2 min-w-0 flex-1">
                            <span
                              className={`text-[9px] font-black uppercase tracking-wider px-1 py-0.5 rounded border shrink-0 ${getMethodColor(
                                req.method
                              )}`}
                            >
                              {req.method}
                            </span>
                            <span className="text-xs truncate font-medium">{req.name}</span>
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              )}
            </div>

            {/* Root Collections / Folders */}
            {rootGroups.map((group) => (
              <FolderTreeNode
                key={group.id}
                group={group}
                level={0}
                repeaterGroups={repeaterGroups}
                repeaterRequests={repeaterRequests}
                activeId={activeId}
                expandedMap={expandedMap}
                toggleExpand={toggleExpand}
                onSelectRequest={onSelectRequest}
                onContextMenuFolder={handleContextMenuFolder}
                onContextMenuRequest={handleContextMenuRequest}
              />
            ))}
          </>
        )}
      </div>

      {/* Floating Right-Click Context Menu */}
      {contextMenu && (
        <div
          ref={menuRef}
          style={{ top: contextMenu.y, left: contextMenu.x }}
          className="fixed z-50 bg-zinc-900 border border-zinc-800 rounded-lg shadow-2xl py-1 w-52 text-xs text-zinc-300 font-medium animate-in fade-in zoom-in-95 duration-100"
          onClick={(e) => e.stopPropagation()}
        >
          {contextMenu.type === 'folder' && contextMenu.targetFolder && (
            <>
              <div className="px-3 py-1.5 font-bold text-zinc-400 text-[10px] uppercase border-b border-zinc-800/80 truncate">
                📁 {contextMenu.targetFolder.name}
              </div>
              <button
                onClick={() => {
                  const targetId = contextMenu.targetFolder!.id;
                  setContextMenu(null);
                  onCreateRequest(targetId);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>➕</span> Add Request
              </button>
              <button
                onClick={() => {
                  const targetId = contextMenu.targetFolder!.id;
                  setContextMenu(null);
                  openPrompt('New Subfolder Name', '', (val) => onCreateGroup(val, targetId));
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>📁</span> Add Subfolder
              </button>
              <div className="my-1 border-t border-zinc-800/80" />
              <button
                onClick={() => {
                  const g = contextMenu.targetFolder!;
                  setContextMenu(null);
                  onOpenDocModal(g);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>📝</span> Markdown Docs
              </button>
              <button
                onClick={() => {
                  const g = contextMenu.targetFolder!;
                  setContextMenu(null);
                  onOpenExtractionModal(g);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>⚡</span> Auto Extract Rules
              </button>
              <button
                onClick={() => {
                  const g = contextMenu.targetFolder!;
                  setContextMenu(null);
                  onRenameGroup(g);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>✏️</span> Rename Folder
              </button>
              <div className="my-1 border-t border-zinc-800/80" />
              <button
                onClick={() => {
                  const g = contextMenu.targetFolder!;
                  setContextMenu(null);
                  onDeleteGroup(g);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-rose-500/20 text-rose-400 flex items-center gap-2"
              >
                <span>🗑️</span> Delete Folder
              </button>
            </>
          )}

          {contextMenu.type === 'request' && contextMenu.targetRequest && (
            <>
              <div className="px-3 py-1.5 font-bold text-zinc-400 text-[10px] uppercase border-b border-zinc-800/80 truncate">
                {contextMenu.targetRequest.method} - {contextMenu.targetRequest.name}
              </div>
              <button
                onClick={() => {
                  const reqId = contextMenu.targetRequest!.id;
                  setContextMenu(null);
                  onSelectRequest(reqId);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>⚡</span> Select & Edit
              </button>
              <div className="my-1 border-t border-zinc-800/80" />
              <button
                onClick={() => {
                  const reqId = contextMenu.targetRequest!.id;
                  setContextMenu(null);
                  onDeleteRequest(reqId);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-rose-500/20 text-rose-400 flex items-center gap-2"
              >
                <span>🗑️</span> Delete Request
              </button>
            </>
          )}

          {contextMenu.type === 'uncategorized' && (
            <>
              <div className="px-3 py-1.5 font-bold text-zinc-400 text-[10px] uppercase border-b border-zinc-800/80 truncate">
                📦 (Uncategorized)
              </div>
              <button
                onClick={() => {
                  setContextMenu(null);
                  onCreateRequest(null);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-purple-500/20 hover:text-purple-300 flex items-center gap-2"
              >
                <span>➕</span> Add Request
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
