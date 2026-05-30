import { useState, useEffect, useRef, useCallback } from 'react';
import { useTraffic, ReplacementCategory, ReplacementEntry } from '@/hooks/traffic';
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from '@dnd-kit/core';
import { arrayMove, SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { DebouncedInput } from '../../ui/DebouncedInput';
import { Button } from '../../ui';

const CATEGORY_INFO: Record<ReplacementCategory, { label: string; description: string; color: string }> = {
  URL_REPLACEMENTS: { label: 'URL Patterns', description: 'Domain prefix replacements for environment switching', color: 'sky' },
  HEADER_REPLACEMENTS: { label: 'Header Replacements', description: 'Replace whole header values based on their KEY (e.g., Authorization)', color: 'emerald' },
  BODY_KEY_REPLACEMENTS: { label: 'Body Keys', description: 'Replace JSON/Form keys in request body', color: 'amber' },
  URL_PARAM_REPLACEMENTS: { label: 'URL Params', description: 'Replace URL query parameter values', color: 'rose' },
  TEXT_REPLACEMENTS: { label: 'Global Text', description: 'Global string replacement across URL, Headers, and Body (e.g., xyz -> {{var}})', color: 'purple' },
};

function SortableReplacementItem({
  category,
  entry,
  idx,
  updateEntry,
  removeEntry
}: {
  category: ReplacementCategory;
  entry: ReplacementEntry;
  idx: number;
  updateEntry: (category: ReplacementCategory, index: number, field: 'pattern' | 'replacement' | 'is_active', value: string | boolean) => void;
  removeEntry: (category: ReplacementCategory, index: number) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: entry.id });
  const style = { transform: CSS.Transform.toString(transform), transition, zIndex: isDragging ? 50 : undefined, opacity: isDragging ? 0.5 : 1 };

  return (
    <div ref={setNodeRef} style={style} className="flex items-center gap-2 group">
      <div {...attributes} {...listeners} className="cursor-grab active:cursor-grabbing p-1 text-zinc-600 hover:text-zinc-400">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><line x1="8" y1="6" x2="16" y2="6"></line><line x1="8" y1="12" x2="16" y2="12"></line><line x1="8" y1="18" x2="16" y2="18"></line></svg>
      </div>
      <button
        onClick={() => updateEntry(category, idx, 'is_active', !entry.is_active)}
        className={`p-2 rounded transition-colors ${entry.is_active ? 'text-emerald-text bg-emerald-500/10' : 'text-zinc-600 hover:text-zinc-400 bg-input-bg'}`}
        title={entry.is_active ? "Rule is Active" : "Rule is Inactive"}
      >
        {entry.is_active ? (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
        ) : (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        )}
      </button>
      <DebouncedInput
        value={entry.pattern}
        onChange={(val) => updateEntry(category, idx, 'pattern', val)}
        placeholder={category === 'HEADER_REPLACEMENTS' ? "Header Key (e.g. Authorization)" : category === 'BODY_KEY_REPLACEMENTS' ? "JSON Key (e.g. user_id)" : category === 'TEXT_REPLACEMENTS' ? "Any Text (e.g. xyz)" : "Pattern (e.g. api.)"}
        className={`flex-1 transition-all`}
        inputClassName={`font-mono text-xs !px-1 ${!entry.is_active ? 'text-zinc-600 opacity-50' : ''}`}
        showIcon={false}
      />
      <span className="text-zinc-600 text-xs">→</span>
      <DebouncedInput
        value={entry.replacement}
        onChange={(val) => updateEntry(category, idx, 'replacement', val)}
        placeholder={category === 'HEADER_REPLACEMENTS' ? "Value (e.g. Bearer {{token}})" : category === 'TEXT_REPLACEMENTS' ? "Replacement (e.g. {{something}})" : "Replacement (e.g. {{env}})"}
        className={`flex-1 transition-all`}
        inputClassName={`font-mono text-xs !px-1 ${!entry.is_active ? 'text-zinc-600 opacity-50' : 'text-amber-500'}`}
        showIcon={false}
      />
      <button
        onClick={() => removeEntry(category, idx)}
        className="p-2 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded transition-colors opacity-0 group-hover:opacity-100"
      >
        ✕
      </button>
    </div>
  );
}

export function ReplacementsSection() {
  const {
    orderedReplacements, saveReplacements, deleteReplacement, isLoading: replacementsLoading,
    prefs, updatePrefs, simpleMode
  } = useTraffic();

  const autoSaveEnabled = prefs.replacementsAutoSave !== false;

  const [expandedCategory, setExpandedCategory] = useState<ReplacementCategory | null>('URL_REPLACEMENTS');
  const [localReplacements, setLocalReplacements] = useState<Record<ReplacementCategory, ReplacementEntry[]>>({
    URL_REPLACEMENTS: [],
    HEADER_REPLACEMENTS: [],
    BODY_KEY_REPLACEMENTS: [],
    URL_PARAM_REPLACEMENTS: [],
    TEXT_REPLACEMENTS: [],
  });
  const [isSaving, setIsSaving] = useState(false);
  const [saveMessage, setSaveMessage] = useState('');

  const debounceRef = useRef<NodeJS.Timeout | null>(null);
  const isInitialLoad = useRef(true);
  const isSyncingRef = useRef(false);
  const lastSavedRef = useRef<string>('');

  const [prevOrdered, setPrevOrdered] = useState<ReplacementEntry[] | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  if (orderedReplacements !== prevOrdered) {
    setPrevOrdered(orderedReplacements);
    const converted: Record<ReplacementCategory, ReplacementEntry[]> = {
      URL_REPLACEMENTS: orderedReplacements.filter(r => r.type === 'URL_REPLACEMENTS').map(r => ({ id: r.id, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })),
      HEADER_REPLACEMENTS: orderedReplacements.filter(r => r.type === 'HEADER_REPLACEMENTS').map(r => ({ id: r.id, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })),
      BODY_KEY_REPLACEMENTS: orderedReplacements.filter(r => r.type === 'BODY_KEY_REPLACEMENTS').map(r => ({ id: r.id, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })),
      URL_PARAM_REPLACEMENTS: orderedReplacements.filter(r => r.type === 'URL_PARAM_REPLACEMENTS').map(r => ({ id: r.id, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })),
      TEXT_REPLACEMENTS: orderedReplacements.filter(r => r.type === 'TEXT_REPLACEMENTS').map(r => ({ id: r.id, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })),
    };
    
    isSyncingRef.current = true;
    setLocalReplacements(converted);
    
    const sortedItems = [...orderedReplacements].sort((a, b) => {
      if (a.type !== b.type) return a.type.localeCompare(b.type);
      return (a.order_index || 0) - (b.order_index || 0);
    });
    const payloadString = JSON.stringify(sortedItems.map(r => ({ id: r.id, type: r.type, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })));
    lastSavedRef.current = payloadString;
  }

  useEffect(() => {
    if (!replacementsLoading) {
      isInitialLoad.current = false;
    }
  }, [replacementsLoading]);

  const saveRef = useRef(saveReplacements);
  useEffect(() => {
    saveRef.current = saveReplacements;
  }, [saveReplacements]);

  const debouncedSave = useCallback((data: Record<ReplacementCategory, ReplacementEntry[]>) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);

    const currentItems = (Object.entries(data) as [ReplacementCategory, ReplacementEntry[]][]).flatMap(([type, entries]) =>
      entries.filter(e => e.pattern).map((e, index) => ({ ...e, type, order_index: index }))
    );

    const sortedCurrent = [...currentItems].sort((a, b) => {
      if (a.type !== b.type) return a.type.localeCompare(b.type);
      return (a.order_index || 0) - (b.order_index || 0);
    });

    const currentPayloadString = JSON.stringify(sortedCurrent.map(r => ({ id: r.id, type: r.type, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })));
    
    if (isSyncingRef.current) {
      isSyncingRef.current = false;
      return;
    }

    if (lastSavedRef.current === currentPayloadString) return;

    const lastItems: { id: string; pattern: string; replacement: string, is_active: boolean }[] = JSON.parse(lastSavedRef.current || '[]');
    const modifiedItems = currentItems.filter(curr => {
      const prev = lastItems.find(l => l.id === curr.id);
      return !prev || prev.pattern !== curr.pattern || prev.replacement !== curr.replacement || prev.is_active !== curr.is_active;
    });

    if (modifiedItems.length === 0 && currentItems.length === lastItems.length) {
      return;
    }

    setSaveMessage('Saving...');
    debounceRef.current = setTimeout(async () => {
      setIsSaving(true);
      try {
        const result = await saveRef.current(currentItems, true);
        if (result.success) {
          lastSavedRef.current = currentPayloadString;
          setSaveMessage('Auto-saved ✓');
        } else {
          setSaveMessage('Save failed');
        }
      } catch (_e) {
        setSaveMessage('Save failed');
      }
      setIsSaving(false);
      setTimeout(() => setSaveMessage(''), 2000);
    }, 800);
  }, []);

  useEffect(() => {
    if (!isInitialLoad.current && autoSaveEnabled) {
      debouncedSave(localReplacements);
    }
  }, [localReplacements, debouncedSave, autoSaveEnabled]);

  const updateEntry = (category: ReplacementCategory, index: number, field: 'pattern' | 'replacement' | 'is_active', value: string | boolean) => {
    isSyncingRef.current = false;
    setLocalReplacements(prev => ({
      ...prev,
      [category]: prev[category].map((entry, i) => i === index ? { ...entry, [field]: value } : entry)
    }));
  };

  const addEntry = (category: ReplacementCategory) => {
    isSyncingRef.current = false;
    setLocalReplacements(prev => ({
      ...prev,
      [category]: [...prev[category], { id: crypto.randomUUID(), pattern: '', replacement: '', is_active: true }]
    }));
  };

  const removeEntry = (category: ReplacementCategory, index: number) => {
    isSyncingRef.current = false;
    const entry = localReplacements[category][index];
    if (entry && entry.id) {
      deleteReplacement(entry.id);
    }
    setLocalReplacements(prev => ({
      ...prev,
      [category]: prev[category].filter((_, i) => i !== index)
    }));
  };

  const handleReorder = (category: ReplacementCategory, event: DragEndEvent) => {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      isSyncingRef.current = false;
      setLocalReplacements(prev => {
        const items = prev[category];
        const oldIndex = items.findIndex(i => i.id === active.id);
        const newIndex = items.findIndex(i => i.id === over.id);
        return {
          ...prev,
          [category]: arrayMove(items, oldIndex, newIndex)
        };
      });
    }
  };

  const handleManualSave = async () => {
    setIsSaving(true);
    setSaveMessage('');
    try {
      const allItems = (Object.entries(localReplacements) as [ReplacementCategory, ReplacementEntry[]][]).flatMap(([type, entries]) =>
        entries.filter(e => e.pattern).map((e, index) => ({ ...e, type, order_index: index }))
      );

      const result = await saveReplacements(allItems, true);
      if (result.success) {
        setSaveMessage('Replacements updated successfully!');
        const sortedItems = [...allItems].sort((a, b) => {
          if (a.type !== b.type) return a.type.localeCompare(b.type);
          return (a.order_index || 0) - (b.order_index || 0);
        });
        lastSavedRef.current = JSON.stringify(sortedItems.map(r => ({ id: r.id, type: r.type, pattern: r.pattern, replacement: r.replacement, is_active: r.is_active })));
      } else {
        setSaveMessage('Error: Failed to save replacements');
      }
    } catch (_e) {
      setSaveMessage('Error: Failed to save replacements');
    }
    setIsSaving(false);
    setTimeout(() => setSaveMessage(''), 3000);
  };

  if (replacementsLoading) {
    return <div className="text-center py-12 border border-zinc-800 border-dashed rounded bg-zinc-900/20 text-zinc-600 font-mono text-[10px] uppercase tracking-widest">Loading replacements...</div>;
  }

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
      <section className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-rose-500 font-bold uppercase text-[10px] tracking-widest">Repeater Replacements</h3>
            <p className="text-zinc-500 text-[10px] font-mono mt-1">Configure variable placeholders applied when sending requests from History to Repeater.</p>
          </div>
          <div className="flex items-center gap-2 px-2 py-1 bg-zinc-900/50 border border-zinc-800 rounded">
            <span className="text-[8px] font-black uppercase tracking-widest text-zinc-500">Auto-Save</span>
            <button 
              onClick={() => updatePrefs({ ...prefs, replacementsAutoSave: !autoSaveEnabled })}
              className={`w-8 h-4 rounded-full relative transition-colors ${autoSaveEnabled ? 'bg-rose-500/50' : 'bg-zinc-800'}`}
            >
              <div className={`absolute top-1 w-2 h-2 rounded-full transition-all ${autoSaveEnabled ? 'right-1 bg-rose-400' : 'left-1 bg-zinc-600'}`} />
            </button>
          </div>
        </div>

        <div className="space-y-3">
          {(Object.keys(CATEGORY_INFO) as ReplacementCategory[]).map(category => {
            const info = CATEGORY_INFO[category];
            const isExpanded = expandedCategory === category;
            const entries = localReplacements[category] || [];

            return (
              <div key={category} className="border border-zinc-800 rounded overflow-hidden bg-zinc-900/30">
                <button
                  onClick={() => setExpandedCategory(isExpanded ? null : category)}
                  className="w-full flex items-center justify-between p-4 bg-zinc-900/50 hover:bg-zinc-800/50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <span className={`w-2 h-2 rounded-full bg-${info.color}-500`} />
                    <div className="text-left">
                      <div className="text-xs text-zinc-300 font-bold uppercase tracking-widest">{info.label}</div>
                      <div className="text-[10px] text-zinc-600 font-mono">{entries.length} rule{entries.length !== 1 ? 's' : ''}</div>
                    </div>
                  </div>
                  <span className={`text-zinc-500 transition-transform ${isExpanded ? 'rotate-180' : ''}`}>▼</span>
                </button>

                {isExpanded && (
                  <div className="p-4 border-t border-zinc-800 space-y-3 bg-zinc-950/30">
                    <p className="text-[10px] text-zinc-500 font-mono italic">{info.description}</p>

                    {entries.length === 0 ? (
                      <div className="text-center py-6 border border-zinc-800 border-dashed rounded text-zinc-700 text-[10px] font-mono uppercase tracking-widest">No replacements configured</div>
                    ) : (
                      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={(e) => handleReorder(category, e)}>
                        <SortableContext items={entries.map(e => e.id)} strategy={verticalListSortingStrategy}>
                          <div className="space-y-2">
                            {entries.map((entry, idx) => (
                              <SortableReplacementItem
                                key={entry.id}
                                category={category}
                                entry={entry}
                                idx={idx}
                                updateEntry={updateEntry}
                                removeEntry={removeEntry}
                              />
                            ))}
                          </div>
                        </SortableContext>
                      </DndContext>
                    )}

                    <button
                      onClick={() => addEntry(category)}
                      className="text-[10px] text-rose-400 hover:text-rose-300 font-bold uppercase tracking-widest flex items-center gap-1 mt-2"
                    >
                      + Add Rule
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="flex items-center justify-between pt-6 border-t border-zinc-800/50">
          <span className={`text-[10px] font-mono uppercase tracking-widest ${saveMessage.includes('Error') ? 'text-rose-400' : 'text-emerald-text'}`}>
            {saveMessage}
          </span>
          <Button
            variant="destructive"
            size="md"
            onClick={handleManualSave}
            disabled={isSaving}
            className="bg-rose-600 hover:bg-rose-500 text-zinc-950 px-8"
          >
            {isSaving ? 'Saving...' : 'Save Configuration'}
          </Button>
        </div>
      </section>
    </div>
  );
}
