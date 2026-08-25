import React, { useState } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Input, Button } from '../../common/ui';

interface RepeaterSidebarProps {
  widthPx?: number;
}

export const RepeaterSidebar: React.FC<RepeaterSidebarProps> = ({ widthPx = 280 }) => {
  const {
    tabs,
    activeTabId,
    setActiveTab,
    createNewRequest,
    updateTab,
    deleteTab,
  } = useRepeaterStore();

  const [search, setSearch] = useState('');
  const [editingTabId, setEditingTabId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');

  const filteredTabs = tabs.filter((t) => {
    if (!search.trim()) return true;
    const term = search.toLowerCase();
    return t.name.toLowerCase().includes(term) || t.url.toLowerCase().includes(term) || t.method.toLowerCase().includes(term);
  });

  const parseUrlParts = (rawUrl: string) => {
    if (!rawUrl.trim()) return { host: 'httpbin.org', path: '/get' };
    try {
      const urlWithScheme = !rawUrl.startsWith('http://') && !rawUrl.startsWith('https://')
        ? `http://${rawUrl}`
        : rawUrl;
      const parsed = new URL(urlWithScheme);
      return {
        host: `${parsed.protocol}//${parsed.host}`,
        path: parsed.pathname + parsed.search,
      };
    } catch {
      return { host: '', path: rawUrl };
    }
  };

  const handleStartRename = (id: string, name: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingTabId(id);
    setEditingName(name);
  };

  const handleSaveRename = (tab: any) => {
    if (editingName.trim() && editingName !== tab.name) {
      updateTab({ ...tab, name: editingName.trim() });
    }
    setEditingTabId(null);
  };

  return (
    <div
      className="bg-surface border-r border-border h-full flex flex-col overflow-hidden text-xs shrink-0 select-none"
      style={{ width: `${widthPx}px` }}
    >
      {/* Top Header & Search Bar */}
      <div className="p-2 bg-header border-b border-border flex flex-col gap-2 shrink-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5 font-semibold text-foreground">
            <MingCuteIcon name="send_plane_line" size={16} className="text-primary" />
            <span>Requests ({tabs.length})</span>
          </div>

          <Button
            variant="primary"
            sizeVariant="xs"
            icon="plus_line"
            onClick={() => createNewRequest()}
            title="Create New Repeater Request"
          >
            New
          </Button>
        </div>

        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Filter requests..."
          leftIcon="search_line"
          rightIcon={search ? 'close_line' : undefined}
          onRightIconClick={() => setSearch('')}
        />
      </div>

      {/* Request Cards List */}
      <div className="flex-1 p-2 overflow-y-auto space-y-2 no-scrollbar">
        {filteredTabs.length === 0 ? (
          <div className="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2">
            <MingCuteIcon name="folder_open_line" size={32} className="opacity-40" />
            <span>{search ? 'No matching requests' : 'No requests created'}</span>
            {!search && (
              <Button variant="outline" sizeVariant="xs" icon="plus_line" onClick={() => createNewRequest()}>
                Create Request
              </Button>
            )}
          </div>
        ) : (
          filteredTabs.map((tab) => {
            const isActive = tab.id === activeTabId;
            const isEditing = editingTabId === tab.id;
            const { host, path } = parseUrlParts(tab.url);
            const hits = tab.executionCount || 0;

            return (
              <div
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`group p-2.5 rounded-lg border text-xs cursor-pointer transition-all flex flex-col gap-1.5 ${
                  isActive
                    ? 'bg-background border-primary/60 shadow-xs ring-1 ring-primary/30'
                    : 'bg-background/60 border-border/70 hover:border-border hover:bg-background/90'
                }`}
              >
                {/* Card Title Header */}
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-1.5 flex-1 min-w-0">
                    <MethodBadge method={tab.method} />

                    {isEditing ? (
                      <input
                        type="text"
                        autoFocus
                        value={editingName}
                        onChange={(e) => setEditingName(e.target.value)}
                        onBlur={() => handleSaveRename(tab)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleSaveRename(tab);
                          if (e.key === 'Escape') setEditingTabId(null);
                        }}
                        className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
                      />
                    ) : (
                      <span
                        onDoubleClick={(e) => handleStartRename(tab.id, tab.name, e)}
                        className={`truncate font-medium ${isActive ? 'text-foreground font-semibold' : 'text-zinc-300'}`}
                        title="Double-click to rename"
                      >
                        {tab.name || 'Untitled Request'}
                      </span>
                    )}
                  </div>

                  {/* Actions: Delete */}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteTab(tab.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-1 rounded transition-opacity"
                    title="Delete Request"
                  >
                    <MingCuteIcon name="close_line" size={13} />
                  </button>
                </div>

                {/* Split Host & Path */}
                <div className="font-mono text-[11px] truncate flex items-center gap-1 leading-tight">
                  <span className="text-muted-foreground/80 shrink-0">{host}</span>
                  <span className="text-foreground font-medium truncate">{path || '/'}</span>
                </div>

                {/* Stats Bar: Hits Count & Last Response Code / Latency */}
                <div className="flex items-center justify-between text-[10px] text-muted-foreground pt-1 border-t border-border/50 font-mono">
                  <span className="flex items-center gap-1 text-zinc-400">
                    <MingCuteIcon name="flash_line" size={11} className="text-amber-400" />
                    <span>{hits} {hits === 1 ? 'hit' : 'hits'}</span>
                  </span>

                  {tab.lastStatusCode ? (
                    <div className="flex items-center gap-1.5">
                      <StatusBadge code={tab.lastStatusCode} />
                      {tab.lastDurationMs != null && (
                        <span>{tab.lastDurationMs}ms</span>
                      )}
                    </div>
                  ) : (
                    <span className="text-zinc-500 italic text-[10px]">Unsent</span>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
