import React, { useState } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MethodBadge } from '../../common/MethodBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const CollectionSidebar: React.FC = () => {
  const {
    groups,
    requests,
    createNewRequest,
    createGroup,
    deleteGroup,
    openTab,
    setCurlModalOpen,
    deleteRequest,
  } = useRepeaterStore();

  const [newGroupName, setNewGroupName] = useState('');
  const [isAddingGroup, setIsAddingGroup] = useState(false);

  const handleCreateGroupSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newGroupName.trim()) return;
    await createGroup(newGroupName.trim());
    setNewGroupName('');
    setIsAddingGroup(false);
  };

  return (
    <aside className="w-64 bg-surface border-r border-border h-full flex flex-col overflow-hidden text-xs shrink-0 select-none">
      {/* Sidebar Top Controls */}
      <div className="p-2 bg-header border-b border-border flex items-center justify-between gap-1">
        <span className="font-semibold text-foreground text-xs">Collections & Requests</span>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setCurlModalOpen(true)}
            className="p-1 rounded hover:bg-neutral-subtle text-muted-foreground hover:text-foreground"
            title="Import cURL command"
          >
            <MingCuteIcon name="download_line" size={14} />
          </button>
          <button
            onClick={() => setIsAddingGroup(true)}
            className="p-1 rounded hover:bg-neutral-subtle text-muted-foreground hover:text-foreground"
            title="Create Collection Folder"
          >
            <MingCuteIcon name="folder_line" size={14} />
          </button>
          <button
            onClick={() => createNewRequest(null)}
            className="p-1 rounded hover:bg-neutral-subtle text-primary"
            title="Create New Request"
          >
            <MingCuteIcon name="plus_line" size={14} />
          </button>
        </div>
      </div>

      {/* New Group Inline Form */}
      {isAddingGroup && (
        <form onSubmit={handleCreateGroupSubmit} className="p-2 bg-background border-b border-border flex items-center gap-1">
          <input
            type="text"
            value={newGroupName}
            onChange={(e) => setNewGroupName(e.target.value)}
            placeholder="Folder Name..."
            className="w-full bg-surface border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
            autoFocus
          />
          <button type="submit" className="p-1 text-emerald-500 hover:bg-emerald-500/10 rounded">
            <MingCuteIcon name="check_line" size={14} />
          </button>
          <button type="button" onClick={() => setIsAddingGroup(false)} className="p-1 text-rose-500 hover:bg-rose-500/10 rounded">
            <MingCuteIcon name="close_line" size={14} />
          </button>
        </form>
      )}

      {/* Tree Content */}
      <div className="flex-1 overflow-y-auto p-2 space-y-3">
        {/* Collections */}
        {groups.map((group) => {
          const groupRequests = requests.filter((r) => r.groupId === group.id);
          return (
            <div key={group.id} className="space-y-1">
              <div className="flex items-center justify-between group px-1 py-0.5 rounded hover:bg-neutral-subtle font-semibold text-foreground">
                <div className="flex items-center gap-1.5 truncate">
                  <MingCuteIcon name="folder_line" size={14} className="text-amber-500" />
                  <span className="truncate">{group.name}</span>
                  <span className="text-[10px] text-muted-foreground font-normal font-mono">({groupRequests.length})</span>
                </div>
                <div className="opacity-0 group-hover:opacity-100 flex items-center gap-1">
                  <button
                    onClick={() => createNewRequest(group.id)}
                    className="text-muted-foreground hover:text-primary p-0.5"
                    title="Add request to folder"
                  >
                    <MingCuteIcon name="plus_line" size={12} />
                  </button>
                  <button
                    onClick={() => deleteGroup(group.id)}
                    className="text-muted-foreground hover:text-rose-500 p-0.5"
                    title="Delete collection"
                  >
                    <MingCuteIcon name="delete_2_line" size={12} />
                  </button>
                </div>
              </div>

              {/* Group Requests */}
              <div className="pl-4 space-y-0.5">
                {groupRequests.map((req) => (
                  <div
                    key={req.id}
                    onClick={() => openTab(req)}
                    className="flex items-center justify-between group/req px-2 py-1 rounded hover:bg-neutral-subtle/80 cursor-pointer font-mono text-xs transition-colors"
                  >
                    <div className="flex items-center gap-2 truncate">
                      <MethodBadge method={req.method} />
                      <span className="text-foreground truncate font-sans text-xs">{req.name}</span>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteRequest(req.id);
                      }}
                      className="opacity-0 group-hover/req:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5"
                    >
                      <MingCuteIcon name="close_line" size={12} />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          );
        })}

        {/* Uncategorized Requests */}
        {requests.filter((r) => !r.groupId).length > 0 && (
          <div className="space-y-1 pt-2 border-t border-border/50">
            <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground px-1">
              Uncategorized
            </span>
            <div className="space-y-0.5">
              {requests
                .filter((r) => !r.groupId)
                .map((req) => (
                  <div
                    key={req.id}
                    onClick={() => openTab(req)}
                    className="flex items-center justify-between group/req px-2 py-1 rounded hover:bg-neutral-subtle/80 cursor-pointer font-mono text-xs transition-colors"
                  >
                    <div className="flex items-center gap-2 truncate">
                      <MethodBadge method={req.method} />
                      <span className="text-foreground truncate font-sans text-xs">{req.name}</span>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteRequest(req.id);
                      }}
                      className="opacity-0 group-hover/req:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5"
                    >
                      <MingCuteIcon name="close_line" size={12} />
                    </button>
                  </div>
                ))}
            </div>
          </div>
        )}
      </div>
    </aside>
  );
};
