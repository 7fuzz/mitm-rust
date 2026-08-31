import React, { useState, useEffect } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import { Button } from './ui/Button';
import type { EnvironmentVariable, VariableVariant } from '../../services/tauri/bridge';

interface VariantManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
  variableKey: string;
  variable: EnvironmentVariable;
  onSave: (updatedVar: EnvironmentVariable) => Promise<void>;
}

export const VariantManagerModal: React.FC<VariantManagerModalProps> = ({
  isOpen,
  onClose,
  variableKey,
  variable,
  onSave,
}) => {
  const rawVariants: VariableVariant[] =
    variable.variants && variable.variants.length > 0
      ? variable.variants
      : [{ name: '(auto)', value: variable.value || '' }];

  const normalizedInitialVariants = (() => {
    const copy = [...rawVariants];
    const autoIdx = copy.findIndex((v) => v.name === '(auto)');
    if (autoIdx === -1) {
      copy.unshift({ name: '(auto)', value: variable.value || '' });
    } else if (autoIdx > 0) {
      const [autoVar] = copy.splice(autoIdx, 1);
      copy.unshift(autoVar);
    }
    return copy;
  })();

  const [variants, setVariants] = useState<VariableVariant[]>(normalizedInitialVariants);
  const [activeIndex, setActiveIndex] = useState<number>(variable.activeIndex || 0);
  const [newVariantName, setNewVariantName] = useState('');
  const [newVariantVal, setNewVariantVal] = useState('');

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const handleUpdateVariant = (idx: number, field: 'name' | 'value', val: string) => {
    if (idx === 0 && field === 'name') return; // Cannot rename (auto)
    const updated = variants.map((v, i) => (i === idx ? { ...v, [field]: val } : v));
    setVariants(updated);
  };

  const handleRemoveVariant = (idx: number) => {
    if (idx === 0) return; // Cannot delete (auto) variant
    if (variants.length <= 1) return;
    const updated = variants.filter((_, i) => i !== idx);
    setVariants(updated);
    if (activeIndex >= updated.length) {
      setActiveIndex(Math.max(0, updated.length - 1));
    }
  };

  const handleAddVariant = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newVariantName.trim() || newVariantName.trim() === '(auto)') return;
    const updated = [...variants, { name: newVariantName.trim(), value: newVariantVal }];
    setVariants(updated);
    setActiveIndex(updated.length - 1);
    setNewVariantName('');
    setNewVariantVal('');
  };

  const handleSave = async () => {
    const selectedVariant = variants[activeIndex] || variants[0];
    const updatedVar: EnvironmentVariable = {
      ...variable,
      activeIndex,
      variants,
      value: selectedVariant?.value || '',
    };
    await onSave(updatedVar);
    onClose();
  };

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 font-sans text-xs select-none cursor-pointer"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="bg-surface border border-border rounded-xl shadow-2xl w-full max-w-lg p-5 flex flex-col gap-4 text-foreground cursor-default select-text"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border pb-3">
          <div className="flex items-center gap-2 font-semibold text-sm">
            <MingCuteIcon name="earth_line" className="text-amber-500" size={18} />
            <span>
              Manage Variants for <span className="font-mono text-primary font-bold">{`{{${variableKey}}}`}</span>
            </span>
          </div>
          <button onClick={onClose} className="p-1 text-muted-foreground hover:text-foreground rounded cursor-pointer">
            <MingCuteIcon name="close_line" size={16} />
          </button>
        </div>

        {/* Variants List Table */}
        <div className="space-y-2 max-h-60 overflow-y-auto pr-1 font-mono">
          {variants.map((variant, idx) => (
            <div key={idx} className="p-2 border border-border rounded-lg bg-background flex items-center gap-2">
              <input
                type="radio"
                name="active-variant"
                checked={activeIndex === idx}
                onChange={() => setActiveIndex(idx)}
                className="cursor-pointer text-primary accent-primary"
                title="Set as active variant"
              />

              {idx === 0 ? (
                <span className="w-1/3 bg-amber-500/10 text-amber-500 font-bold border border-amber-500/30 rounded px-2 py-1 text-xs flex items-center justify-between" title="System Auto Extraction Variant (Cannot delete)">
                  <span>(auto)</span>
                  <MingCuteIcon name="lock_line" size={12} />
                </span>
              ) : (
                <input
                  type="text"
                  value={variant.name}
                  onChange={(e) => handleUpdateVariant(idx, 'name', e.target.value)}
                  placeholder="Variant Name (e.g. Staging)"
                  className="w-1/3 bg-surface border border-border rounded px-2 py-1 text-xs text-amber-500 font-bold focus:outline-none focus:border-amber-500"
                />
              )}

              <input
                type="text"
                value={variant.value}
                onChange={(e) => handleUpdateVariant(idx, 'value', e.target.value)}
                placeholder="Variant Value..."
                className="w-1/2 bg-surface border border-border rounded px-2 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
              />

              {idx !== 0 && variants.length > 1 ? (
                <button
                  type="button"
                  onClick={() => handleRemoveVariant(idx)}
                  className="p-1 text-muted-foreground hover:text-rose-400 rounded transition-colors cursor-pointer"
                  title="Remove variant"
                >
                  <MingCuteIcon name="delete_2_line" size={14} />
                </button>
              ) : (
                <div className="w-6 shrink-0" />
              )}
            </div>
          ))}
        </div>

        {/* Add Variant Quick Form */}
        <form onSubmit={handleAddVariant} className="flex items-center gap-2 pt-2 border-t border-border font-mono">
          <input
            type="text"
            value={newVariantName}
            onChange={(e) => setNewVariantName(e.target.value)}
            placeholder="New Variant Name"
            className="w-1/3 bg-background border border-border rounded px-2 py-1 text-xs text-foreground outline-none focus:border-amber-500"
          />
          <input
            type="text"
            value={newVariantVal}
            onChange={(e) => setNewVariantVal(e.target.value)}
            placeholder="Initial Value..."
            className="w-1/2 bg-background border border-border rounded px-2 py-1 text-xs text-foreground outline-none font-mono focus:border-primary"
          />
          <Button type="submit" variant="amber" size="xs" icon="plus_line">
            Add
          </Button>
        </form>

        {/* Footer Actions */}
        <div className="flex justify-end gap-2 border-t border-border pt-3">
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" onClick={handleSave}>
            Save Variants
          </Button>
        </div>
      </div>
    </div>
  );
};
