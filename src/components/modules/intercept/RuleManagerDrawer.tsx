import React, { useState } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Select } from '../../common/ui';

const TARGET_OPTIONS = [
  { value: 'domain', label: 'Domain' },
  { value: 'path', label: 'Path Pattern' },
  { value: 'method', label: 'HTTP Method' },
] as const;

const ACTION_OPTIONS = [
  { value: 'intercept', label: 'Intercept (Whitelist)' },
  { value: 'pass', label: 'Pass (Blacklist)' },
] as const;

interface RuleManagerDrawerProps {
  isMinimized: boolean;
  onToggleMinimize: () => void;
  widthPx: number;
}

export const RuleManagerDrawer: React.FC<RuleManagerDrawerProps> = ({
  isMinimized,
  onToggleMinimize,
  widthPx,
}) => {
  const { interceptRules, addInterceptRule, toggleInterceptRule, deleteInterceptRule } = useProxyStore();

  const [target, setTarget] = useState<'domain' | 'path' | 'method'>('domain');
  const [pattern, setPattern] = useState('');
  const [action, setAction] = useState<'intercept' | 'pass'>('intercept');

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pattern.trim()) return;
    await addInterceptRule({ target, pattern: pattern.trim(), action, enabled: true });
    setPattern('');
  };

  if (isMinimized) {
    return (
      <div
        onClick={onToggleMinimize}
        className="bg-surface hover:bg-neutral-subtle border-l border-border w-10 h-full flex flex-col items-center py-3 overflow-hidden text-xs shrink-0 select-none cursor-pointer transition-colors group"
        title="Click anywhere to expand Intercept Rule Manager"
      >
        <button
          className="p-1.5 rounded text-muted-foreground group-hover:text-primary transition-colors mb-4"
        >
          <MingCuteIcon name="chevron_left_line" size={16} />
        </button>

        <div className="flex-1 flex items-center justify-center">
          <span className="font-semibold text-muted-foreground group-hover:text-foreground text-2xs uppercase tracking-wider rotate-90 whitespace-nowrap transition-colors">
            Rules ({interceptRules.length})
          </span>
        </div>
      </div>
    );
  }

  return (
    <div
      className="bg-surface border-l border-border h-full flex flex-col overflow-hidden text-xs shrink-0"
      style={{ width: `${widthPx}px` }}
    >
      {/* Clickable Header Bar to Hide/Minimize */}
      <div
        onClick={onToggleMinimize}
        className="p-3 bg-header border-b border-border font-semibold flex items-center justify-between select-none cursor-pointer hover:bg-neutral-subtle transition-colors group"
        title="Click title to minimize Rule Manager"
      >
        <div className="flex items-center gap-2">
          <MingCuteIcon name="shield_line" size={16} className="text-primary" />
          <span className="group-hover:text-primary transition-colors">Intercept Rule Manager ({interceptRules.length})</span>
        </div>
        <button
          className="p-1 text-muted-foreground group-hover:text-foreground rounded"
        >
          <MingCuteIcon name="chevron_right_line" size={16} />
        </button>
      </div>

      {/* Rules Table */}
      <div className="flex-1 p-2 overflow-y-auto space-y-2">
        {interceptRules.length === 0 ? (
          <div className="py-8 text-center text-muted-foreground italic">No intercept rules defined</div>
        ) : (
          interceptRules.map((rule) => (
            <div
              key={rule.id}
              className={`p-2 border rounded transition-colors flex flex-col gap-1 ${
                rule.enabled ? 'border-border bg-background' : 'border-border/40 opacity-50 bg-neutral-subtle'
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 font-mono">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={() => toggleInterceptRule(rule.id)}
                    className="rounded border-border text-primary"
                  />
                  <span className="uppercase text-3xs font-bold text-muted-foreground">{rule.target}:</span>
                  <span className="font-medium text-foreground">{rule.pattern}</span>
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    deleteInterceptRule(rule.id);
                  }}
                  className="text-muted-foreground hover:text-rose-500 p-0.5 rounded cursor-pointer"
                >
                  <MingCuteIcon name="delete_2_line" size={13} />
                </button>
              </div>

              <div className="flex items-center gap-2 text-3xs">
                <span
                  className={`px-1.5 py-0.2 rounded font-mono uppercase border font-semibold ${
                    rule.action === 'intercept'
                      ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                      : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                  }`}
                >
                  {rule.action === 'intercept' ? 'Intercept (Whitelist)' : 'Pass (Blacklist)'}
                </span>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Add Rule Form */}
      <form onSubmit={handleAdd} className="p-3 bg-header border-t border-border space-y-2">
        <span className="font-semibold text-foreground text-2xs block">Add Rule (Whitelist / Blacklist)</span>
        <div className="grid grid-cols-2 gap-2">
          <Select
            value={target}
            onChange={(e) => setTarget(e.target.value as any)}
            options={TARGET_OPTIONS}
            className="w-full"
          />
          <Select
            value={action}
            onChange={(e) => setAction(e.target.value as any)}
            options={ACTION_OPTIONS}
            className="w-full"
          />
        </div>
        <input
          type="text"
          value={pattern}
          onChange={(e) => setPattern(e.target.value)}
          placeholder="e.g. *.stripe.com or /api/v1/*"
          className="w-full bg-background border border-border rounded px-2 py-1 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
        />
        <button
          type="submit"
          className="w-full py-1.5 bg-primary text-primary-foreground font-medium rounded text-xs hover:bg-primary-hover transition-colors flex items-center justify-center gap-1 cursor-pointer"
        >
          <MingCuteIcon name="plus_line" size={14} />
          Add Rule
        </button>
      </form>
    </div>
  );
};
