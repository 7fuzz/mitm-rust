import React, { useEffect } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import type { SourceScope } from '../../../types';
import { Switch } from './Switch';

export interface SourceScopeSectionProps {
  value: SourceScope;
  onChange: (scope: SourceScope) => void;
  /** Feature being scoped; drives the copy */
  verb: 'intercept' | 'rewrite';
  className?: string;
}

/** Section for limiting a feature to traffic from selected proxy listeners (sources). */
export const SourceScopeSection: React.FC<SourceScopeSectionProps> = ({ value, onChange, verb, className = '' }) => {
  const { listeners, fetchListeners } = useProxyStore();

  useEffect(() => {
    if (listeners.length === 0) fetchListeners();
  }, [listeners.length, fetchListeners]);

  const toggleId = (id: number) =>
    onChange({
      ...value,
      ids: value.ids.includes(id) ? value.ids.filter((v) => v !== id) : [...value.ids, id],
    });

  const done = verb === 'intercept' ? 'intercepted' : 'rewritten';
  const staleIds = value.ids.filter((id) => !listeners.some((l) => l.id === id));
  const noneSelected = !value.all && value.ids.every((id) => staleIds.includes(id));

  return (
    <div className={`bg-surface border border-border rounded-lg text-xs select-none ${className}`}>
      <div className="flex items-center justify-between gap-2 px-3 py-2">
        <div className="min-w-0">
          <div className="font-semibold text-foreground">Sources</div>
          <div className="text-[11px] text-muted-foreground truncate">
            {value.all ? `Traffic from every listener is ${done}` : `Only checked listeners are ${done}`}
          </div>
        </div>
        <label className="flex items-center gap-1.5 shrink-0 cursor-pointer">
          <span className="text-[11px] font-medium text-muted-foreground whitespace-nowrap">
            {verb.charAt(0).toUpperCase() + verb.slice(1)} all
          </span>
          <Switch
            checked={value.all}
            onChange={() => onChange({ ...value, all: !value.all })}
            title={value.all ? 'Turn off to pick specific sources' : `${verb.charAt(0).toUpperCase() + verb.slice(1)} traffic from every source`}
          />
        </label>
      </div>

      {!value.all && (
        <div className="border-t border-border px-3 py-2 space-y-1">
          {listeners.length === 0 && staleIds.length === 0 && (
            <div className="text-[11px] text-muted-foreground">No listeners configured.</div>
          )}
          {listeners.map((l) => (
            <label
              key={l.id}
              className={`flex items-center gap-2 cursor-pointer rounded px-1 py-0.5 hover:bg-neutral-subtle ${l.enabled ? '' : 'opacity-60'}`}
              title={l.enabled ? undefined : 'Listener is disabled'}
            >
              <input
                type="checkbox"
                checked={value.ids.includes(l.id)}
                onChange={() => toggleId(l.id)}
                className="rounded accent-primary cursor-pointer w-3.5 h-3.5"
              />
              <span className="font-medium text-foreground truncate">{l.label}</span>
              <span className="ml-auto font-mono text-[10px] text-muted-foreground">{l.address}</span>
            </label>
          ))}
          {staleIds.map((id) => (
            <label
              key={`stale-${id}`}
              className="flex items-center gap-2 cursor-pointer rounded px-1 py-0.5 text-rose-400"
              title="This listener was removed and never matches. Uncheck to clear it."
            >
              <input
                type="checkbox"
                checked
                onChange={() => toggleId(id)}
                className="rounded accent-rose-500 cursor-pointer w-3.5 h-3.5"
              />
              <span className="line-through">Removed listener #{id}</span>
            </label>
          ))}
          {noneSelected && (
            <div className="text-[11px] text-amber-500 pt-0.5">No source checked, so nothing will be {done}.</div>
          )}
        </div>
      )}
    </div>
  );
};
