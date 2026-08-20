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

export function RepeaterToolbarLeft() {
  return null;
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
