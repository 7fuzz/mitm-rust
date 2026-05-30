import { memo, useRef, useEffect } from 'react';

interface TrafficItemProps {
  id: string;
  method: string;
  status: number; // 0 for pending/unsent
  title: string;
  subtitle?: string;
  timestamp?: number;
  group?: string;
  hitCount?: number;
  isIntercepted?: boolean;
  isActive: boolean;
  isHighlighted?: boolean;
  activeColor?: 'emerald' | 'purple' | 'sky' | 'rose';
  onClick: (id: string) => void;
  onDelete?: (id: string) => void;
}

export const TrafficItem = memo(({
  id, method, status, title, subtitle, timestamp, group, hitCount, isIntercepted, isActive, isHighlighted, activeColor = 'emerald', onClick, onDelete
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
    if (m === 'DELETE') return 'text-rose-400';
    if (m === 'PUT' || m === 'PATCH') return 'text-amber-400';
    return 'text-purple-400';
  };

  const getStatusColor = (s: number) => {
    if (s === 0) return 'text-zinc-600';
    if (s < 300) return 'text-emerald-text';
    if (s < 400) return 'text-amber-500';
    return 'text-rose-500';
  };

  const activeBorder =
    activeColor === 'purple' ? 'border-l-purple-500' :
      activeColor === 'sky' ? 'border-l-sky-500' :
        activeColor === 'rose' ? 'border-l-rose-500' :
          'border-l-emerald-500';

  return (
    <div
      ref={itemRef}
      onClick={() => onClick(id)}
      className={`cursor-pointer transition-all border-l-4 group relative ${
        isActive 
          ? `bg-zinc-900 ${activeBorder}` 
          : isHighlighted 
            ? 'bg-emerald-500/10 border-l-emerald-500/50 shadow-inner shadow-emerald-500/5' 
            : 'bg-transparent border-l-transparent hover:border-l-zinc-800 hover:bg-zinc-900/40'
      } ${isIntercepted ? 'border-l-rose-500 bg-rose-500/5' : ''}`}
    >
      <div className="p-3 space-y-1.5 flex flex-col min-w-0">
        <div className="flex items-center gap-2 justify-between">
          <div className="flex items-center gap-2 text-[10px] uppercase font-black tracking-wider shrink-0">
            <span className={`px-1.5 py-0.5 rounded bg-zinc-950/50 border border-zinc-800/50 ${getMethodColor(method)}`}>{method}</span>
            <span className={`${getStatusColor(status)} font-mono`}>
              {status === 0 ? (isIntercepted ? 'PAUSED' : 'PENDING') : status}
            </span>
            {isIntercepted && (
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-rose-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-rose-500"></span>
              </span>
            )}
            {/* Visual badge for Groups */}
            {group && group !== 'Default' && (
              <span className="text-[8px] bg-purple-500/10 text-purple-400/80 px-1.5 py-0.5 rounded border border-purple-500/20 font-mono truncate max-w-24">
                {group}
              </span>
            )}
            {hitCount !== undefined && hitCount > 0 && (
              <span className="text-[8px] bg-emerald-500/10 text-emerald-text/80 px-1.5 py-0.5 rounded border border-emerald-500/20 font-mono shrink-0">
                Hits: {hitCount}
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
        <div className="text-zinc-300 text-xs truncate w-full font-medium" title={title}>
          {title}
        </div>
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
