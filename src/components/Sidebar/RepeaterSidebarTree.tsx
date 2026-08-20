import { useState, useMemo } from 'react';
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

interface TreeNodeProps {
  group: RepeaterGroup;
  level: number;
  repeaterGroups: RepeaterGroup[];
  repeaterRequests: RepeaterRequest[];
  activeId: string | null;
  expandedMap: Record<string, boolean>;
  toggleExpand: (id: string) => void;
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

function FolderTreeNode({
  group,
  level,
  repeaterGroups,
  repeaterRequests,
  activeId,
  expandedMap,
  toggleExpand,
  onSelectRequest,
  onDeleteRequest,
  onCreateRequest,
  onCreateGroup,
  onRenameGroup,
  onDeleteGroup,
  onOpenDocModal,
  onOpenExtractionModal,
  openPrompt,
}: TreeNodeProps) {
  const isExpanded = expandedMap[group.id] ?? true;

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

        {/* Action Buttons on Hover */}
        <div
          className="flex items-center gap-1 opacity-0 group-hover/folder:opacity-100 transition-opacity"
          onClick={(e) => e.stopPropagation()}
        >
          <button
            onClick={() => onCreateRequest(group.id)}
            className="px-1.5 py-0.5 text-[9px] font-bold text-emerald-400 bg-emerald-500/10 hover:bg-emerald-500/20 rounded border border-emerald-500/30"
            title="Add Request inside this folder"
          >
            + Req
          </button>
          <button
            onClick={() =>
              openPrompt('New Subfolder Name', '', (val) => onCreateGroup(val, group.id))
            }
            className="px-1.5 py-0.5 text-[9px] font-bold text-purple-400 bg-purple-500/10 hover:bg-purple-500/20 rounded border border-purple-500/30"
            title="Add Subfolder inside this folder"
          >
            + Folder
          </button>
          <button
            onClick={() => onOpenDocModal(group)}
            className="p-1 text-zinc-400 hover:text-purple-400"
            title="Documentation (Markdown)"
          >
            📝
          </button>
          <button
            onClick={() => onOpenExtractionModal(group)}
            className="p-1 text-zinc-400 hover:text-amber-400"
            title="Auto Extract Rules"
          >
            ⚡
          </button>
          <button
            onClick={() => onRenameGroup(group)}
            className="p-1 text-zinc-400 hover:text-purple-400"
            title="Rename Folder"
          >
            ✏️
          </button>
          <button
            onClick={() => onDeleteGroup(group)}
            className="p-1 text-zinc-400 hover:text-rose-500"
            title="Delete Folder"
          >
            🗑️
          </button>
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
              onDeleteRequest={onDeleteRequest}
              onCreateRequest={onCreateRequest}
              onCreateGroup={onCreateGroup}
              onRenameGroup={onRenameGroup}
              onDeleteGroup={onDeleteGroup}
              onOpenDocModal={onOpenDocModal}
              onOpenExtractionModal={onOpenExtractionModal}
              openPrompt={openPrompt}
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
                className={`group/req flex items-center justify-between py-1.5 pr-2 rounded cursor-pointer transition-all border-l-2 ${
                  isActive
                    ? 'bg-purple-500/10 border-l-purple-500 text-purple-300'
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
                <div
                  className="flex items-center gap-1 opacity-0 group-hover/req:opacity-100 transition-opacity"
                  onClick={(e) => e.stopPropagation()}
                >
                  <button
                    onClick={() => onDeleteRequest(req.id)}
                    className="p-1 text-zinc-500 hover:text-rose-400"
                    title="Delete Request"
                  >
                    🗑️
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

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
  const [expandedMap, setExpandedMap] = useState<Record<string, boolean>>({});

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

  const [uncategorizedExpanded, setUncategorizedExpanded] = useState(true);

  return (
    <div className="flex flex-col h-full bg-zinc-950 border-r border-zinc-800/80 text-zinc-300 select-none">
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
                  className={`flex items-center justify-between p-2 rounded cursor-pointer transition-all border-l-2 ${
                    activeId === req.id
                      ? 'bg-purple-500/10 border-l-purple-500 text-purple-300'
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
            {/* Top-level Uncategorized Folder Node */}
            <div className="select-none">
              <div
                className="group/folder flex items-center justify-between py-1.5 px-2 hover:bg-zinc-900/80 rounded cursor-pointer transition-colors text-zinc-300"
                onClick={() => setUncategorizedExpanded(!uncategorizedExpanded)}
              >
                <div className="flex items-center gap-1.5 min-w-0 flex-1">
                  <span className="text-[10px] text-zinc-500 w-4 h-4 flex items-center justify-center shrink-0">
                    {uncategorizedExpanded ? '▼' : '▶'}
                  </span>
                  <span className="text-amber-400 shrink-0 text-xs">📦</span>
                  <span className="text-xs font-bold truncate text-zinc-300">Uncategorized</span>
                  <span className="text-[9px] text-zinc-500 font-mono shrink-0 bg-zinc-950 px-1 py-0.5 rounded border border-zinc-800">
                    {uncategorizedRequests.length}
                  </span>
                </div>

                <div
                  className="flex items-center gap-1 opacity-0 group-hover/folder:opacity-100 transition-opacity"
                  onClick={(e) => e.stopPropagation()}
                >
                  <button
                    onClick={() => onCreateRequest(null)}
                    className="px-1.5 py-0.5 text-[9px] font-bold text-emerald-400 bg-emerald-500/10 hover:bg-emerald-500/20 rounded border border-emerald-500/30"
                    title="Add Uncategorized Request"
                  >
                    + Req
                  </button>
                </div>
              </div>

              {/* Uncategorized Requests nested inside folder */}
              {uncategorizedExpanded && (
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
                          className={`group/req flex items-center justify-between py-1.5 pr-2 rounded cursor-pointer transition-all border-l-2 ${
                            isActive
                              ? 'bg-purple-500/10 border-l-purple-500 text-purple-300'
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
                          <div
                            className="flex items-center gap-1 opacity-0 group-hover/req:opacity-100 transition-opacity"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <button
                              onClick={() => onDeleteRequest(req.id)}
                              className="p-1 text-zinc-500 hover:text-rose-400"
                              title="Delete Request"
                            >
                              🗑️
                            </button>
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
                onDeleteRequest={onDeleteRequest}
                onCreateRequest={onCreateRequest}
                onCreateGroup={onCreateGroup}
                onRenameGroup={onRenameGroup}
                onDeleteGroup={onDeleteGroup}
                onOpenDocModal={onOpenDocModal}
                onOpenExtractionModal={onOpenExtractionModal}
                openPrompt={openPrompt}
              />
            ))}
          </>
        )}
      </div>
    </div>
  );
}
