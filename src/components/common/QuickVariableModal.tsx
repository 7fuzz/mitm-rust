import React, { useState, useEffect, useRef } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { MingCuteIcon } from './MingCuteIcon';
import type { EnvironmentVariable, VariableVariant } from '../../services/tauri/bridge';

type InlineEditType = 'value' | 'rename-var' | 'rename-variant' | 'new-variant';

export const QuickVariableModal: React.FC = () => {
  const { isQuickVarModalOpen, setQuickVarModalOpen } = useSettingsStore();
  const {
    activeEnvironmentId,
    environmentsList,
    environments,
    saveEnvironmentVariables,
    setActiveEnv,
  } = useWorkspaceStore();

  const [search, setSearch] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [isSecret, setIsSecret] = useState(false);

  const [inlineEdit, setInlineEdit] = useState<{
    type: InlineEditType;
    varIndex: number;
    value: string;
    extraValue?: string;
  } | null>(null);

  const listRef = useRef<HTMLDivElement>(null);
  const editInputRef = useRef<HTMLInputElement>(null);

  const currentEnvFromList = environmentsList.find((e) => e.id === activeEnvironmentId || e.isActive) || environmentsList[0];
  const currentEnvFromLegacy = environments.find((e) => e.id === activeEnvironmentId);
  const currentEnvName = currentEnvFromList?.name || currentEnvFromLegacy?.name || 'Environment';

  const activeVars = currentEnvFromList?.variables || [];

  const filteredVars = activeVars.filter((v) => {
    const keyStr = typeof v.key === 'string' ? v.key : String(v.key || '');
    const valStr = typeof v.value === 'string' ? v.value : String(v.value || '');
    return (
      keyStr.toLowerCase().includes(search.toLowerCase()) ||
      valStr.toLowerCase().includes(search.toLowerCase())
    );
  });

  useEffect(() => {
    if (isQuickVarModalOpen) {
      setSelectedIndex(0);
      setSearch('');
      setInlineEdit(null);
    }
  }, [isQuickVarModalOpen]);

  useEffect(() => {
    if (inlineEdit) {
      setTimeout(() => editInputRef.current?.focus(), 20);
    }
  }, [inlineEdit]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setQuickVarModalOpen(!isQuickVarModalOpen);
      }
      if (e.key === 'Escape' && isQuickVarModalOpen) {
        if (inlineEdit) {
          setInlineEdit(null);
        } else {
          setQuickVarModalOpen(false);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isQuickVarModalOpen, inlineEdit, setQuickVarModalOpen]);

  if (!isQuickVarModalOpen) return null;

  const handleSelectEnvironment = async (envId: string) => {
    const targetInList = environmentsList.find((e) => e.id === envId);
    if (targetInList) {
      await saveEnvironmentVariables({ ...targetInList, isActive: true, updatedAtMs: Date.now() });
    } else {
      await setActiveEnv(envId);
    }
  };

  const getVariants = (v: EnvironmentVariable): VariableVariant[] => {
    if (v.variants && v.variants.length > 0) {
      return v.variants;
    }
    return [{ name: '(auto)', value: v.value || '' }];
  };

  const handleSaveInline = async () => {
    if (!inlineEdit || !currentEnvFromList) return;
    const { type, varIndex, value, extraValue } = inlineEdit;
    const targetVar = filteredVars[varIndex];
    if (!targetVar) return;

    const realIndex = activeVars.findIndex((v) => v.key === targetVar.key);
    if (realIndex === -1) return;

    const updatedVars = [...activeVars];
    const currentVar = { ...updatedVars[realIndex] };
    const variants = getVariants(currentVar);
    const activeIdx = currentVar.activeIndex || 0;

    if (type === 'value') {
      const updatedVariants = variants.map((v, idx) =>
        idx === activeIdx ? { ...v, value } : v
      );
      updatedVars[realIndex] = {
        ...currentVar,
        value,
        variants: updatedVariants,
      };
    } else if (type === 'rename-var') {
      if (value.trim()) {
        updatedVars[realIndex] = {
          ...currentVar,
          key: value.trim().toUpperCase(),
        };
      }
    } else if (type === 'rename-variant') {
      if (value.trim()) {
        const updatedVariants = variants.map((v, idx) =>
          idx === activeIdx ? { ...v, name: value.trim() } : v
        );
        updatedVars[realIndex] = {
          ...currentVar,
          variants: updatedVariants,
        };
      }
    } else if (type === 'new-variant') {
      if (value.trim()) {
        const updatedVariants = [
          ...variants,
          { name: value.trim(), value: extraValue || '' },
        ];
        updatedVars[realIndex] = {
          ...currentVar,
          variants: updatedVariants,
          activeIndex: updatedVariants.length - 1,
          value: extraValue || '',
        };
      }
    }

    const updatedEnv = {
      ...currentEnvFromList,
      variables: updatedVars,
      updatedAtMs: Date.now(),
    };

    await saveEnvironmentVariables(updatedEnv);
    setInlineEdit(null);
  };

  const handleDeleteVariable = async (varKey: string) => {
    if (!currentEnvFromList) return;
    const updatedVars = activeVars.filter((v) => v.key !== varKey);
    await saveEnvironmentVariables({
      ...currentEnvFromList,
      variables: updatedVars,
      updatedAtMs: Date.now(),
    });
  };

  const handleDeleteVariant = async (varIndex: number) => {
    if (!currentEnvFromList) return;
    const targetVar = filteredVars[varIndex];
    if (!targetVar) return;

    const realIndex = activeVars.findIndex((v) => v.key === targetVar.key);
    if (realIndex === -1) return;

    const currentVar = activeVars[realIndex];
    const variants = getVariants(currentVar);
    if (variants.length <= 1) return;

    const activeIdx = currentVar.activeIndex || 0;
    const updatedVariants = variants.filter((_, idx) => idx !== activeIdx);
    const newActiveIdx = Math.max(0, activeIdx - 1);

    const updatedVars = [...activeVars];
    updatedVars[realIndex] = {
      ...currentVar,
      variants: updatedVariants,
      activeIndex: newActiveIdx,
      value: updatedVariants[newActiveIdx]?.value || '',
    };

    await saveEnvironmentVariables({
      ...currentEnvFromList,
      variables: updatedVars,
      updatedAtMs: Date.now(),
    });
  };

  const handleCycleVariant = async (varIndex: number, direction: 'next' | 'prev' = 'next') => {
    if (!currentEnvFromList) return;
    const targetVar = filteredVars[varIndex];
    if (!targetVar) return;

    const realIndex = activeVars.findIndex((v) => v.key === targetVar.key);
    if (realIndex === -1) return;

    const currentVar = activeVars[realIndex];
    const variants = getVariants(currentVar);
    if (variants.length <= 1) return;

    const currentIdx = currentVar.activeIndex || 0;
    const nextIdx =
      direction === 'next'
        ? (currentIdx + 1) % variants.length
        : (currentIdx - 1 + variants.length) % variants.length;

    const nextVariant = variants[nextIdx];
    const updatedVars = [...activeVars];
    updatedVars[realIndex] = {
      ...currentVar,
      activeIndex: nextIdx,
      value: nextVariant.value,
    };

    await saveEnvironmentVariables({
      ...currentEnvFromList,
      variables: updatedVars,
      updatedAtMs: Date.now(),
    });
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKey.trim() || !currentEnvFromList) return;
    const initialVal = newValue;
    const newVar: EnvironmentVariable = {
      key: newKey.trim().toUpperCase(),
      value: initialVal,
      enabled: true,
      type: isSecret ? 'secret' : 'default',
      activeIndex: 0,
      variants: [{ name: '(auto)', value: initialVal }],
    };
    const updatedEnv = {
      ...currentEnvFromList,
      variables: [...(currentEnvFromList.variables || []), newVar],
      updatedAtMs: Date.now(),
    };
    await saveEnvironmentVariables(updatedEnv);
    setNewKey('');
    setNewValue('');
  };

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          setQuickVarModalOpen(false);
        }
      }}
      className="fixed inset-0 z-50 flex items-start justify-center pt-12 bg-black/60 backdrop-blur-xs animate-in fade-in duration-150 font-sans text-xs select-none cursor-pointer"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-xl bg-surface border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-[88vh] text-foreground cursor-default select-text"
      >
        {/* Modal Header */}
        <div className="p-3 border-b border-border flex items-center justify-between bg-header">
          <div className="flex items-center gap-2 font-semibold text-sm">
            <MingCuteIcon name="earth_line" size={18} className="text-primary" />
            <span>Quick Environment & Variable Switcher</span>
          </div>
          <button
            onClick={() => setQuickVarModalOpen(false)}
            className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors cursor-pointer"
          >
            <MingCuteIcon name="close_line" size={16} />
          </button>
        </div>

        {/* Environment Selector Bar */}
        <div className="p-3 bg-background border-b border-border flex flex-col gap-2">
          <label className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider block font-mono">
            Active Workspace Environments
          </label>
          <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar py-0.5">
            {environmentsList.length > 0
              ? environmentsList.map((env) => {
                  const isActive = env.id === activeEnvironmentId || env.isActive;
                  return (
                    <button
                      key={env.id}
                      onClick={() => handleSelectEnvironment(env.id)}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold cursor-pointer transition-all shrink-0 ${
                        isActive
                          ? 'bg-primary text-primary-foreground border-primary shadow-2xs font-bold'
                          : 'bg-surface hover:bg-neutral-subtle border-border text-foreground'
                      }`}
                    >
                      <MingCuteIcon name="earth_line" size={13} />
                      <span>{env.name}</span>
                      {isActive && (
                        <span className="bg-emerald-500 text-white text-[9px] px-1 py-0.2 rounded font-mono font-bold">
                          ACTIVE
                        </span>
                      )}
                    </button>
                  );
                })
              : environments.map((env) => {
                  const isActive = env.id === activeEnvironmentId;
                  return (
                    <button
                      key={env.id}
                      onClick={() => handleSelectEnvironment(env.id)}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold cursor-pointer transition-all shrink-0 ${
                        isActive
                          ? 'bg-primary text-primary-foreground border-primary shadow-2xs font-bold'
                          : 'bg-surface hover:bg-neutral-subtle border-border text-foreground'
                      }`}
                    >
                      <span>{env.name}</span>
                      {isActive && (
                        <span className="bg-emerald-500 text-white text-[9px] px-1 py-0.2 rounded font-mono font-bold">
                          ACTIVE
                        </span>
                      )}
                    </button>
                  );
                })}
          </div>
        </div>

        {/* Filter Input Bar */}
        <div className="p-3 border-b border-border flex items-center gap-2 bg-header font-mono">
          <MingCuteIcon name="search_line" className="text-muted-foreground" size={16} />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={`Filter variables in ${currentEnvName} (e.g. {{AUTH_TOKEN}})...`}
            className="w-full bg-transparent text-xs focus:outline-none text-foreground placeholder:text-muted-foreground font-mono"
            autoFocus
          />
        </div>

        {/* Variables List */}
        <div ref={listRef} className="p-3 overflow-y-auto space-y-2 flex-1 custom-scrollbar">
          {filteredVars.length === 0 ? (
            <div className="py-8 text-center text-xs text-muted-foreground italic font-sans">
              No variables in "{currentEnvName}"
            </div>
          ) : (
            filteredVars.map((v, idx) => {
              const keyStr = typeof v.key === 'string' ? v.key : String(v.key || '');
              const isSecretVal = v.type === 'secret';
              const isSelected = selectedIndex === idx;
              const variants = getVariants(v);
              const activeVarIdx = v.activeIndex || 0;
              const activeVariant = variants[activeVarIdx] || variants[0] || { name: '(auto)', value: v.value };
              const valStr = typeof activeVariant.value === 'string' ? activeVariant.value : String(activeVariant.value || '');

              const isEditingThis = isSelected && inlineEdit && inlineEdit.varIndex === idx;

              return (
                <div
                  key={idx}
                  onClick={() => setSelectedIndex(idx)}
                  className={`p-2.5 rounded-lg border transition-all flex flex-col gap-2 cursor-pointer ${
                    isSelected
                      ? 'bg-primary/10 border-primary/50 shadow-2xs'
                      : 'bg-background/50 border-border/60 hover:bg-neutral-subtle/50'
                  }`}
                >
                  {/* Top Line: Key & Variant Info */}
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 font-mono text-xs overflow-hidden">
                      {isEditingThis && inlineEdit.type === 'rename-var' ? (
                        <div className="flex items-center gap-1">
                          <span className="text-[10px] text-primary font-bold">KEY:</span>
                          <input
                            ref={editInputRef}
                            type="text"
                            value={inlineEdit.value}
                            onChange={(e) => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                            onKeyDown={(e) => e.key === 'Enter' && handleSaveInline()}
                            className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-primary font-bold outline-none"
                          />
                        </div>
                      ) : (
                        <div
                          className="flex items-center gap-1"
                          onDoubleClick={() => setInlineEdit({ type: 'rename-var', varIndex: idx, value: keyStr })}
                        >
                          <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold">
                            {`{{${keyStr}}}`}
                          </span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setInlineEdit({ type: 'rename-var', varIndex: idx, value: keyStr });
                            }}
                            className="p-0.5 text-muted-foreground hover:text-foreground opacity-60 hover:opacity-100 transition-opacity"
                            title="Rename variable key"
                          >
                            <MingCuteIcon name="edit_line" size={12} />
                          </button>
                        </div>
                      )}
                    </div>

                    {/* Variant Badge & Actions */}
                    <div className="flex items-center gap-1.5 shrink-0">
                      {isEditingThis && inlineEdit.type === 'rename-variant' ? (
                        <div className="flex items-center gap-1">
                          <span className="text-[10px] text-amber-500 font-bold">Variant Name:</span>
                          <input
                            ref={editInputRef}
                            type="text"
                            value={inlineEdit.value}
                            onChange={(e) => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                            onKeyDown={(e) => e.key === 'Enter' && handleSaveInline()}
                            className="bg-background border border-amber-500 rounded px-1.5 py-0.5 text-xs text-amber-500 font-bold outline-none"
                          />
                        </div>
                      ) : (
                        <div className="flex items-center gap-1 bg-amber-500/10 border border-amber-500/30 rounded px-2 py-0.5">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCycleVariant(idx, 'prev');
                            }}
                            className="text-amber-500 hover:text-amber-400 font-bold px-0.5 cursor-pointer"
                            title="Previous variant"
                          >
                            ‹
                          </button>
                          <span
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCycleVariant(idx, 'next');
                            }}
                            onDoubleClick={(e) => {
                              e.stopPropagation();
                              setInlineEdit({ type: 'rename-variant', varIndex: idx, value: activeVariant.name });
                            }}
                            className="text-[10px] uppercase font-mono font-bold text-amber-500 cursor-pointer"
                            title="Click to cycle variant or double click to rename variant"
                          >
                            Variant: <span className="underline">{activeVariant.name}</span>
                          </span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCycleVariant(idx, 'next');
                            }}
                            className="text-amber-500 hover:text-amber-400 font-bold px-0.5 cursor-pointer"
                            title="Next variant"
                          >
                            ›
                          </button>
                        </div>
                      )}

                      {/* Add Variant Button */}
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setInlineEdit({
                            type: 'new-variant',
                            varIndex: idx,
                            value: 'New Variant',
                            extraValue: '',
                          });
                        }}
                        className="px-1.5 py-0.5 text-[10px] font-mono font-semibold bg-surface hover:bg-neutral-subtle border border-border rounded text-muted-foreground hover:text-foreground transition-colors cursor-pointer flex items-center gap-0.5"
                        title="Add new variant"
                      >
                        <MingCuteIcon name="add_line" size={11} />
                        <span>Variant</span>
                      </button>

                      {/* Delete Variant Button */}
                      {variants.length > 1 && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            if (window.confirm(`Delete variant "${activeVariant.name}"?`)) {
                              handleDeleteVariant(idx);
                            }
                          }}
                          className="p-1 text-muted-foreground hover:text-rose-400 rounded transition-colors cursor-pointer"
                          title={`Delete variant ${activeVariant.name}`}
                        >
                          <MingCuteIcon name="delete_2_line" size={12} />
                        </button>
                      )}

                      {/* Delete Variable Button */}
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (window.confirm(`Permanently delete variable "${keyStr}"?`)) {
                            handleDeleteVariable(keyStr);
                          }
                        }}
                        className="p-1 text-muted-foreground hover:text-rose-500 rounded transition-colors cursor-pointer ml-1"
                        title={`Delete variable ${keyStr}`}
                      >
                        <MingCuteIcon name="delete_fill" size={13} />
                      </button>
                    </div>
                  </div>

                  {/* Bottom Line: Value Field / Inline Editor */}
                  <div className="flex items-center justify-between gap-2 font-mono text-xs">
                    {isEditingThis && inlineEdit.type === 'value' ? (
                      <div className="flex items-center gap-2 w-full">
                        <span className="text-[10px] text-emerald-500 font-bold uppercase shrink-0">Edit Value:</span>
                        <input
                          ref={editInputRef}
                          type="text"
                          value={inlineEdit.value}
                          onChange={(e) => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') handleSaveInline();
                            if (e.key === 'Escape') setInlineEdit(null);
                          }}
                          className="w-full bg-background border border-emerald-500 rounded px-2 py-1 text-xs text-foreground outline-none font-mono"
                        />
                        <button
                          onClick={handleSaveInline}
                          className="px-2 py-1 bg-emerald-500 text-white rounded font-sans text-xs font-semibold cursor-pointer shrink-0"
                        >
                          Save
                        </button>
                        <button
                          onClick={() => setInlineEdit(null)}
                          className="px-2 py-1 bg-surface border border-border text-muted-foreground hover:text-foreground rounded font-sans text-xs cursor-pointer shrink-0"
                        >
                          Cancel
                        </button>
                      </div>
                    ) : isEditingThis && inlineEdit.type === 'new-variant' ? (
                      <div className="flex items-center gap-2 w-full bg-amber-500/10 p-2 rounded border border-amber-500/40">
                        <div className="flex flex-col gap-1 w-full">
                          <div className="flex items-center justify-between">
                            <span className="text-[9px] uppercase font-bold text-amber-500">New Variant:</span>
                            <button
                              type="button"
                              onClick={() => setInlineEdit(null)}
                              className="text-muted-foreground hover:text-foreground text-[10px] cursor-pointer"
                            >
                              ✕ Close
                            </button>
                          </div>
                          <div className="flex items-center gap-2">
                            <input
                              ref={editInputRef}
                              type="text"
                              placeholder="Variant Name (e.g. Staging)"
                              value={inlineEdit.value}
                              onChange={(e) => setInlineEdit({ ...inlineEdit, value: e.target.value })}
                              onKeyDown={(e) => {
                                if (e.key === 'Enter') handleSaveInline();
                                if (e.key === 'Escape') setInlineEdit(null);
                              }}
                              className="w-1/2 bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground outline-none"
                            />
                            <input
                              type="text"
                              placeholder="Initial Value..."
                              value={inlineEdit.extraValue || ''}
                              onChange={(e) => setInlineEdit({ ...inlineEdit, extraValue: e.target.value })}
                              onKeyDown={(e) => {
                                if (e.key === 'Enter') handleSaveInline();
                                if (e.key === 'Escape') setInlineEdit(null);
                              }}
                              className="w-1/2 bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground outline-none font-mono"
                            />
                            <button
                              onClick={handleSaveInline}
                              className="px-2.5 py-1 bg-amber-500 text-black font-bold rounded text-xs cursor-pointer shrink-0"
                            >
                              Add
                            </button>
                            <button
                              type="button"
                              onClick={() => setInlineEdit(null)}
                              className="px-2 py-1 bg-surface border border-border text-muted-foreground hover:text-foreground rounded text-xs cursor-pointer shrink-0"
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div
                        onDoubleClick={() => setInlineEdit({ type: 'value', varIndex: idx, value: valStr })}
                        onClick={() => setInlineEdit({ type: 'value', varIndex: idx, value: valStr })}
                        className="w-full px-2 py-1 bg-background border border-border/80 rounded flex items-center justify-between text-muted-foreground hover:text-foreground hover:border-primary/50 transition-colors cursor-text"
                        title="Click to edit value"
                      >
                        <span className="truncate max-w-[420px]">
                          {valStr ? (isSecretVal ? '••••••••' : valStr) : <span className="opacity-40 italic">&lt;empty&gt;</span>}
                        </span>
                        <span className="text-[10px] text-muted-foreground/60 font-sans">Click to edit</span>
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Create Variable Form */}
        <form onSubmit={handleCreate} className="p-3 border-t border-border bg-header flex items-center gap-2 font-mono">
          <input
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="NEW_KEY"
            className="w-1/3 bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <input
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="Value..."
            className="w-1/2 bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <label className="flex items-center gap-1 text-[11px] text-muted-foreground cursor-pointer select-none font-sans">
            <input
              type="checkbox"
              checked={isSecret}
              onChange={(e) => setIsSecret(e.target.checked)}
              className="rounded border-border text-primary"
            />
            Mask
          </label>
          <button
            type="submit"
            className="px-3 py-1 bg-primary text-primary-foreground text-xs font-sans font-semibold rounded hover:bg-primary/90 transition-colors shrink-0 shadow-2xs cursor-pointer"
          >
            Add Variable
          </button>
        </form>
      </div>
    </div>
  );
};
