import React, { useState, useRef } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { MethodBadge } from '../../common/MethodBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { RuleManagerDrawer } from './RuleManagerDrawer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl, Button } from '../../common/ui';

const DIRECTION_OPTIONS = [
  { value: 'request', label: 'Request Only' },
  { value: 'response', label: 'Response Only' },
  { value: 'both', label: 'Both (Req & Res)' },
] as const;

export const InterceptView: React.FC = () => {
  const {
    interceptConfig,
    toggleIntercept,
    setInterceptDirection,
    pendingQueue,
    forwardPendingFlow,
    dropPendingFlow,
    forwardAllPending,
  } = useProxyStore();

  const [selectedFlowId, setSelectedFlowId] = useState<string | null>(
    pendingQueue.length > 0 ? pendingQueue[0].id : null
  );

  const activeFlow = pendingQueue.find((f) => f.id === selectedFlowId) || (pendingQueue.length > 0 ? pendingQueue[0] : null);

  const [editedBody, setEditedBody] = useState<string>('');

  // Resizable panel widths & drawer minimize state
  const [queueWidth, setQueueWidth] = useState<number>(256);
  const [ruleDrawerWidth, setRuleDrawerWidth] = useState<number>(320);
  const [isRuleDrawerMinimized, setIsRuleDrawerMinimized] = useState<boolean>(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const isResizingQueue = useRef(false);
  const isResizingRuleDrawer = useRef(false);

  const handleMouseDownQueueSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingQueue.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingQueue.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = moveEvent.clientX - rect.left;
      setQueueWidth(Math.min(Math.max(newWidth, 160), 480));
    };

    const handleMouseUp = () => {
      isResizingQueue.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const handleMouseDownRuleDrawerSplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingRuleDrawer.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingRuleDrawer.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = rect.right - moveEvent.clientX;
      setRuleDrawerWidth(Math.min(Math.max(newWidth, 200), 550));
    };

    const handleMouseUp = () => {
      isResizingRuleDrawer.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  React.useEffect(() => {
    if (activeFlow) {
      setEditedBody(activeFlow.body);
    }
  }, [activeFlow]);

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden">
      {/* Full Width Top Control Bar (Fixed Header) */}
      <div className="p-3 bg-header border-b border-border flex items-center justify-between gap-3 shrink-0 text-xs select-none">
        {/* Left: Intercept ON / OFF Toggle & Direction Selector */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => toggleIntercept()}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-full font-bold transition-all shadow-md ${
              interceptConfig.enabled
                ? 'bg-rose-500 text-white ring-4 ring-rose-500/20 animate-pulse'
                : 'bg-neutral-subtle border border-border text-muted-foreground'
            }`}
          >
            <MingCuteIcon name="shield_line" size={16} />
            <span>Intercept: {interceptConfig.enabled ? 'ON' : 'OFF'}</span>
          </button>

          {/* Direction Target Selector (Atomic SegmentedControl) */}
          <SegmentedControl
            label="Target"
            value={interceptConfig.direction}
            onChange={(val) => setInterceptDirection(val as any)}
            options={DIRECTION_OPTIONS}
          />

          {/* Pending Counter Pill */}
          <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-surface border border-border font-mono font-medium">
            <span className="w-2 h-2 rounded-full bg-amber-500 animate-ping" />
            <span>{pendingQueue.length} pending waiting</span>
          </div>
        </div>

        {/* Right Action Controls: Forward, Drop, Forward All (Atomic Buttons) */}
        <div className="flex items-center gap-2">
          <Button
            variant="success"
            icon="play_line"
            disabled={!activeFlow}
            onClick={() => activeFlow && forwardPendingFlow(activeFlow.id, editedBody)}
          >
            Forward
          </Button>

          <Button
            variant="danger"
            icon="close_circle_line"
            disabled={!activeFlow}
            onClick={() => activeFlow && dropPendingFlow(activeFlow.id)}
          >
            Drop
          </Button>

          <Button
            variant="subtle"
            disabled={pendingQueue.length === 0}
            onClick={forwardAllPending}
          >
            Forward All
          </Button>
        </div>
      </div>

      {/* Main Workspace Split Area (Below Control Bar) */}
      <div className="flex-1 flex overflow-hidden">
        {/* Pending Queue List (Left Side - Resizable Width) */}
        <div
          className="border-r border-border bg-surface flex flex-col overflow-hidden shrink-0"
          style={{ width: `${queueWidth}px` }}
        >
          <div className="p-2 bg-header border-b border-border text-[11px] font-semibold uppercase tracking-wider text-muted-foreground select-none">
            Queue ({pendingQueue.length})
          </div>
          <div className="flex-1 overflow-y-auto divide-y divide-border">
            {pendingQueue.length === 0 ? (
              <div className="p-4 text-center text-muted-foreground italic text-xs">
                No intercepted flows in queue
              </div>
            ) : (
              pendingQueue.map((flow) => {
                const isSelected = activeFlow?.id === flow.id;
                return (
                  <div
                    key={flow.id}
                    onClick={() => setSelectedFlowId(flow.id)}
                    className={`p-2 cursor-pointer font-mono text-xs transition-colors ${
                      isSelected ? 'bg-primary/15 font-semibold text-foreground border-l-2 border-primary' : 'hover:bg-neutral-subtle'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-1 mb-1">
                      <MethodBadge method={flow.method} />
                      <span className="text-[10px] text-muted-foreground">{new Date(flow.timestamp).toLocaleTimeString()}</span>
                    </div>
                    <div className="text-foreground text-xs truncate">{flow.url}</div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* INVISIBLE DRAGGABLE RESIZER HANDLE FOR QUEUE SIDEBAR */}
        <div
          onMouseDown={handleMouseDownQueueSplit}
          className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
          title="Drag to adjust Queue sidebar width"
        >
          <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
        </div>

        {/* Pending Editor (Center Main Panel) */}
        <div className="flex-1 flex flex-col bg-surface overflow-hidden">
          {activeFlow ? (
            <div className="flex-1 flex flex-col p-3 overflow-hidden gap-3">
              {/* Request Header Banner */}
              <div className="p-2.5 rounded bg-header border border-border flex items-center justify-between font-mono text-xs">
                <div className="flex items-center gap-2">
                  <MethodBadge method={activeFlow.method} />
                  <span className="font-bold text-foreground">{activeFlow.url}</span>
                </div>
                <span className="px-2 py-0.5 rounded bg-amber-500/10 text-amber-500 font-bold uppercase text-[10px] border border-amber-500/20">
                  INTERCEPTED
                </span>
              </div>

              {/* Body Monaco Editor */}
              <div className="flex-1 flex flex-col gap-1 overflow-hidden">
                <span className="text-xs font-semibold text-foreground">Editable Request Payload:</span>
                <div className="flex-1 overflow-hidden">
                  <CodeEditor
                    value={editedBody}
                    onChange={setEditedBody}
                    language="json"
                    readOnly={false}
                  />
                </div>
              </div>
            </div>
          ) : (
            <div className="h-full flex flex-col items-center justify-center text-muted-foreground italic text-xs p-6">
              <MingCuteIcon name="shield_line" size={40} className="mb-2 opacity-30" />
              No intercepted request selected. Enable Intercept toggle above or trigger an HTTP request through the proxy.
            </div>
          )}
        </div>

        {/* INVISIBLE DRAGGABLE RESIZER HANDLE FOR RULE MANAGER DRAWER */}
        {!isRuleDrawerMinimized && (
          <div
            onMouseDown={handleMouseDownRuleDrawerSplit}
            className="w-1 cursor-col-resize -mx-0.5 bg-transparent hover:bg-primary/50 active:bg-primary shrink-0 transition-colors flex items-center justify-center group z-10 relative"
            title="Drag to adjust Rule Manager width"
          >
            <div className="h-8 w-0.5 bg-primary opacity-0 group-hover:opacity-100 transition-opacity rounded" />
          </div>
        )}

        {/* Right Drawer: Rule Manager (Same workspace row below top control bar) */}
        <RuleManagerDrawer
          isMinimized={isRuleDrawerMinimized}
          onToggleMinimize={() => setIsRuleDrawerMinimized(!isRuleDrawerMinimized)}
          widthPx={ruleDrawerWidth}
        />
      </div>
    </div>
  );
};
