import React, { useState } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Input, Button } from '../../common/ui';

/** Fills its parent; RepeaterView owns the (resizable) width */
export const RepeaterSidebar: React.FC = () => {
  const {
    tabs,
    activeTabId,
    setActiveTab,
    createNewRequest,
    deleteTab,
  } = useRepeaterStore();

  const [search, setSearch] = useState('');

  const parseUrlParts = (rawUrl: string) => {
    if (!rawUrl || !rawUrl.trim()) return { host: 'https://localhost', path: '/' };
    try {
      const urlWithScheme = !rawUrl.startsWith('http://') && !rawUrl.startsWith('https://')
        ? `https://${rawUrl}`
        : rawUrl;
      const parsed = new URL(urlWithScheme);
      return {
        host: `${parsed.protocol}//${parsed.host}`,
        path: (parsed.pathname || '/') + parsed.search,
      };
    } catch {
      return { host: rawUrl, path: '/' };
    }
  };

  const filteredTabs = tabs.filter((t) => {
    if (!search.trim()) return true;
    const term = search.toLowerCase();
    const { host, path } = parseUrlParts(t.url);
    return host.toLowerCase().includes(term) || path.toLowerCase().includes(term) || t.method.toLowerCase().includes(term);
  });

  const handleCreateNew = () => {
    setSearch('');
    createNewRequest();
  };

  return (
    <div
      className="bg-surface border-r border-border h-full w-full flex flex-col overflow-hidden text-xs select-none"
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
            onClick={handleCreateNew}
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
      <div className="flex-1 overflow-y-auto divide-y divide-border/50 no-scrollbar">
        {filteredTabs.length === 0 ? (
          <div className="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2">
            <MingCuteIcon name="folder_open_line" size={32} className="opacity-40" />
            <span>{search ? 'No matching requests' : 'No requests created'}</span>
            {!search && (
              <Button variant="outline" sizeVariant="xs" icon="plus_line" onClick={handleCreateNew}>
                Create Request
              </Button>
            )}
          </div>
        ) : (
          filteredTabs.map((tab) => {
            const isActive = tab.id === activeTabId;
            const { host, path } = parseUrlParts(tab.url);
            const hits = tab.executionCount || 0;

            return (
              <div
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`group px-3 py-2 border-l-2 text-xs cursor-pointer transition-colors flex flex-col gap-1 ${
                  isActive
                    ? 'bg-primary/10 border-l-primary'
                    : 'border-l-transparent hover:bg-neutral-subtle text-foreground'
                }`}
              >
                {/* Main Title Line: [Method Badge] https://hostname */}
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-1.5 flex-1 min-w-0 font-mono">
                    <MethodBadge method={tab.method} />
                    <span
                      className={`truncate text-xs font-semibold ${
                        isActive ? 'text-primary' : 'text-foreground'
                      }`}
                      title={tab.url}
                    >
                      {host}
                    </span>
                  </div>

                  {/* Actions: Delete */}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteTab(tab.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 -my-1 p-1 rounded transition-opacity cursor-pointer"
                    title="Delete Request"
                  >
                    <MingCuteIcon name="close_line" size={13} />
                  </button>
                </div>

                {/* Subtitle: Path / Directory */}
                <div className="font-mono text-[11px] truncate text-muted-foreground leading-tight pl-0.5">
                  {path || '/'}
                </div>

                {/* Stats Bar: Hits Count & Last Response Code / Latency */}
                <div className="flex items-center justify-between text-[10px] text-muted-foreground font-mono">
                  <span className="flex items-center gap-1 text-muted-foreground">
                    <MingCuteIcon name="flash_line" size={11} className="text-amber-500" />
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
                    <span className="text-muted-foreground/60 italic text-[10px]">Unsent</span>
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
