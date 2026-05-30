import { memo } from 'react';

export type FilterState = 'include' | 'exclude' | undefined;

interface MultiSelectFilterProps {
  options: string[];
  filterStates: Record<string, FilterState>;
  onToggle: (option: string) => void;
  onClear: () => void;
  onToggleVisibility?: () => void;
  isVisible?: boolean;
  className?: string;
}

export const MultiSelectFilter = memo(({
  options,
  filterStates,
  onToggle,
  onClear,
  onToggleVisibility,
  isVisible = true,
  className = ''
}: MultiSelectFilterProps) => {
  const hasActiveFilters = Object.values(filterStates).some(v => v !== undefined);

  if (!isVisible) {
    return null;
  }

  return (
    <div className={className}>
      <div className="flex gap-1 overflow-x-auto pb-1 scrollbar-hide">
        {options.map((option) => {
          const state = filterStates[option];
          return (
            <button
              key={option}
              onClick={() => onToggle(option)}
              title={state === 'include' ? 'Include only' : state === 'exclude' ? 'Exclude' : 'No filter'}
              className={`px-2 py-1 text-[9px] uppercase font-bold tracking-widest rounded transition-all whitespace-nowrap ${
                state === 'include'
                  ? 'bg-emerald-600 text-white border border-emerald-500'
                  : state === 'exclude'
                  ? 'bg-rose-600 text-white border border-rose-500'
                  : 'bg-zinc-950 text-zinc-500 border border-zinc-800 hover:text-zinc-300 hover:border-zinc-700'
              }`}
            >
              {option}
            </button>
          );
        })}
      </div>
      {(hasActiveFilters || onToggleVisibility) && (
        <div className="flex gap-1 mt-2">
          {hasActiveFilters && (
            <button
              onClick={onClear}
              className="px-2 py-1 text-[8px] uppercase font-bold tracking-widest rounded bg-zinc-900 text-zinc-400 border border-zinc-700 hover:text-zinc-200 hover:border-zinc-600 transition-all"
            >
              Clear Filter
            </button>
          )}
        </div>
      )}
    </div>
  );
});

MultiSelectFilter.displayName = 'MultiSelectFilter';
