import { useEffect } from 'react';
import type { Environment, EnvironmentVariable } from '../../../services/tauri/bridge';
import type { InlineEditState } from './QuickVariableItem';
import { getVariableVariants } from './QuickVariableItem';

interface UseQuickVariableShortcutsParams {
  isOpen: boolean;
  onClose: () => void;
  filteredVars: EnvironmentVariable[];
  selectedIndex: number;
  setSelectedIndex: React.Dispatch<React.SetStateAction<number>>;
  inlineEdit: InlineEditState | null;
  setInlineEdit: (state: InlineEditState | null) => void;
  onSaveInline: () => void;
  onCycleVariant: (varIndex: number, direction: 'next' | 'prev') => void;
  onDeleteVariant: (varIndex: number) => void;
  onDeleteVariable: (varKey: string) => void;
  onSelectEnvironment: (envId: string) => void;
  environmentsList: Environment[];
  activeEnvironmentId: string | null;
  setSearch: React.Dispatch<React.SetStateAction<string>>;
  searchInputRef: React.RefObject<HTMLInputElement | null>;
  editInputRef: React.RefObject<HTMLInputElement | null>;
  newKeyInputRef: React.RefObject<HTMLInputElement | null>;
  newValueInputRef: React.RefObject<HTMLInputElement | null>;
  showToast: (msg: string) => void;
}

export const useQuickVariableShortcuts = ({
  isOpen,
  onClose,
  filteredVars,
  selectedIndex,
  setSelectedIndex,
  inlineEdit,
  setInlineEdit,
  onSaveInline,
  onCycleVariant,
  onDeleteVariant,
  onDeleteVariable,
  onSelectEnvironment,
  environmentsList,
  activeEnvironmentId,
  setSearch,
  searchInputRef,
  editInputRef,
  newKeyInputRef,
  newValueInputRef,
  showToast,
}: UseQuickVariableShortcutsParams) => {
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle modal with Ctrl+K / Cmd+K
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        onClose();
        return;
      }

      const activeEl = document.activeElement;
      const isSearchFocused = activeEl === searchInputRef.current;
      const isNewKeyFocused =
        activeEl === newKeyInputRef.current || activeEl === newValueInputRef.current;
      const isInlineEditFocused = activeEl === editInputRef.current;

      // 1. Search bar shortcuts:
      // Enter keeps the query and blurs back to list; Esc clears search and blurs back to list
      if (isSearchFocused) {
        if (e.key === 'Enter') {
          e.preventDefault();
          searchInputRef.current?.blur();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          setSearch('');
          searchInputRef.current?.blur();
        }
        return;
      }

      // 2. Inline Edit shortcuts: Enter saves, Esc cancels (PREVENTS CLOSING DIALOG)
      if (inlineEdit || isInlineEditFocused) {
        if (e.key === 'Enter') {
          e.preventDefault();
          e.stopPropagation();
          onSaveInline();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          setInlineEdit(null);
        }
        return;
      }

      // 3. New variable inputs: Esc blurs
      if (isNewKeyFocused) {
        if (e.key === 'Escape') {
          e.preventDefault();
          (activeEl as HTMLElement)?.blur();
        }
        return;
      }

      // Any other text input focused: Esc blurs without closing modal
      if (
        activeEl?.tagName === 'INPUT' ||
        activeEl?.tagName === 'TEXTAREA' ||
        activeEl?.getAttribute('contenteditable') === 'true'
      ) {
        if (e.key === 'Escape') {
          e.preventDefault();
          (activeEl as HTMLElement)?.blur();
        }
        return;
      }

      // 4. Modal Root Shortcuts:

      // Esc: Close modal (only when not editing)
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }

      // / : Activate search input
      if (e.key === '/') {
        e.preventDefault();
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
        return;
      }

      // Up / Down: Switch current selected variable with WRAP-AROUND
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (filteredVars.length > 0) {
          setSelectedIndex((prev) => (prev <= 0 ? filteredVars.length - 1 : prev - 1));
        }
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (filteredVars.length > 0) {
          setSelectedIndex((prev) => (prev >= filteredVars.length - 1 ? 0 : prev + 1));
        }
        return;
      }

      // Left / Right: Switch variant of current selected variable
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        if (filteredVars.length > 0) {
          onCycleVariant(selectedIndex, 'prev');
        }
        return;
      }
      if (e.key === 'ArrowRight') {
        e.preventDefault();
        if (filteredVars.length > 0) {
          onCycleVariant(selectedIndex, 'next');
        }
        return;
      }

      // e : Switch to next environment
      if (e.key === 'e') {
        e.preventDefault();
        if (environmentsList.length > 1) {
          const currentIdx = environmentsList.findIndex(
            (env) => env.id === activeEnvironmentId || env.isActive
          );
          const nextIdx = (currentIdx + 1) % environmentsList.length;
          onSelectEnvironment(environmentsList[nextIdx].id);
          showToast(`Switched Environment: ${environmentsList[nextIdx].name}`);
        }
        return;
      }

      // E (Shift+E) : Switch to previous environment
      if (e.key === 'E') {
        e.preventDefault();
        if (environmentsList.length > 1) {
          const currentIdx = environmentsList.findIndex(
            (env) => env.id === activeEnvironmentId || env.isActive
          );
          const prevIdx = (currentIdx - 1 + environmentsList.length) % environmentsList.length;
          onSelectEnvironment(environmentsList[prevIdx].id);
          showToast(`Switched Environment: ${environmentsList[prevIdx].name}`);
        }
        return;
      }

      const targetVar = filteredVars[selectedIndex];
      if (!targetVar) {
        if (e.key === 'N') {
          e.preventDefault();
          newKeyInputRef.current?.focus();
        }
        return;
      }

      const variants = getVariableVariants(targetVar);
      const activeVarIdx = targetVar.activeIndex || 0;
      const activeVariant = variants[activeVarIdx] || variants[0] || { name: '(auto)', value: targetVar.value };

      // n : New variant for current selected variable
      if (e.key === 'n') {
        e.preventDefault();
        setInlineEdit({
          type: 'new-variant',
          varIndex: selectedIndex,
          value: '',
          extraValue: '',
        });
        return;
      }

      // N (Shift+N) : New variable
      if (e.key === 'N') {
        e.preventDefault();
        newKeyInputRef.current?.focus();
        return;
      }

      // r : Rename active variant
      if (e.key === 'r') {
        e.preventDefault();
        setInlineEdit({
          type: 'rename-variant',
          varIndex: selectedIndex,
          value: activeVariant.name,
        });
        return;
      }

      // R (Shift+R) : Rename variable key
      if (e.key === 'R') {
        e.preventDefault();
        setInlineEdit({
          type: 'rename-var',
          varIndex: selectedIndex,
          value: targetVar.key,
        });
        return;
      }

      // d : Delete current active variant
      if (e.key === 'd') {
        e.preventDefault();
        if (variants.length > 1) {
          onDeleteVariant(selectedIndex);
        } else {
          showToast(`Cannot delete default (auto) variant`);
        }
        return;
      }

      // D (Shift+D) : Delete variable
      if (e.key === 'D') {
        e.preventDefault();
        onDeleteVariable(targetVar.key);
        return;
      }

      // c : Copy variant value to clipboard
      if (e.key === 'c') {
        e.preventDefault();
        const textToCopy = activeVariant.value || '';
        navigator.clipboard.writeText(textToCopy);
        showToast(`Copied variant value: "${textToCopy.slice(0, 24)}${textToCopy.length > 24 ? '...' : ''}"`);
        return;
      }

      // C (Shift+C) : Copy variable tag {{KEY}} to clipboard
      if (e.key === 'C') {
        e.preventDefault();
        const textToCopy = `{{${targetVar.key}}}`;
        navigator.clipboard.writeText(textToCopy);
        showToast(`Copied variable tag: ${textToCopy}`);
        return;
      }

      // Enter : Edit active variant value
      if (e.key === 'Enter') {
        e.preventDefault();
        setInlineEdit({
          type: 'value',
          varIndex: selectedIndex,
          value: activeVariant.value || '',
        });
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    isOpen,
    onClose,
    inlineEdit,
    selectedIndex,
    filteredVars,
    activeEnvironmentId,
    environmentsList,
    setSelectedIndex,
    setInlineEdit,
    onSaveInline,
    onCycleVariant,
    onDeleteVariant,
    onDeleteVariable,
    onSelectEnvironment,
    setSearch,
    searchInputRef,
    editInputRef,
    newKeyInputRef,
    newValueInputRef,
    showToast,
  ]);
};
