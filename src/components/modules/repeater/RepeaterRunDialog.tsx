import React, { useEffect, useMemo, useRef, useState } from 'react';
import type { RepeaterHistoryItem } from '../../../services/tauri/bridge';
import type { TrafficItem } from '../../../types';
import { Dialog } from '../../common/ui';
import { PaneHeader } from '../../common/PaneHeader';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { MethodBadge } from '../../common/MethodBadge';
import { StatusBadge } from '../../common/StatusBadge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { CopyCustomModal } from '../history/CopyCustomModal';
import { historyItemToTrafficItem, RUN_COPY_ACTIONS } from '../../../utils/repeaterTraffic';

interface RepeaterRunDialogProps {
  /** Runs newest first, as the drawer lists them */
  runs: RepeaterHistoryItem[];
  index: number | null;
  onIndexChange: (index: number) => void;
  onClose: () => void;
  /** True when the editor has edits made after its latest run */
  hasUnsentChanges: boolean;
  onLoad: (run: RepeaterHistoryItem) => Promise<void>;
  onLoadAndSend: (run: RepeaterHistoryItem) => Promise<void>;
}

const RunBody: React.FC<{ body?: string }> = ({ body }) => {
  const formatted = useMemo(() => {
    if (!body) return null;
    try {
      return { text: JSON.stringify(JSON.parse(body), null, 2), language: 'json' };
    } catch {
      return { text: body, language: body.trim().startsWith('<') ? 'html' : 'plaintext' };
    }
  }, [body]);

  if (!formatted) return <div className="p-4 text-muted-foreground italic">No body</div>;
  return <CodeEditor value={formatted.text} language={formatted.language} readOnly bare />;
};

const toRows = (items: Array<{ key: string; value: string }>, prefix: string) =>
  items.map((h, i) => ({ id: `${prefix}-${i}`, key: h.key, value: h.value, enabled: true }));

/** Full request/response view of one Repeater run, with actions to load it back into the editor. */
export const RepeaterRunDialog: React.FC<RepeaterRunDialogProps> = ({
  runs,
  index,
  onIndexChange,
  onClose,
  hasUnsentChanges,
  onLoad,
  onLoadAndSend,
}) => {
  const run = index !== null ? runs[index] : undefined;
  const [reqTab, setReqTab] = useState<'body' | 'headers' | 'params'>('body');
  const [resTab, setResTab] = useState<'body' | 'headers'>('body');
  const [pendingLoad, setPendingLoad] = useState<'load' | 'send' | null>(null);
  const [copyMenuOpen, setCopyMenuOpen] = useState(false);
  const [copyNotification, setCopyNotification] = useState<string | null>(null);
  const [customCopyItem, setCustomCopyItem] = useState<TrafficItem | null>(null);
  const copyMenuRef = useRef<HTMLDivElement>(null);

  // A confirmation belongs to the run it was asked for
  useEffect(() => {
    setPendingLoad(null);
  }, [index]);

  useEffect(() => {
    if (!copyMenuOpen) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (copyMenuRef.current && !copyMenuRef.current.contains(e.target as Node)) setCopyMenuOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [copyMenuOpen]);

  // Arrow keys step through runs (left = older, right = newer)
  useEffect(() => {
    if (index === null) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest('input, textarea, .monaco-editor')) return;
      if (e.key === 'ArrowLeft' && index < runs.length - 1) onIndexChange(index + 1);
      if (e.key === 'ArrowRight' && index > 0) onIndexChange(index - 1);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [index, runs.length, onIndexChange]);

  const params = useMemo(() => {
    if (!run) return [];
    try {
      const url = new URL(/^https?:\/\//.test(run.url) ? run.url : `https://${run.url}`);
      return Array.from(url.searchParams.entries()).map(([key, value], i) => ({ id: `p-${i}`, key, value, enabled: true }));
    } catch {
      return [];
    }
  }, [run]);

  if (!run || index === null) return null;

  const runNumber = runs.length - index;

  const doLoad = async (mode: 'load' | 'send') => {
    if (hasUnsentChanges && pendingLoad !== mode) {
      setPendingLoad(mode);
      return;
    }
    setPendingLoad(null);
    if (mode === 'send') await onLoadAndSend(run);
    else await onLoad(run);
    onClose();
  };

  const copy = (text: string, message: string) => {
    navigator.clipboard.writeText(text);
    setCopyMenuOpen(false);
    setCopyNotification(message);
    setTimeout(() => setCopyNotification(null), 1800);
  };

  const navButtonClass =
    'p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed';
  const footerButtonClass =
    'flex items-center gap-1.5 px-3 py-1 rounded border text-xs font-medium transition-colors cursor-pointer';

  return (
    <>
      <Dialog
        isOpen
        onClose={onClose}
        size="full"
        bodyClassName="flex-1 min-h-0 flex flex-col"
        footer={
          <>
            <div className="relative mr-auto" ref={copyMenuRef}>
              <button
                onClick={() => setCopyMenuOpen(!copyMenuOpen)}
                className={`${footerButtonClass} bg-background border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle`}
              >
                <MingCuteIcon name={copyNotification ? 'check_line' : 'copy_line'} size={13} className={copyNotification ? 'text-emerald-400' : ''} />
                <span>{copyNotification || 'Copy'}</span>
                <MingCuteIcon name="down_line" size={12} className="opacity-60" />
              </button>
              {copyMenuOpen && (
                <div className="absolute left-0 bottom-full mb-1 w-56 bg-surface border border-border rounded-lg shadow-xl py-1 z-50 font-mono text-xs flex flex-col">
                  {RUN_COPY_ACTIONS.map((action) => (
                    <button
                      key={action.label}
                      onClick={() => copy(action.format(historyItemToTrafficItem(run)), action.message)}
                      className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 cursor-pointer"
                    >
                      <MingCuteIcon name={action.icon} size={14} className={action.iconClass} />
                      <span>{action.label}</span>
                    </button>
                  ))}
                  <button
                    onClick={() => {
                      setCopyMenuOpen(false);
                      setCustomCopyItem(historyItemToTrafficItem(run));
                    }}
                    className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 cursor-pointer"
                  >
                    <MingCuteIcon name="settings_3_line" size={14} className="text-muted-foreground" />
                    <span>Copy custom...</span>
                  </button>
                </div>
              )}
            </div>

            {pendingLoad && (
              <span className="text-amber-500 text-[11px]">
                The editor has unsent changes. Click again to replace them.
              </span>
            )}
            <button
              onClick={() => doLoad('load')}
              className={`${footerButtonClass} ${
                pendingLoad === 'load'
                  ? 'bg-amber-500/20 border-amber-500/50 text-amber-600 dark:text-amber-300'
                  : 'bg-background border-border text-foreground hover:bg-neutral-subtle'
              }`}
              title="Replace the editor's method, URL, params, headers, and body with this run's"
            >
              <MingCuteIcon name="refresh_line" size={13} />
              <span>{pendingLoad === 'load' ? 'Replace & Load' : 'Load into editor'}</span>
            </button>
            <button
              onClick={() => doLoad('send')}
              className={`${footerButtonClass} ${
                pendingLoad === 'send'
                  ? 'bg-amber-500 border-amber-500 text-white'
                  : 'bg-primary border-primary text-primary-foreground hover:bg-primary-hover'
              }`}
              title="Load this run into the editor and send it again"
            >
              <MingCuteIcon name="send_plane_line" size={13} />
              <span>{pendingLoad === 'send' ? 'Replace & Send' : 'Load & Send'}</span>
            </button>
          </>
        }
      >
        {/* Run summary & navigation */}
        <div className="h-11 px-3 bg-header border-b border-border flex items-center gap-2 shrink-0 font-mono">
          <button onClick={() => onIndexChange(index + 1)} disabled={index >= runs.length - 1} className={navButtonClass} title="Older run (←)">
            <MingCuteIcon name="chevron_left_line" size={15} />
          </button>
          <span className="text-[11px] text-muted-foreground whitespace-nowrap">
            Run {runNumber} of {runs.length}
          </span>
          <button onClick={() => onIndexChange(index - 1)} disabled={index <= 0} className={navButtonClass} title="Newer run (→)">
            <MingCuteIcon name="chevron_right_line" size={15} />
          </button>
          <div className="h-4 w-px bg-border mx-1" />
          <MethodBadge method={run.method} />
          <StatusBadge code={run.statusCode} />
          <span className="text-foreground font-semibold truncate select-text" title={run.url}>
            {run.url}
          </span>
          <span className="ml-auto text-[11px] text-muted-foreground whitespace-nowrap">
            {run.durationMs}ms · {run.responseBody?.length ?? 0} B · {new Date(run.executedAtMs).toLocaleString()}
          </span>
          <button onClick={onClose} className={navButtonClass} title="Close (Esc)">
            <MingCuteIcon name="close_line" size={15} />
          </button>
        </div>

        {/* Request (as sent) | Response */}
        <div className="flex-1 min-h-0 flex">
          <div className="w-1/2 flex flex-col min-w-0 border-r border-border">
            <PaneHeader
              title="Request"
              tabs={[
                { value: 'body', label: 'Body' },
                { value: 'headers', label: 'Headers', count: run.requestHeaders.length },
                { value: 'params', label: 'Params', count: params.length },
              ]}
              activeTab={reqTab}
              onTabChange={(val) => setReqTab(val as typeof reqTab)}
            />
            <div className={`flex-1 min-h-0 ${reqTab === 'body' ? 'overflow-hidden' : 'p-2 overflow-auto'}`}>
              {reqTab === 'body' && <RunBody body={run.requestBody} />}
              {reqTab === 'headers' && <KeyValueEditor items={toRows(run.requestHeaders, 'rq')} onChange={() => {}} readOnly />}
              {reqTab === 'params' && <KeyValueEditor items={params} onChange={() => {}} readOnly />}
            </div>
          </div>

          <div className="w-1/2 flex flex-col min-w-0">
            <PaneHeader
              title="Response"
              tabs={[
                { value: 'body', label: 'Body' },
                { value: 'headers', label: 'Headers', count: run.responseHeaders.length },
              ]}
              activeTab={resTab}
              onTabChange={(val) => setResTab(val as typeof resTab)}
            />
            <div className={`flex-1 min-h-0 ${resTab === 'body' ? 'overflow-hidden' : 'p-2 overflow-auto'}`}>
              {resTab === 'body' && <RunBody body={run.responseBody} />}
              {resTab === 'headers' && <KeyValueEditor items={toRows(run.responseHeaders, 'rs')} onChange={() => {}} readOnly />}
            </div>
          </div>
        </div>
      </Dialog>

      <CopyCustomModal isOpen={!!customCopyItem} item={customCopyItem} onClose={() => setCustomCopyItem(null)} />
    </>
  );
};
