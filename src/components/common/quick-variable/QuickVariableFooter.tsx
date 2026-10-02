import React from 'react';

interface QuickVariableFooterProps {
  toast: string | null;
}

export const QuickVariableFooter: React.FC<QuickVariableFooterProps> = ({ toast }) => {
  return (
    <div className="px-3 py-1.5 bg-background border-t border-border flex items-center justify-between text-3xs font-mono text-muted-foreground select-none shrink-0">
      <div className="flex items-center gap-2.5 overflow-x-auto no-scrollbar py-0.5">
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">↑↓</kbd> Select</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">←→</kbd> Variant</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">Enter</kbd> Edit</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">n</kbd>/<kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">N</kbd> New Variant/Var</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">r</kbd>/<kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">R</kbd> Rename</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">d</kbd>/<kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">D</kbd> Delete</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">c</kbd>/<kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">C</kbd> Copy</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">/</kbd> Search</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">e</kbd>/<kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">E</kbd> Env</span>
        <span><kbd className="px-1 py-0.2 rounded bg-surface border border-border text-foreground font-bold">Esc</kbd> Close</span>
      </div>

      {toast && (
        <div className="bg-primary text-primary-foreground px-2 py-0.5 rounded font-sans text-3xs font-semibold animate-fade-in shrink-0 ml-2 shadow-2xs">
          {toast}
        </div>
      )}
    </div>
  );
};
