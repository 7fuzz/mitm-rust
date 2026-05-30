import React from 'react';
import { useTraffic } from '@/hooks/traffic';
import { DebouncedInput } from '../../ui/DebouncedInput';
import { 
  DndContext, 
  closestCenter, 
  KeyboardSensor, 
  PointerSensor, 
  useSensor, 
  useSensors,
  DragEndEvent
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { restrictToVerticalAxis } from '@dnd-kit/modifiers';
import { VariableItem } from './VariableItem';
import { EnvironmentEditorModal } from '../../Modals';

interface EnvironmentsSectionProps {
  openPrompt: (title: string, initialValue: string, action: (val: string) => void) => void;
  openConfirm: (title: string, message: string, action: () => void) => void;
}

export function EnvironmentsSection({ openPrompt, openConfirm }: EnvironmentsSectionProps) {
  const {
    environments, activeEnvId, setActiveEnvironment, createEnvironment, renameEnvironment, deleteEnvironment,
    variables, addVariable, updateVariable, deleteVariable, saveVariable, saveAllVariables, reorderVariables,
    prefs, updatePrefs
  } = useTraffic();

  const [isSavingAll, setIsSavingAll] = React.useState(false);
  const [saveMessage, setSaveMessage] = React.useState('');
  const [editModal, setEditModal] = React.useState<{ isOpen: boolean, envId: string | null, envName: string }>({ 
    isOpen: false, envId: null, envName: '' 
  });

  const envVariables = React.useMemo(() => 
    variables.filter(v => v.environmentId === activeEnvId),
    [variables, activeEnvId]
  );

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      const oldIndex = envVariables.findIndex((v) => v.id === active.id);
      const newIndex = envVariables.findIndex((v) => v.id === over.id);
      const reordered = arrayMove(envVariables, oldIndex, newIndex);
      reorderVariables(reordered);
    }
  };

  const handleSaveAll = async () => {
    setIsSavingAll(true);
    setSaveMessage('Saving variables...');
    try {
      await saveAllVariables();
      setSaveMessage('All variables saved ✓');
    } catch (e) {
      setSaveMessage('Failed to save variables');
    }
    setIsSavingAll(false);
    setTimeout(() => setSaveMessage(''), 3000);
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h3 className="text-amber-500 font-bold uppercase text-[10px] tracking-widest">Available Environments</h3>
            <div className="flex items-center gap-2 px-2 py-1 bg-zinc-900/50 border border-zinc-800 rounded">
              <span className="text-[8px] font-black uppercase tracking-widest text-zinc-500">Auto-Save</span>
              <button 
                onClick={() => updatePrefs({ ...prefs, autoSave: !prefs.autoSave })}
                className={`w-8 h-4 rounded-full relative transition-colors ${prefs.autoSave ? 'bg-emerald-500/50' : 'bg-zinc-800'}`}
              >
                <div className={`absolute top-1 w-2 h-2 rounded-full transition-all ${prefs.autoSave ? 'right-1 bg-emerald-400' : 'left-1 bg-zinc-600'}`} />
              </button>
            </div>
          </div>
          <button onClick={() => openPrompt('New Environment Name', '', createEnvironment)} className="px-3 py-1.5 bg-amber-600/10 border border-amber-600/30 text-amber-500 hover:bg-amber-600/20 rounded text-[9px] font-black uppercase tracking-widest transition-all">+ Create Environment</button>
        </div>
        <div className="grid grid-cols-3 gap-4">
          {environments.map(env => (
            <div key={env.id} className={`p-4 rounded border transition-all flex flex-col gap-3 ${activeEnvId === env.id ? 'bg-amber-500/5 border-amber-500/40 shadow-lg shadow-amber-900/10' : 'bg-zinc-900/30 border-zinc-800'}`}>
              <div className="flex items-center justify-between">
                <span className={`text-xs font-bold uppercase tracking-wider ${activeEnvId === env.id ? 'text-amber-400' : 'text-zinc-400'}`}>{env.name}</span>
                <div className="flex items-center gap-1">
                  <button 
                    onClick={() => setEditModal({ isOpen: true, envId: env.id, envName: env.name })} 
                    className="p-1 text-zinc-600 hover:text-zinc-300"
                    title="Edit Environment"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
                  </button>
                  {env.id !== 'default-env-id' && <button onClick={() => openConfirm('Delete Environment', `Are you sure? All variables in "${env.name}" will be lost.`, () => deleteEnvironment(env.id))} className="p-1 text-zinc-600 hover:text-rose-500"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg></button>}
                </div>
              </div>
              <button 
                onClick={() => setActiveEnvironment(env.id)}
                disabled={activeEnvId === env.id}
                className={`w-full py-1.5 rounded text-[9px] font-black uppercase tracking-[0.2em] border transition-all ${activeEnvId === env.id ? 'bg-amber-500 border-amber-400 text-zinc-950' : 'bg-zinc-950 border-zinc-800 text-zinc-600 hover:text-zinc-300 hover:border-zinc-700'}`}
              >
                {activeEnvId === env.id ? 'Current_Active' : 'Activate'}
              </button>
            </div>
          ))}
        </div>
      </section>

      <section className="space-y-4 pt-8 border-t border-zinc-800">
        <div className="flex items-center justify-between">
          <h3 className="text-emerald-text font-bold uppercase text-[10px] tracking-widest">Global Variables ({environments.find(e => e.id === activeEnvId)?.name})</h3>
          <button 
            onClick={() => addVariable({ 
              id: crypto.randomUUID(), 
              environmentId: activeEnvId, 
              name: '', 
              values: [
                { id: crypto.randomUUID(), name: 'Default', value: '' },
                { id: crypto.randomUUID(), name: '(auto)', value: '' }
              ], 
              activeIndex: 0 
            })}
            className="px-3 py-1.5 bg-emerald-600/10 border border-emerald-600/30 text-emerald-text hover:bg-emerald-600/20 rounded text-[9px] font-black uppercase tracking-widest transition-all"
          >
            + Add New Variable
          </button>
        </div>
        
        <div className="bg-zinc-950 border border-zinc-800 rounded-lg divide-y divide-zinc-800 overflow-hidden">
          {envVariables.length === 0 ? (
            <div className="p-12 text-center text-zinc-600 font-mono text-[10px] uppercase tracking-widest">No variables defined for this environment</div>
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
              modifiers={[restrictToVerticalAxis]}
            >
              <SortableContext
                items={envVariables.map(v => v.id)}
                strategy={verticalListSortingStrategy}
              >
                {envVariables.map(v => (
                  <VariableItem 
                    key={v.id} 
                    v={v} 
                    activeEnvId={activeEnvId} 
                    autoSave={prefs.autoSave}
                    updateVariable={updateVariable}
                    saveVariable={saveVariable}
                    deleteVariable={deleteVariable}
                  />
                ))}
              </SortableContext>
            </DndContext>
          )}
        </div>

        {!prefs.autoSave && envVariables.length > 0 && (
          <div className="flex items-center justify-between pt-4">
            <span className="text-[10px] font-mono uppercase tracking-widest text-emerald-text">{saveMessage}</span>
            <button 
              onClick={handleSaveAll}
              disabled={isSavingAll}
              className="px-6 py-2 bg-emerald-600 hover:bg-emerald-500 text-zinc-950 text-[10px] rounded uppercase font-black tracking-widest transition-colors disabled:opacity-50"
            >
              {isSavingAll ? 'Saving...' : 'Save All Variables'}
            </button>
          </div>
        )}
      </section>

      <EnvironmentEditorModal 
        isOpen={editModal.isOpen} 
        envId={editModal.envId} 
        envName={editModal.envName} 
        onClose={() => setEditModal(prev => ({ ...prev, isOpen: false }))} 
      />
    </div>
  );
}
