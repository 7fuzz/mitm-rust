import { memo, useRef, useEffect } from 'react';

const splitUrlForDisplay = (url: string): [string, string] => {
  try {
    const parsed = new URL(url);
    const hostPart = parsed.origin;
    const pathPart = parsed.pathname + parsed.search + parsed.hash;
    return [hostPart, pathPart || '/'];
  } catch {
    const protocolIndex = url.indexOf('://');
    if (protocolIndex !== -1) {
      const firstSlash = url.indexOf('/', protocolIndex + 3);
      if (firstSlash !== -1) {
        return [url.substring(0, firstSlash), url.substring(firstSlash)];
      }
    }
    return [url, ''];
  }
};

interface TrafficItemProps {
  id: string;
  method: string;
  status: number; // 0 for pending/unsent
  title: string;
  subtitle?: string;
  timestamp?: number;
  group?: string;
  hitCount?: number;
  duration_ms?: number;
  isIntercepted?: boolean;
  isActive: boolean;
  isHighlighted?: boolean;
  activeColor?: 'emerald' | 'purple' | 'sky' | 'rose';
  onClick: (id: string) => void;
  onDelete?: (id: string) => void;
  dragHandleProps?: Record<string, any>;
}

export const TrafficItem = memo(({
  id, method, status, title, subtitle, timestamp, group, hitCount, duration_ms, isIntercepted, isActive, isHighlighted, activeColor = 'emerald', onClick, onDelete, dragHandleProps
}: TrafficItemProps) => {

  const itemRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isHighlighted && itemRef.current) {
      itemRef.current.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  }, [isHighlighted]);

  const getMethodColor = (m: string) => {
    if (m === 'GET') return 'text-sky-text';
    if (m === 'POST') return 'text-emerald-text';
    if (m === 'DELETE') return 'text-rose-text';
    if (m === 'PUT' || m === 'PATCH') return 'text-amber-text';
    return 'text-purple-text';
  };

  const getStatusColor = (s: number) => {
    if (s === 0) return 'text-zinc-600';
    if (s < 300) return 'text-emerald-text';
    if (s < 400) return 'text-amber-text';
    return 'text-rose-text';
  };

  const activeBorder =
    activeColor === 'purple' ? 'border-l-purple-border' :
      activeColor === 'sky' ? 'border-l-sky-border' :
        activeColor === 'rose' ? 'border-l-rose-border' :
          'border-l-emerald-border';

  return (
    <div
      ref={itemRef}
      onClick={() => onClick(id)}
      className={`cursor-pointer transition-all border-l-4 group relative flex items-center ${
        isActive 
          ? `bg-zinc-900/80 ${activeBorder}` 
          : isHighlighted 
            ? 'bg-emerald-highlight-bg border-l-emerald-highlight-border shadow-inner shadow-emerald-500/5' 
            : 'bg-transparent border-l-transparent hover:border-l-zinc-800 hover:bg-zinc-900/40'
      } ${isIntercepted ? 'border-l-rose-border bg-rose-highlight-bg' : ''}`}
    >
      {dragHandleProps && (
        <div 
          {...dragHandleProps} 
          className="pl-2 cursor-grab active:cursor-grabbing text-zinc-600 hover:text-zinc-400 transition-colors shrink-0"
          onClick={(e) => e.stopPropagation()}
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><circle cx="9" cy="5" r="1"></circle><circle cx="9" cy="12" r="1"></circle><circle cx="9" cy="19" r="1"></circle><circle cx="15" cy="5" r="1"></circle><circle cx="15" cy="12" r="1"></circle><circle cx="15" cy="19" r="1"></circle></svg>
        </div>
      )}
      <div className="p-3 space-y-1.5 flex flex-col min-w-0 flex-1">
        <div className="flex items-center gap-2 justify-between">
          <div className="flex items-center gap-2 text-[10px] uppercase font-black tracking-wider shrink-0">
            <span className={`px-1.5 py-0.5 rounded bg-zinc-950/50 border border-zinc-800/50 ${getMethodColor(method)}`}>{method}</span>
            <span className={`${getStatusColor(status)} font-mono`}>
              {status === 0 ? (isIntercepted ? 'PAUSED' : 'PENDING') : status}
            </span>
            {isIntercepted && (
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-rose-border opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-rose-border"></span>
              </span>
            )}
            {/* Visual badge for Groups */}
            {group && group !== 'Default' && (
              <span className="text-[8px] bg-purple-highlight-bg text-purple-text px-1.5 py-0.5 rounded border border-purple-highlight-border font-mono truncate max-w-24">
                {group}
              </span>
            )}
            {hitCount !== undefined && hitCount > 0 && (
              <span className="text-[8px] bg-emerald-highlight-bg text-emerald-text px-1.5 py-0.5 rounded border border-emerald-highlight-border font-mono shrink-0">
                Hits: {hitCount}
              </span>
            )}
            {duration_ms !== undefined && duration_ms > 0 && (
              <span className="text-[8px] bg-sky-highlight-bg text-sky-text px-1.5 py-0.5 rounded border border-sky-highlight-border font-mono shrink-0">
                {duration_ms}ms
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {timestamp && (
              <span className="text-zinc-600 text-[9px] font-mono">
                {new Date(timestamp).toLocaleTimeString()}
              </span>
            )}
            {onDelete && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(id);
                }}
                className="p-1 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded opacity-0 group-hover:opacity-100 transition-all"
                title="Delete"
              >
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
              </button>
            )}
          </div>
        </div>
        {(() => {
          const [host, path] = splitUrlForDisplay(title);
          return (
            <div className="text-xs w-full flex flex-col min-w-0 font-medium" title={title}>
              {host && <span className="text-zinc-500 text-[10px] font-normal truncate leading-tight">{host}</span>}
              <span className="text-zinc-300 truncate leading-normal">{path || '/'}</span>
            </div>
          );
        })()}
        {subtitle && (
          <div className="text-zinc-400 text-[10px] truncate font-mono" title={subtitle}>
            {subtitle}
          </div>
        )}
      </div>
    </div>
  );
});

TrafficItem.displayName = 'TrafficItem';
