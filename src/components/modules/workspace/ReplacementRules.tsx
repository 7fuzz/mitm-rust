import React, { useState } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Select } from '../../common/ui';

const TARGET_OPTIONS = [
  { value: 'body', label: 'Body String' },
  { value: 'header', label: 'Header Value' },
  { value: 'url', label: 'URL String' },
] as const;

export const ReplacementRules: React.FC = () => {
  const { replacements, addReplacement, toggleReplacement, deleteReplacementRule } =
    useWorkspaceStore();

  const [domain, setDomain] = useState('*');
  const [target, setTarget] = useState<'header' | 'body' | 'url'>('body');
  const [pattern, setPattern] = useState('');
  const [replacementStr, setReplacementStr] = useState('');
  const [isRegex, setIsRegex] = useState(false);

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pattern.trim()) return;
    await addReplacement({
      domain: domain.trim() || '*',
      target,
      isRegex,
      pattern: pattern.trim(),
      replacement: replacementStr,
      enabled: true,
    });
    setPattern('');
    setReplacementStr('');
  };

  return (
    <div className="bg-surface border border-border rounded-lg p-3 space-y-3 text-xs">
      <div className="flex items-center gap-2">
        <MingCuteIcon name="refresh_line" size={16} className="text-primary" />
        <span className="font-semibold text-foreground text-sm">Automated Traffic Replacement Rules</span>
      </div>

      <div className="border border-border rounded overflow-hidden">
        <table className="w-full text-left font-mono text-xs">
          <thead>
            <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
              <th className="w-8 px-2 py-1.5 text-center">Active</th>
              <th className="px-3 py-1.5">Match Domain</th>
              <th className="px-3 py-1.5">Target</th>
              <th className="px-3 py-1.5">Pattern</th>
              <th className="px-3 py-1.5">Replacement</th>
              <th className="w-12 px-2 py-1.5 text-center">Action</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {replacements.length === 0 ? (
              <tr>
                <td colSpan={6} className="px-3 py-4 text-center text-muted-foreground italic font-sans">
                  No automated replacement rules configured
                </td>
              </tr>
            ) : (
              replacements.map((r) => (
                <tr key={r.id} className="hover:bg-neutral-subtle/50">
                  <td className="px-2 py-1.5 text-center">
                    <input
                      type="checkbox"
                      checked={r.enabled}
                      onChange={() => toggleReplacement(r.id)}
                      className="rounded border-border text-primary"
                    />
                  </td>
                  <td className="px-3 py-1.5 font-bold">{r.domain}</td>
                  <td className="px-3 py-1.5 uppercase font-semibold text-primary">{r.target}</td>
                  <td className="px-3 py-1.5 text-rose-500 font-bold">{r.pattern}</td>
                  <td className="px-3 py-1.5 text-emerald-500 font-bold">{r.replacement}</td>
                  <td className="px-2 py-1.5 text-center">
                    <button onClick={() => deleteReplacementRule(r.id)} className="text-muted-foreground hover:text-rose-500">
                      <MingCuteIcon name="delete_2_line" size={14} />
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Add Form */}
      <form onSubmit={handleAdd} className="grid grid-cols-6 gap-2 pt-1 font-mono">
        <input
          type="text"
          value={domain}
          onChange={(e) => setDomain(e.target.value)}
          placeholder="Domain (*)"
          className="bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <Select
          value={target}
          onChange={(e) => setTarget(e.target.value as any)}
          options={TARGET_OPTIONS}
        />
        <input
          type="text"
          value={pattern}
          onChange={(e) => setPattern(e.target.value)}
          placeholder="Match Pattern"
          className="bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <input
          type="text"
          value={replacementStr}
          onChange={(e) => setReplacementStr(e.target.value)}
          placeholder="Replacement String"
          className="bg-background border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <label className="flex items-center gap-1 text-[11px] text-muted-foreground cursor-pointer select-none font-sans">
          <input
            type="checkbox"
            checked={isRegex}
            onChange={(e) => setIsRegex(e.target.checked)}
            className="rounded border-border text-primary"
          />
          Regex
        </label>
        <button
          type="submit"
          className="px-3 py-1 bg-primary text-primary-foreground text-xs font-sans font-medium rounded hover:bg-primary-hover transition-colors shrink-0"
        >
          Add Rule
        </button>
      </form>
    </div>
  );
};
