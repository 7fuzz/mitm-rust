import React from 'react';

export type TriState = 'include' | 'exclude' | 'neutral';

export interface TriStateItem {
  value: string;
  label: string;
}

export interface TriStateFilterProps {
  items: TriStateItem[] | readonly TriStateItem[];
  values: Record<string, TriState>;
  onChange: (value: string, nextState: TriState) => void;
  className?: string;
}

export const TriStateFilter: React.FC<TriStateFilterProps> = ({
  items,
  values,
  onChange,
  className = '',
}) => {
  const handleItemClick = (val: string) => {
    const currentState = values[val] || 'neutral';
    const nextState: TriState =
      currentState === 'neutral'
        ? 'include'
        : currentState === 'include'
        ? 'exclude'
        : 'neutral';
    onChange(val, nextState);
  };

  return (
    <div className={`inline-flex items-center gap-1 bg-surface border border-border p-0.5 rounded-lg shadow-2xs font-mono select-none ${className}`}>
      {items.map((item) => {
        const state = values[item.value] || 'neutral';

        const stateClasses =
          state === 'include'
            ? 'bg-emerald-500/20 text-emerald-700 dark:text-emerald-300 border-emerald-500/40 font-extrabold shadow-2xs'
            : state === 'exclude'
            ? 'bg-rose-500/20 text-rose-700 dark:text-rose-300 border-rose-500/40 font-extrabold line-through opacity-85'
            : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle border-transparent font-semibold';

        return (
          <button
            key={item.value}
            type="button"
            onClick={() => handleItemClick(item.value)}
            className={`px-1.5 py-0.5 rounded text-[10px] uppercase border transition-all cursor-pointer ${stateClasses}`}
            title={`${item.label}: Click to toggle (${state === 'include' ? 'Green = Whitelisted' : state === 'exclude' ? 'Red = Excluded' : 'Neutral'})`}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
};
