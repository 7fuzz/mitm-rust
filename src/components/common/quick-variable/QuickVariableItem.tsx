import React from 'react';
import { MingCuteIcon } from '../MingCuteIcon';
import type { EnvironmentVariable, VariableVariant } from '../../../services/tauri/bridge';

export type InlineEditType = 'value' | 'rename-var' | 'rename-variant' | 'new-variant';

export interface InlineEditState {
  type: InlineEditType;
  varIndex: number;
  value: string;
  extraValue?: string;
}

interface QuickVariableItemProps {
  variable: EnvironmentVariable;
  index: number;
  isSelected: boolean;
  inlineEdit: InlineEditState | null;
  editInputRef: React.RefObject<HTMLInputElement | null>;
  onSelect: (index: number) => void;
  onSetInlineEdit: (state: InlineEditState | null) => void;
  onSaveInline: () => void;
  onCycleVariant: (varIndex: number, direction: 'next' | 'prev') => void;
  onDeleteVariant: (varIndex: number) => void;
  onDeleteVariable: (varKey: string) => void;
}

export const getVariableVariants = (v: EnvironmentVariable): VariableVariant[] => {
  if (v.variants && v.variants.length > 0) {
    return v.variants;
  }
  return [{ name: '(auto)', value: v.value || '' }];
};

export const QuickVariableItem: React.FC<QuickVariableItemProps> = ({
  variable,
  index,
  isSelected,
  inlineEdit,
  editInputRef,
  onSelect,
  onSetInlineEdit,
  onSaveInline,
  onCycleVariant,
  onDeleteVariant,
  onDeleteVariable,
}) => {
  const keyStr = typeof variable.key === 'string' ? variable.key : String(variable.key || '');
  const isSecretVal = variable.type === 'secret';
  const variants = getVariableVariants(variable);
  const activeVarIdx = variable.activeIndex || 0;
  const activeVariant = variants[activeVarIdx] || variants[0] || { name: '(auto)', value: variable.value };
  const valStr = typeof activeVariant.value === 'string' ? activeVariant.value : String(activeVariant.value || '');

  const isEditingThis = isSelected && inlineEdit && inlineEdit.varIndex === index;

  return (
    <div
      onClick={() => onSelect(index)}
      className={`p-2.5 rounded-lg border transition-all flex flex-col gap-2 cursor-pointer ${
        isSelected
          ? 'bg-primary/10 border-primary/50 shadow-2xs ring-1 ring-primary/30'
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
                onChange={(e) => onSetInlineEdit({ ...inlineEdit, value: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    e.stopPropagation();
                    onSaveInline();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    e.stopPropagation();
                    onSetInlineEdit(null);
                  }
                }}
                className="bg-background border border-primary rounded px-1.5 py-0.5 text-xs text-primary font-bold outline-none font-mono"
              />
            </div>
          ) : (
            <div
              className="flex items-center gap-1"
              onDoubleClick={() => onSetInlineEdit({ type: 'rename-var', varIndex: index, value: keyStr })}
            >
              <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold">
                {`{{${keyStr}}}`}
              </span>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onSetInlineEdit({ type: 'rename-var', varIndex: index, value: keyStr });
                }}
                className="p-0.5 text-muted-foreground hover:text-foreground opacity-60 hover:opacity-100 transition-opacity"
                title="Rename variable key (R)"
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
              <span className="text-[10px] text-amber-500 font-bold">Variant:</span>
              <input
                ref={editInputRef}
                type="text"
                value={inlineEdit.value}
                onChange={(e) => onSetInlineEdit({ ...inlineEdit, value: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    e.stopPropagation();
                    onSaveInline();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    e.stopPropagation();
                    onSetInlineEdit(null);
                  }
                }}
                className="bg-background border border-amber-500 rounded px-1.5 py-0.5 text-xs text-amber-500 font-bold outline-none font-mono"
              />
            </div>
          ) : (
            <div className="flex items-center gap-1 bg-amber-500/10 border border-amber-500/30 rounded px-2 py-0.5">
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onCycleVariant(index, 'prev');
                }}
                className="text-amber-500 hover:text-amber-400 font-bold px-0.5 cursor-pointer"
                title="Previous variant (←)"
              >
                ‹
              </button>
              <span
                onClick={(e) => {
                  e.stopPropagation();
                  onCycleVariant(index, 'next');
                }}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  onSetInlineEdit({ type: 'rename-variant', varIndex: index, value: activeVariant.name });
                }}
                className="text-[10px] uppercase font-mono font-bold text-amber-500 cursor-pointer"
                title="Click or press ←/→ to cycle variant, 'r' to rename"
              >
                Variant: <span className="underline">{activeVariant.name}</span>
              </span>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onCycleVariant(index, 'next');
                }}
                className="text-amber-500 hover:text-amber-400 font-bold px-0.5 cursor-pointer"
                title="Next variant (→)"
              >
                ›
              </button>
            </div>
          )}

          {/* Add Variant Button */}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSetInlineEdit({
                type: 'new-variant',
                varIndex: index,
                value: 'New Variant',
                extraValue: '',
              });
            }}
            className="px-1.5 py-0.5 text-[10px] font-mono font-semibold bg-surface hover:bg-neutral-subtle border border-border rounded text-muted-foreground hover:text-foreground transition-colors cursor-pointer flex items-center gap-0.5"
            title="Add new variant (n)"
          >
            <MingCuteIcon name="add_line" size={11} />
            <span>Variant</span>
          </button>

          {/* Delete Variant Button */}
          {variants.length > 1 && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                if (window.confirm(`Delete variant "${activeVariant.name}"?`)) {
                  onDeleteVariant(index);
                }
              }}
              className="p-1 text-muted-foreground hover:text-rose-400 rounded transition-colors cursor-pointer"
              title={`Delete variant ${activeVariant.name} (d)`}
            >
              <MingCuteIcon name="delete_2_line" size={12} />
            </button>
          )}

          {/* Delete Variable Button */}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              if (window.confirm(`Permanently delete variable "${keyStr}"?`)) {
                onDeleteVariable(keyStr);
              }
            }}
            className="p-1 text-muted-foreground hover:text-rose-500 rounded transition-colors cursor-pointer ml-1"
            title={`Delete variable ${keyStr} (D)`}
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
              onChange={(e) => onSetInlineEdit({ ...inlineEdit, value: e.target.value })}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  e.stopPropagation();
                  onSaveInline();
                } else if (e.key === 'Escape') {
                  e.preventDefault();
                  e.stopPropagation();
                  onSetInlineEdit(null);
                }
              }}
              className="w-full bg-background border border-emerald-500 rounded px-2 py-1 text-xs text-foreground outline-none font-mono"
            />
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onSaveInline();
              }}
              className="px-2 py-1 bg-emerald-500 text-white rounded font-sans text-xs font-semibold cursor-pointer shrink-0"
            >
              Save (Enter)
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onSetInlineEdit(null);
              }}
              className="px-2 py-1 bg-surface border border-border text-muted-foreground hover:text-foreground rounded font-sans text-xs cursor-pointer shrink-0"
            >
              Cancel (Esc)
            </button>
          </div>
        ) : isEditingThis && inlineEdit.type === 'new-variant' ? (
          <div className="flex items-center gap-2 w-full bg-amber-500/10 p-2 rounded border border-amber-500/40">
            <div className="flex flex-col gap-1 w-full">
              <div className="flex items-center justify-between">
                <span className="text-[9px] uppercase font-bold text-amber-500">New Variant:</span>
                <button
                  type="button"
                  onClick={() => onSetInlineEdit(null)}
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
                  onChange={(e) => onSetInlineEdit({ ...inlineEdit, value: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      e.stopPropagation();
                      onSaveInline();
                    } else if (e.key === 'Escape') {
                      e.preventDefault();
                      e.stopPropagation();
                      onSetInlineEdit(null);
                    }
                  }}
                  className="w-1/2 bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground outline-none font-mono"
                />
                <input
                  type="text"
                  placeholder="Initial Value..."
                  value={inlineEdit.extraValue || ''}
                  onChange={(e) => onSetInlineEdit({ ...inlineEdit, extraValue: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      e.stopPropagation();
                      onSaveInline();
                    } else if (e.key === 'Escape') {
                      e.preventDefault();
                      e.stopPropagation();
                      onSetInlineEdit(null);
                    }
                  }}
                  className="w-1/2 bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground outline-none font-mono"
                />
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onSaveInline();
                  }}
                  className="px-2.5 py-1 bg-amber-500 text-black font-bold rounded text-xs cursor-pointer shrink-0"
                >
                  Add
                </button>
                <button
                  type="button"
                  onClick={() => onSetInlineEdit(null)}
                  className="px-2 py-1 bg-surface border border-border text-muted-foreground hover:text-foreground rounded text-xs cursor-pointer shrink-0"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        ) : (
          <div
            onDoubleClick={() => onSetInlineEdit({ type: 'value', varIndex: index, value: valStr })}
            onClick={() => onSetInlineEdit({ type: 'value', varIndex: index, value: valStr })}
            className="w-full px-2 py-1 bg-background border border-border/80 rounded flex items-center justify-between text-muted-foreground hover:text-foreground hover:border-primary/50 transition-colors cursor-text"
            title="Click or press Enter to edit value"
          >
            <span className="truncate max-w-[420px]">
              {valStr ? (isSecretVal ? '••••••••' : valStr) : <span className="opacity-40 italic">&lt;empty&gt;</span>}
            </span>
            <span className="text-[10px] text-muted-foreground/60 font-sans">Press Enter to edit</span>
          </div>
        )}
      </div>
    </div>
  );
};
