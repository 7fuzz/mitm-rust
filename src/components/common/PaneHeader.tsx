import React from 'react';

export interface PaneHeaderTab {
  value: string;
  label: string;
  /** Shown next to the label when non-zero */
  count?: number;
}

interface PaneHeaderProps {
  /** Small uppercase label before the tabs, e.g. REQUEST */
  title?: string;
  tabs: PaneHeaderTab[];
  activeTab: string;
  onTabChange: (value: string) => void;
  /** Controls aligned right, e.g. the body format switch */
  right?: React.ReactNode;
}

/** Single header row per pane: title, underline tabs with counts, and optional right-aligned controls. */
export const PaneHeader: React.FC<PaneHeaderProps> = ({ title, tabs, activeTab, onTabChange, right }) => (
  <div className="bg-header border-b border-border px-2 flex items-center gap-3 shrink-0 select-none h-8">
    {title && (
      <>
        <span className="font-semibold text-muted-foreground text-[10px] uppercase tracking-wider shrink-0">{title}</span>
        <div className="h-4 w-px bg-border shrink-0" />
      </>
    )}
    <div className="flex items-stretch h-full gap-0.5 shrink-0">
      {tabs.map((tab) => {
        const isActive = tab.value === activeTab;
        return (
          <button
            key={tab.value}
            onClick={() => onTabChange(tab.value)}
            className={`px-2.5 text-[11px] transition-colors cursor-pointer whitespace-nowrap rounded-t ${
              isActive
                ? // Inset shadow as the underline so the scroll container can't clip it
                  'bg-surface text-foreground font-semibold shadow-[inset_0_-2px_0_var(--color-primary)]'
                : 'text-muted-foreground font-medium hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            {tab.label}
            {tab.count ? <span className="ml-1 text-[10px] text-muted-foreground font-mono">{tab.count}</span> : null}
          </button>
        );
      })}
    </div>
    {right && <div className="ml-auto shrink-0 flex items-center gap-2">{right}</div>}
  </div>
);
