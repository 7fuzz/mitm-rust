import { useState, useRef, useEffect } from 'react';
import { Traffic } from '@/types/traffic';
import { MultiSelectFilter, FilterState } from '../ui/MultiSelectFilter';
import { useHotkeys } from '@/hooks/ui/useHotkeys';
import { TrafficItem } from './TrafficItem';
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from '@dnd-kit/core';
import { arrayMove, SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { DebouncedInput } from '../ui/DebouncedInput';

interface TrafficListProps {
  items: Traffic[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onDelete?: (id: string) => void;
  onReorder?: (newIds: string[]) => void;
  activeColor?: 'emerald' | 'purple' | 'sky';
  layout?: 'sidebar' | 'table';
  isFocused?: boolean;
}

function SortableTrafficItem({ req, activeId, highlightedId, activeColor, onSelect, onDelete }: { req: Traffic, activeId: string | null, highlightedId: string | null, activeColor: 'emerald' | 'purple' | 'sky', onSelect: (id: string) => void, onDelete?: (id: string) => void }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: req.id });
  const style = { transform: CSS.Transform.toString(transform), transition, zIndex: isDragging ? 50 : undefined, opacity: isDragging ? 0.5 : 1 };
  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners} className="touch-none">
      <TrafficItem
        id={req.id}
        method={req.method}
        status={req.status_code}
        title={req.url}
        group={req.group}
        hitCount={req.hit_count}
        isIntercepted={req.is_intercepted}
        isActive={activeId === req.id}
        isHighlighted={highlightedId === req.id}
        activeColor={activeColor}
        onClick={onSelect}
        onDelete={onDelete}
      />
    </div>
  );
}

const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS', 'HEAD'];
const STATUS_FILTERS = ['2XX', '3XX', '4XX', '5XX', 'UNSENT'];

function getStatusCategory(status?: number): string {
  if (!status) return 'UNSENT';
  if (status < 300) return '2XX';
  if (status < 400) return '3XX';
  if (status < 500) return '4XX';
  return '5XX';
}

const getSafeHostname = (url: string, host: string) => {
  if (host) return host;
  try {
    return new URL(url).hostname;
  } catch {
    return '';
  }
};

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

export function TrafficList({ items, activeId, onSelect, onDelete, onReorder, activeColor = 'emerald', layout = 'sidebar', isFocused = true }: TrafficListProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [methodFilter, setMethodFilter] = useState<Record<string, FilterState>>({});
  const [statusFilter, setStatusFilter] = useState<Record<string, FilterState>>({});
  const [showFilters, setShowFilters] = useState(false);
  const [highlightedId, setHighlightedId] = useState<string | null>(activeId);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (activeId) setHighlightedId(activeId);
  }, [activeId]);

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 8 } }), useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }));

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (over && active.id !== over.id && onReorder) {
      const oldIndex = filteredItems.findIndex(i => i.id === active.id);
      const newIndex = filteredItems.findIndex(i => i.id === over.id);
      const newItems = arrayMove(filteredItems, oldIndex, newIndex);
      onReorder(newItems.map(i => i.id));
    }
  };

  const toggleMethod = (method: string) => setMethodFilter(prev => ({ ...prev, [method]: prev[method] === undefined ? 'include' : prev[method] === 'include' ? 'exclude' : undefined }));
  const toggleStatus = (status: string) => setStatusFilter(prev => ({ ...prev, [status]: prev[status] === undefined ? 'include' : prev[status] === 'include' ? 'exclude' : undefined }));

  const filteredItems = items.filter(req => {
    const matchesSearch = req.url.toLowerCase().includes(searchTerm.toLowerCase());

    const methodIncludes = Object.entries(methodFilter).filter(([_, state]) => state === 'include').map(([method]) => method);
    const methodExcludes = Object.entries(methodFilter).filter(([_, state]) => state === 'exclude').map(([method]) => method);
    let matchesMethod = true;
    if (methodIncludes.length > 0) matchesMethod = methodIncludes.includes(req.method.toUpperCase());
    if (matchesMethod && methodExcludes.length > 0) matchesMethod = !methodExcludes.includes(req.method.toUpperCase());

    const statusCat = getStatusCategory(req.status_code);
    const statusIncludes = Object.entries(statusFilter).filter(([_, state]) => state === 'include').map(([status]) => status);
    const statusExcludes = Object.entries(statusFilter).filter(([_, state]) => state === 'exclude').map(([status]) => status);
    let matchesStatus = true;
    if (statusIncludes.length > 0) matchesStatus = statusIncludes.includes(statusCat);
    if (matchesStatus && statusExcludes.length > 0) matchesStatus = !statusExcludes.includes(statusCat);

    return matchesSearch && matchesMethod && matchesStatus;
  });

  useHotkeys([
    {
      key: 's',
      enabled: isFocused,
      handler: () => searchInputRef.current?.focus(),
    },
    {
      key: 'ArrowDown',
      enabled: isFocused,
      ignoreInputs: false,
      handler: () => {
        if (filteredItems.length === 0) return;
        const currentIndex = filteredItems.findIndex(item => item.id === highlightedId);
        const nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % filteredItems.length;
        setHighlightedId(filteredItems[nextIndex].id);
      },
    },
    {
      key: 'ArrowUp',
      enabled: isFocused,
      ignoreInputs: false,
      handler: () => {
        if (filteredItems.length === 0) return;
        const currentIndex = filteredItems.findIndex(item => item.id === highlightedId);
        const nextIndex = currentIndex === -1 ? filteredItems.length - 1 : (currentIndex - 1 + filteredItems.length) % filteredItems.length;
        setHighlightedId(filteredItems[nextIndex].id);
      },
    },
    {
      key: 'Enter',
      enabled: isFocused,
      ignoreInputs: false,
      handler: (e) => {
        const target = e.target as HTMLElement;
        if (target.tagName === 'INPUT') {
          target.blur();
          e.stopPropagation();
        } else if (highlightedId) {
          onSelect(highlightedId);
        }
      }
    },
    {
      key: 'Escape',
      enabled: isFocused,
      ignoreInputs: false,
      handler: (e) => {
        const target = e.target as HTMLElement;
        if (target.tagName === 'INPUT') {
          target.blur();
          e.stopPropagation();
        }
      }
    }
  ], [filteredItems, highlightedId, onSelect, isFocused, activeId], false); // Use capture: false

  const activeBorder = activeColor === 'purple' ? 'border-l-purple-500' : activeColor === 'sky' ? 'border-l-sky-500' : 'border-l-emerald-500';

  return (
    <div className="flex flex-col h-full bg-zinc-950">

      {/* Search & Filter Bar */}
      <div className="p-3 border-b border-zinc-800 space-y-3 bg-zinc-900/30 shrink-0">
        <div className="flex items-center justify-between gap-2">
          <div className="relative flex-1">
            <DebouncedInput
              ref={searchInputRef}
              value={searchTerm}
              onChange={setSearchTerm}
              placeholder="Search..."
              className="w-full"
            />
          </div>
          <button onClick={() => setShowFilters(!showFilters)} className="px-2 py-1.5 text-[9px] uppercase font-bold tracking-widest rounded bg-zinc-900 text-zinc-400 border border-zinc-700 hover:text-zinc-200 hover:border-zinc-600 transition-all whitespace-nowrap">
            {showFilters ? 'Hide' : 'Show'} Filter
          </button>
        </div>

        {showFilters && (
          <div className={`flex ${layout === 'sidebar' ? 'flex-col gap-4' : 'gap-6'}`}>
            <div className="flex-1 w-full overflow-x-auto pb-2 scrollbar-thin scrollbar-thumb-zinc-800">
              <div className="text-[8px] uppercase text-zinc-600 font-black tracking-widest mb-1.5">Methods</div>
              <MultiSelectFilter options={METHODS} filterStates={methodFilter} onToggle={toggleMethod} onClear={() => setMethodFilter({})} />
            </div>
            <div className="flex-1 w-full overflow-x-auto pb-2 scrollbar-thin scrollbar-thumb-zinc-800">
              <div className="text-[8px] uppercase text-zinc-600 font-black tracking-widest mb-1.5">Response Status</div>
              <MultiSelectFilter options={STATUS_FILTERS} filterStates={statusFilter} onToggle={toggleStatus} onClear={() => setStatusFilter({})} />
            </div>
          </div>
        )}
      </div>

      {/* List / Table Render */}
      <div className="flex-1 overflow-auto relative">
        {filteredItems.length === 0 ? (
          <div className="p-4 text-center text-zinc-500 text-[10px] uppercase tracking-widest mt-4">No requests</div>
        ) : layout === 'table' ? (
          <table className="w-full text-left border-collapse whitespace-nowrap">
            <thead className="sticky top-0 bg-zinc-900 border-b border-zinc-800 text-[10px] text-zinc-400 uppercase tracking-widest z-10 shadow-sm">
              <tr>
                <th className="p-2 pl-4 font-bold">#</th>
                <th className="p-2 font-bold">Method</th>
                <th className="p-2 font-bold">Host</th>
                <th className="p-2 font-bold">Path</th>
                <th className="p-2 font-bold">Status</th>
                <th className="p-2 font-bold">Hits</th>
                <th className="p-2 pr-4 font-bold text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-800/50 text-xs font-mono text-zinc-300">
              {filteredItems.map((req, idx) => (
                <tr
                  key={req.id}
                  onClick={() => onSelect(req.id)}
                  className={`cursor-pointer hover:bg-zinc-800/50 transition-colors border-l-2 ${activeId === req.id ? `bg-zinc-800/80 ${activeBorder}` : 'border-l-transparent'} ${req.is_intercepted ? 'bg-rose-500/5' : ''}`}
                >
                  <td className="p-2 pl-4 text-zinc-600">{filteredItems.length - idx}</td>
                  <td className={`p-2 font-bold ${getMethodColor(req.method)}`}>{req.method}</td>
                  <td className="p-2 text-zinc-400">{getSafeHostname(req.url, req.host)}</td>
                  <td className="p-2 truncate max-w-xl" title={req.url}>{req.url}</td>
                  <td className={`p-2 font-bold ${getStatusColor(req.status_code)}`}>{req.status_code === 0 ? 'PENDING' : req.status_code}</td>
                  <td className="p-2 text-zinc-500">{req.hit_count || 0}</td>
                  <td className="p-2 pr-4 text-right">
                    {onDelete && (
                      <button
                        onClick={(e) => { e.stopPropagation(); onDelete(req.id); }}
                        className="p-1 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded transition-all"
                        title="Delete"
                      >
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="absolute inset-0 overflow-y-auto divide-y divide-zinc-800/50">
            {onReorder ? (
              <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                <SortableContext items={filteredItems.map(i => i.id)} strategy={verticalListSortingStrategy}>
                  {filteredItems.map(req => (
                    <SortableTrafficItem key={req.id} req={req} activeId={activeId} highlightedId={highlightedId} activeColor={activeColor} onSelect={onSelect} onDelete={onDelete} />
                  ))}
                </SortableContext>
              </DndContext>
            ) : (
              filteredItems.map(req => (
                <TrafficItem
                  key={req.id} 
                  id={req.id} 
                  method={req.method} 
                  status={req.status_code} 
                  title={req.url} 
                  group={req.group} 
                  hitCount={req.hit_count} 
                  isIntercepted={req.is_intercepted} 
                  isActive={activeId === req.id} 
                  isHighlighted={highlightedId === req.id}
                  activeColor={activeColor} 
                  onClick={(id) => {
                    setHighlightedId(id);
                    onSelect(id);
                  }} 
                  onDelete={onDelete}
                />
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}
