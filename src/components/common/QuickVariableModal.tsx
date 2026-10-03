import React, { useState, useEffect, useRef } from 'react';
import { useSettingsStore } from '../../stores/useSettingsStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { useRepeaterEnvStore } from '../../stores/useRepeaterEnvStore';
import { MingCuteIcon } from './MingCuteIcon';
import type { EnvironmentVariable } from '../../services/tauri/bridge';
import { QuickVariableEnvSelector } from './quick-variable/QuickVariableEnvSelector';
import { QuickVariableItem, type InlineEditState } from './quick-variable/QuickVariableItem';
import { getVariableVariants } from '../../utils/envVariables';
import { QuickVariableFooter } from './quick-variable/QuickVariableFooter';
import { useQuickVariableShortcuts } from './quick-variable/useQuickVariableShortcuts';

export const QuickVariableModal: React.FC = () => {
  const { isQuickVarModalOpen, setQuickVarModalOpen, quickVarScope } = useSettingsStore();
  const workspaceEnvs = useWorkspaceStore();
  const repeaterEnvs = useRepeaterEnvStore();
  const isRepeater = quickVarScope === 'repeater';
  const { activeEnvironmentId, environmentsList, saveEnvironmentVariables } = isRepeater ? repeaterEnvs : workspaceEnvs;
  const environments = isRepeater ? [] : workspaceEnvs.environments;
  const setActiveEnv = workspaceEnvs.setActiveEnv;
  const loadRepeaterEnvs = repeaterEnvs.loadEnvironments;

  useEffect(() => {
    if (isQuickVarModalOpen && isRepeater) loadRepeaterEnvs();
  }, [isQuickVarModalOpen, isRepeater, loadRepeaterEnvs]);

  const [search, setSearch] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [isSecret, setIsSecret] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const [inlineEdit, setInlineEdit] = useState<InlineEditState | null>(null);

  const modalRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const editInputRef = useRef<HTMLInputElement>(null);
  const newKeyInputRef = useRef<HTMLInputElement>(null);
  const newValueInputRef = useRef<HTMLInputElement>(null);

  const currentEnvFromList =
    environmentsList.find((e) => e.id === activeEnvironmentId || e.isActive) || environmentsList[0];
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

  const showToast = (msg: string) => {
    setToast(msg);
    setTimeout(() => {
      setToast((prev) => (prev === msg ? null : prev));
    }, 2000);
  };

  useEffect(() => {
    if (isQuickVarModalOpen) {
      setSelectedIndex(0);
      setSearch('');
      setInlineEdit(null);
      setToast(null);
      // Focus modal container so keyboard navigation works immediately on open
      setTimeout(() => {
        modalRef.current?.focus();
      }, 30);
    }
  }, [isQuickVarModalOpen]);

  useEffect(() => {
    if (selectedIndex >= filteredVars.length && filteredVars.length > 0) {
      setSelectedIndex(filteredVars.length - 1);
    }
  }, [filteredVars.length, selectedIndex]);

  // Instant scroll (behavior: 'auto') prevents lag / animation queuing when holding arrow keys
  useEffect(() => {
    if (listRef.current && listRef.current.children[selectedIndex]) {
      const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
      selectedEl.scrollIntoView({ block: 'nearest', behavior: 'auto' });
    }
  }, [selectedIndex]);

  useEffect(() => {
    if (inlineEdit) {
      setTimeout(() => editInputRef.current?.focus(), 20);
    }
  }, [inlineEdit]);

  const handleSelectEnvironment = async (envId: string) => {
    const targetInList = environmentsList.find((e) => e.id === envId);
    if (targetInList) {
      await saveEnvironmentVariables({ ...targetInList, isActive: true, updatedAtMs: Date.now() });
    } else {
      await setActiveEnv(envId);
    }
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
    const variants = getVariableVariants(currentVar);
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
      showToast(`Updated value for {{${currentVar.key}}}`);
    } else if (type === 'rename-var') {
      if (value.trim()) {
        updatedVars[realIndex] = {
          ...currentVar,
          key: value.trim(),
        };
        showToast(`Renamed variable to {{${value.trim()}}}`);
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
        showToast(`Renamed variant to "${value.trim()}"`);
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
        showToast(`Added variant "${value.trim()}"`);
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
    showToast(`Deleted variable {{${varKey}}}`);
  };

  const handleDeleteVariant = async (varIndex: number) => {
    if (!currentEnvFromList) return;
    const targetVar = filteredVars[varIndex];
    if (!targetVar) return;

    const realIndex = activeVars.findIndex((v) => v.key === targetVar.key);
    if (realIndex === -1) return;

    const currentVar = activeVars[realIndex];
    const variants = getVariableVariants(currentVar);
    if (variants.length <= 1) return;

    const activeIdx = currentVar.activeIndex || 0;
    const targetVariantName = variants[activeIdx]?.name || 'variant';
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
    showToast(`Deleted variant "${targetVariantName}"`);
  };

  const handleCycleVariant = async (varIndex: number, direction: 'next' | 'prev' = 'next') => {
    if (!currentEnvFromList) return;
    const targetVar = filteredVars[varIndex];
    if (!targetVar) return;

    const realIndex = activeVars.findIndex((v) => v.key === targetVar.key);
    if (realIndex === -1) return;

    const currentVar = activeVars[realIndex];
    const variants = getVariableVariants(currentVar);
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
    showToast(`Variant: ${nextVariant.name}`);
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKey.trim() || !currentEnvFromList) return;
    const initialVal = newValue;
    const keyName = newKey.trim().toUpperCase();
    const newVar: EnvironmentVariable = {
      key: keyName,
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
    showToast(`Created variable {{${keyName}}}`);
  };

  // Keyboard shortcut controller
  useQuickVariableShortcuts({
    isOpen: isQuickVarModalOpen,
    onClose: () => setQuickVarModalOpen(false),
    filteredVars,
    selectedIndex,
    setSelectedIndex,
    inlineEdit,
    setInlineEdit,
    onSaveInline: handleSaveInline,
    onCycleVariant: handleCycleVariant,
    onDeleteVariant: handleDeleteVariant,
    onDeleteVariable: handleDeleteVariable,
    onSelectEnvironment: handleSelectEnvironment,
    environmentsList,
    activeEnvironmentId,
    setSearch,
    searchInputRef,
    editInputRef,
    newKeyInputRef,
    newValueInputRef,
    showToast,
  });

  if (!isQuickVarModalOpen) return null;

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          setQuickVarModalOpen(false);
        }
      }}
      className="fixed inset-0 z-50 flex items-start justify-center pt-10 bg-black/60 backdrop-blur-xs animate-in fade-in duration-150 font-sans text-xs select-none cursor-pointer"
    >
      <div
        ref={modalRef}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-2xl bg-surface border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh] text-foreground cursor-default select-text outline-none"
      >
        {/* Modal Header */}
        <div className="p-3 border-b border-border flex items-center justify-between bg-header shrink-0">
          <div className="flex items-center gap-2 font-semibold text-sm">
            <MingCuteIcon name="earth_line" size={18} className="text-primary" />
            <span>{isRepeater ? 'Repeater Variables' : 'Quick Environment & Variable Switcher'}</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-2xs font-mono text-muted-foreground">Press <kbd className="px-1 py-0.5 rounded bg-background border border-border text-foreground font-bold">Esc</kbd> to close</span>
            <button
              type="button"
              onClick={() => setQuickVarModalOpen(false)}
              className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors cursor-pointer"
            >
              <MingCuteIcon name="close_line" size={16} />
            </button>
          </div>
        </div>

        {/* Environment Selector Bar */}
        <QuickVariableEnvSelector
          environmentsList={environmentsList}
          environments={environments}
          activeEnvironmentId={activeEnvironmentId}
          onSelectEnvironment={handleSelectEnvironment}
          label={isRepeater ? 'Repeater Environments' : 'Active Workspace Environments'}
        />

        {/* Filter Input Bar */}
        <div className="p-2.5 border-b border-border flex items-center gap-2 bg-header font-mono shrink-0">
          <MingCuteIcon name="search_line" className="text-muted-foreground" size={16} />
          <input
            ref={searchInputRef}
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={`Type '/' to search variables in ${currentEnvName} (e.g. {{AUTH_TOKEN}})...`}
            className="w-full bg-transparent text-xs focus:outline-none text-foreground placeholder:text-muted-foreground font-mono"
          />
          {search && (
            <button
              type="button"
              onClick={() => {
                setSearch('');
                searchInputRef.current?.focus();
              }}
              className="text-muted-foreground hover:text-foreground text-xs p-1"
            >
              <MingCuteIcon name="close_line" size={13} />
            </button>
          )}
        </div>

        {/* Variables List */}
        <div ref={listRef} className="p-3 overflow-y-auto space-y-2 flex-1 custom-scrollbar min-h-[220px]">
          {filteredVars.length === 0 ? (
            <div className="py-12 text-center text-xs text-muted-foreground italic font-sans flex flex-col items-center gap-2">
              <MingCuteIcon name="earth_line" size={28} className="opacity-30" />
              <span>No variables found in "{currentEnvName}"</span>
              <span className="text-2xs font-mono">Press <kbd className="px-1 py-0.5 rounded bg-surface border border-border text-foreground">Shift+N</kbd> to add a new variable</span>
            </div>
          ) : (
            filteredVars.map((v, idx) => (
              <QuickVariableItem
                key={v.key || idx}
                variable={v}
                index={idx}
                isSelected={selectedIndex === idx}
                inlineEdit={inlineEdit}
                editInputRef={editInputRef}
                onSelect={setSelectedIndex}
                onSetInlineEdit={setInlineEdit}
                onSaveInline={handleSaveInline}
                onCycleVariant={handleCycleVariant}
                onDeleteVariant={handleDeleteVariant}
                onDeleteVariable={handleDeleteVariable}
              />
            ))
          )}
        </div>

        {/* Create Variable Form */}
        <form onSubmit={handleCreate} className="p-2.5 border-t border-border bg-header flex items-center gap-2 font-mono shrink-0">
          <input
            ref={newKeyInputRef}
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="NEW_KEY (Shift+N)"
            className="w-1/3 bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <input
            ref={newValueInputRef}
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="Value..."
            className="w-1/2 bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <label className="flex items-center gap-1 text-2xs text-muted-foreground cursor-pointer select-none font-sans">
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

        {/* Shortcut Cheat Sheet Footer */}
        <QuickVariableFooter toast={toast} />
      </div>
    </div>
  );
};
