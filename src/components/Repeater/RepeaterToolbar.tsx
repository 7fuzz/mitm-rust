import { useRef, useEffect } from 'react';
import { Button } from '../ui';

interface RepeaterGroupOption {
  id: string;
  name: string;
}

interface RepeaterToolbarProps {
  simpleMode: boolean;
  activeGroupId: string;
  repeaterGroups: RepeaterGroupOption[];
  activeGroupObj?: RepeaterGroupOption;
  hasCurrentReq: boolean;
  hasCurrentResponse: boolean;
  isLoading: boolean;
  showNewMenu: boolean;
  setShowNewMenu: (val: boolean) => void;
  onSwitchGroup: (groupId: string) => void;
  onOpenDocModal: () => void;
  onRenameGroup: () => void;
  onOpenExtractionModal: () => void;
  onDeleteGroup: () => void;
  onOpenHistoryModal: () => void;
  onClearResponse: () => void;
  onAddEmptyRequest: () => void;
  onOpenCurlModal: () => void;
  onOpenWebhookModal: () => void;
  onDuplicateRequest: () => void;
  onCopyAsCurl: () => void;
  onExecute: () => void;
}

export function RepeaterToolbarLeft({
  simpleMode,
  activeGroupId,
  repeaterGroups,
  onOpenDocModal,
  onRenameGroup,
  onOpenExtractionModal,
  onDeleteGroup,
}: Pick<
  RepeaterToolbarProps,
  | 'simpleMode'
  | 'activeGroupId'
  | 'repeaterGroups'
  | 'onOpenDocModal'
  | 'onRenameGroup'
  | 'onOpenExtractionModal'
  | 'onDeleteGroup'
>) {
  if (simpleMode) return null;

  const isSystemGroup = !activeGroupId || activeGroupId === 'All' || activeGroupId === 'null';
  const currentGroup = repeaterGroups.find((g) => g.id === activeGroupId);
  const groupLabel = isSystemGroup ? 'Uncategorized' : currentGroup?.name || 'Collection';

  return (
    <div className="flex items-center gap-2 bg-zinc-950 p-1 rounded-full border border-zinc-800 px-3 shadow-inner shadow-app-shadow/50">
      <span className="text-[9px] text-zinc-500 font-black uppercase tracking-widest hidden sm:inline-block">
        Location:
      </span>
      <div className="flex items-center gap-1.5 px-2.5 py-1 bg-zinc-900 border border-zinc-800 rounded-md text-xs font-bold text-purple-300">
        <span>📁</span>
        <span className="truncate max-w-48">{groupLabel}</span>
      </div>

      <div className="flex items-center gap-1 border-l border-zinc-800 pl-2 ml-1">
        <button
          onClick={onOpenDocModal}
          disabled={isSystemGroup}
          className="px-2 py-1 text-[10px] font-bold text-zinc-400 hover:text-purple-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors flex items-center gap-1 bg-zinc-900 border border-zinc-800 rounded hover:border-purple-500/40"
          title="Collection Documentation & Notes (Markdown)"
        >
          <span>📝 Docs</span>
        </button>

        <button
          onClick={onOpenExtractionModal}
          disabled={isSystemGroup}
          className="px-2 py-1 text-[10px] font-bold text-zinc-400 hover:text-amber-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors flex items-center gap-1 bg-zinc-900 border border-zinc-800 rounded hover:border-amber-500/40"
          title="Collection Auto Extraction Rules (Extract response values into variables)"
        >
          <span>⚡ Auto Extract</span>
        </button>

        <button
          onClick={onRenameGroup}
          disabled={isSystemGroup}
          className="p-1 text-zinc-500 hover:text-purple-400 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors"
          title="Rename Collection"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 20h9"></path>
            <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path>
          </svg>
        </button>

        <button
          onClick={onDeleteGroup}
          disabled={isSystemGroup}
          className="p-1 text-zinc-500 hover:text-rose-500 disabled:opacity-20 disabled:hover:text-zinc-500 transition-colors"
          title="Delete Collection"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
        </button>
      </div>
    </div>
  );
}

export function RepeaterToolbarRight({
  hasCurrentReq,
  hasCurrentResponse,
  isLoading,
  showNewMenu,
  setShowNewMenu,
  onOpenHistoryModal,
  onClearResponse,
  onAddEmptyRequest,
  onOpenCurlModal,
  onOpenWebhookModal,
  onDuplicateRequest,
  onCopyAsCurl,
  onExecute,
}: Pick<
  RepeaterToolbarProps,
  | 'hasCurrentReq'
  | 'hasCurrentResponse'
  | 'isLoading'
  | 'showNewMenu'
  | 'setShowNewMenu'
  | 'onOpenHistoryModal'
  | 'onClearResponse'
  | 'onAddEmptyRequest'
  | 'onOpenCurlModal'
  | 'onOpenWebhookModal'
  | 'onDuplicateRequest'
  | 'onCopyAsCurl'
  | 'onExecute'
>) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowNewMenu(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [setShowNewMenu]);

  return (
    <div className="flex items-center">
      <Button
        variant="ghost"
        size="sm"
        onClick={onOpenHistoryModal}
        disabled={!hasCurrentReq}
        title="View Request History"
        className="mr-2"
      >
        History
      </Button>

      <Button
        variant="destructive"
        size="sm"
        onClick={onClearResponse}
        disabled={!hasCurrentResponse}
        className="mr-2"
      >
        Clear
      </Button>

      <div className="flex items-center gap-px">
        <div className="relative" ref={menuRef}>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setShowNewMenu(!showNewMenu)}
            className="rounded-r-none border-r-0 text-emerald-text"
            title="New Request"
          >
            + New ▾
          </Button>

          {showNewMenu && (
            <div className="absolute top-full left-0 mt-1 bg-zinc-900 border border-zinc-700 rounded shadow-xl z-50 min-w-44 py-1 text-[11px]">
              <button
                onClick={() => {
                  setShowNewMenu(false);
                  onAddEmptyRequest();
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-zinc-800 text-zinc-300 flex items-center gap-2"
              >
                <span className="text-emerald-400">◇</span> Empty Request
              </button>

              <button
                onClick={() => {
                  setShowNewMenu(false);
                  onOpenCurlModal();
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-zinc-800 text-zinc-300 flex items-center gap-2"
              >
                <span className="text-amber-400">⌘</span> Import from cURL...
              </button>

              <button
                onClick={() => {
                  setShowNewMenu(false);
                  onOpenWebhookModal();
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-zinc-800 text-zinc-300 flex items-center gap-2 border-t border-zinc-800"
              >
                <span className="text-purple-400">⚡</span> New Webhook Request...
              </button>
            </div>
          )}
        </div>

        <Button
          variant="secondary"
          size="sm"
          onClick={onDuplicateRequest}
          disabled={!hasCurrentReq}
          className="rounded-none border-r-0"
          title="Duplicate Request"
        >
          Clone
        </Button>

        <Button
          variant="secondary"
          size="sm"
          onClick={onCopyAsCurl}
          disabled={!hasCurrentReq}
          className="rounded-l-none"
          title="Copy as cURL"
        >
          cURL
        </Button>
      </div>

      <Button
        variant="purple"
        size="sm"
        onClick={onExecute}
        disabled={isLoading || !hasCurrentReq}
        className="ml-2 min-w-24"
      >
        {isLoading ? 'Executing...' : 'Execute'}
      </Button>
    </div>
  );
}
