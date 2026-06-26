import { useState, useEffect, useCallback } from 'react';
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from '@dnd-kit/core';
import { arrayMove, SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useTraffic, RepeaterGroup, SyncData } from '@/hooks/traffic';
import { CollectionAssignmentModal } from '../../Modals';
import { Select } from '../../ui';
import { invoke } from '@/lib/utils/tauri';

interface CollectionsSectionProps {
  selectedGroupId: string | null;
  setSelectedGroupId: (id: string | null) => void;
  openPrompt: (title: string, initialValue: string, action: (val: string) => void) => void;
  openConfirm: (title: string, message: string, action: () => void) => void;
}

function SortableGroupItem({ group, isActive, onSelect, onRename, onDelete, onAssign, onClone }: { group: RepeaterGroup, isActive: boolean, onSelect: (id: string) => void, onRename: (group: RepeaterGroup) => void, onDelete: (group: RepeaterGroup) => void, onAssign: (group: RepeaterGroup) => void, onClone: (group: RepeaterGroup) => void }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: group.id });
  const style = { transform: CSS.Transform.toString(transform), transition, zIndex: isDragging ? 50 : undefined, opacity: isDragging ? 0.5 : 1 };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`group flex items-center justify-between p-3 rounded border transition-all cursor-pointer ${isActive ? 'bg-purple-500/10 border-purple-500/40 text-purple-400' : 'bg-zinc-900/50 border-zinc-800 text-zinc-400 hover:bg-zinc-800/50'}`}
      onClick={() => onSelect(group.id)}
    >
      <div className="flex items-center gap-3 overflow-hidden">
        <div {...attributes} {...listeners} className="cursor-grab active:cursor-grabbing p-1 hover:text-zinc-200">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><line x1="8" cy="6" x2="16" y2="6"></line><line x1="8" cy="12" x2="16" y2="12"></line><line x1="8" cy="18" x2="16" y2="18"></line></svg>
        </div>
        <span className="font-bold text-xs uppercase tracking-wider truncate">{group.name}</span>
      </div>
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button 
          onClick={(e) => { e.stopPropagation(); onAssign(group); }} 
          className="px-2 py-1 text-[8px] font-black uppercase tracking-widest border border-zinc-700 rounded hover:border-sky-500 hover:text-sky-400 transition-all"
        >
          Assign
        </button>
        <button 
          onClick={(e) => { e.stopPropagation(); onClone(group); }} 
          className="px-2 py-1 text-[8px] font-black uppercase tracking-widest border border-zinc-700 rounded hover:border-amber-500 hover:text-amber-400 transition-all"
        >
          Clone
        </button>
        <button onClick={(e) => { e.stopPropagation(); onRename(group); }} className="p-1.5 hover:text-purple-400"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg></button>
        <button onClick={(e) => { e.stopPropagation(); onDelete(group); }} className="p-1.5 hover:text-rose-500"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg></button>
      </div>
    </div>
  );
}

export function CollectionsSection({ selectedGroupId, setSelectedGroupId, openPrompt, openConfirm }: CollectionsSectionProps) {
  const { repeaterGroups, createGroup, renameGroup, deleteGroup, cloneGroup, reorderGroups } = useTraffic();
  const [localGroups, setLocalGroups] = useState<RepeaterGroup[]>([]);
  const [envFilter, setEnvFilter] = useState<'current' | 'all' | 'unassigned'>('all');
  const [assignModal, setAssignModal] = useState<{ isOpen: boolean, groupId: string | null, groupName: string }>({ isOpen: false, groupId: null, groupName: '' });

  const loadLocalGroups = useCallback(async () => {
    try {
      const data = await invoke<SyncData>('sync_data', {
        allGroups: envFilter === 'all',
        unassignedOnly: envFilter === 'unassigned'
      });
      setLocalGroups(data.repeaterGroups);
    } catch (e) {
      console.error('Failed to load collections with filter:', e);
    }
  }, [envFilter]);

  useEffect(() => {
    loadLocalGroups();
  }, [loadLocalGroups, repeaterGroups]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  const handleGroupReorder = async (event: DragEndEvent) => {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      const oldIndex = localGroups.findIndex(g => g.id === active.id);
      const newIndex = localGroups.findIndex(g => g.id === over.id);
      const newGroups = arrayMove(localGroups, oldIndex, newIndex);
      reorderGroups(newGroups.map(g => g.id));
    }
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
      <section className="space-y-4">
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <h3 className="text-purple-500 font-bold uppercase text-[10px] tracking-widest shrink-0">Collections Management</h3>
            <div className="flex items-center gap-1.5 bg-zinc-950 p-0.5 rounded-lg border border-zinc-800/80 pl-2">
              <span className="text-[8px] text-zinc-500 font-black uppercase tracking-wider">Filter:</span>
              <Select
                value={envFilter}
                onChange={(val) => setEnvFilter(val as any)}
                options={[
                  { value: 'all', label: 'All Collections' },
                  { value: 'current', label: 'Current Environment Only' },
                  { value: 'unassigned', label: 'Unassigned Only' }
                ]}
                className="w-48 !py-1 !px-2 border-none bg-transparent"
              />
            </div>
          </div>
          <button onClick={() => openPrompt('New Collection Name', '', createGroup)} className="px-3 py-1.5 bg-purple-600/10 border border-purple-600/30 text-purple-500 hover:bg-purple-600/20 rounded text-[9px] font-black uppercase tracking-widest transition-all">+ Create Collection</button>
        </div>
        
        <div className="grid grid-cols-1 gap-3">
          {localGroups.length === 0 ? (
            <div className="text-center py-12 border border-zinc-800 border-dashed rounded text-zinc-500 text-xs font-mono">
              No collections found in this view filter.
            </div>
          ) : (
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleGroupReorder}>
              <SortableContext items={localGroups.map(g => g.id)} strategy={verticalListSortingStrategy}>
                {localGroups.map(group => (
                  <SortableGroupItem
                    key={group.id}
                    group={group}
                    isActive={selectedGroupId === group.id}
                    onSelect={setSelectedGroupId}
                    onRename={(g: RepeaterGroup) => openPrompt('Rename Collection', g.name, (val) => renameGroup(g.id, val))}
                    onDelete={(g: RepeaterGroup) => openConfirm('Delete Collection', `Permanently destroy "${g.name}" and all requests inside?`, () => deleteGroup(g.id))}
                    onAssign={(g: RepeaterGroup) => setAssignModal({ isOpen: true, groupId: g.id, groupName: g.name })}
                    onClone={(g: RepeaterGroup) => openPrompt('Clone Collection', `${g.name} (Copy)`, (val) => cloneGroup(g.id, val))}
                  />
                ))}
              </SortableContext>
            </DndContext>
          )}
        </div>
      </section>

      <CollectionAssignmentModal 
        isOpen={assignModal.isOpen} 
        groupId={assignModal.groupId} 
        groupName={assignModal.groupName} 
        onClose={() => setAssignModal(prev => ({ ...prev, isOpen: false }))} 
      />
    </div>
  );
}
