import React from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl } from '../../common/ui';
import type { PayloadSet } from '../../../services/tauri/bridge';

const countValues = (set: PayloadSet): number => {
  if (set.kind === 'numbers') {
    const step = set.step || 1;
    if (step === 0) return 0;
    const n = Math.floor((set.to - set.from) / step) + 1;
    return Math.max(0, n);
  }
  return set.list.length;
};

const PayloadSetEditor: React.FC<{ index: number; label: string }> = ({ index, label }) => {
  const set = useFuzzerStore((s) => s.payloadSets[index]);
  const setPayloadSet = useFuzzerStore((s) => s.setPayloadSet);
  if (!set) return null;

  return (
    <div className="border border-border rounded-lg bg-background overflow-hidden">
      <div className="px-2.5 py-1.5 bg-header border-b border-border flex items-center gap-2">
        <span className="font-semibold text-foreground text-2xs flex-1">{label}</span>
        <SegmentedControl
          sizeVariant="xs"
          value={set.kind}
          onChange={(kind) => setPayloadSet(index, { kind: kind as PayloadSet['kind'] })}
          options={[
            { value: 'list', label: 'List' },
            { value: 'numbers', label: 'Numbers' },
          ]}
        />
        <span className="text-3xs font-mono text-muted-foreground tabular-nums">{countValues(set)} values</span>
      </div>

      <div className="p-2 space-y-2">
        {set.kind === 'list' ? (
          <textarea
            value={set.list.join('\n')}
            onChange={(e) => setPayloadSet(index, { list: e.target.value.split('\n').filter((l) => l.length > 0) })}
            placeholder={'one payload per line\nadmin\nroot\ntest'}
            rows={5}
            className="w-full bg-background border border-border rounded p-2 font-mono text-2xs text-foreground focus:outline-none focus:border-primary resize-y"
          />
        ) : (
          <div className="grid grid-cols-4 gap-2 font-mono text-2xs">
            {(['from', 'to', 'step', 'pad'] as const).map((field) => (
              <label key={field} className="flex flex-col gap-0.5">
                <span className="text-muted-foreground capitalize">{field}</span>
                <input
                  type="number"
                  value={set[field]}
                  onChange={(e) => setPayloadSet(index, { [field]: Number(e.target.value) } as Partial<PayloadSet>)}
                  className="bg-background border border-border rounded px-1.5 py-1 text-foreground focus:outline-none focus:border-primary"
                />
              </label>
            ))}
          </div>
        )}

        <div className="flex items-center gap-2 flex-wrap text-2xs font-mono">
          <input
            type="text"
            value={set.prefix}
            onChange={(e) => setPayloadSet(index, { prefix: e.target.value })}
            placeholder="prefix"
            className="w-20 bg-background border border-border rounded px-1.5 py-0.5 text-foreground focus:outline-none focus:border-primary"
          />
          <input
            type="text"
            value={set.suffix}
            onChange={(e) => setPayloadSet(index, { suffix: e.target.value })}
            placeholder="suffix"
            className="w-20 bg-background border border-border rounded px-1.5 py-0.5 text-foreground focus:outline-none focus:border-primary"
          />
          <label className="flex items-center gap-1 text-muted-foreground cursor-pointer select-none ml-auto">
            <input
              type="checkbox"
              checked={set.urlEncode}
              onChange={(e) => setPayloadSet(index, { urlEncode: e.target.checked })}
              className="accent-primary h-3 w-3 cursor-pointer"
            />
            URL-encode
          </label>
        </div>
      </div>
    </div>
  );
};

export const FuzzerPayloadPanel: React.FC = () => {
  const { attackType, setAttackType, positions, payloadSets, config, setConfig } = useFuzzerStore();

  return (
    <div className="space-y-3 text-xs">
      <div className="flex items-center gap-2">
        <span className="text-2xs text-muted-foreground w-20 shrink-0">Attack</span>
        <SegmentedControl
          sizeVariant="sm"
          value={attackType}
          onChange={(v) => setAttackType(v as typeof attackType)}
          options={[
            { value: 'sniper', label: 'Sniper' },
            { value: 'clusterbomb', label: 'Cluster bomb' },
          ]}
        />
        <span className="text-3xs text-muted-foreground">
          {attackType === 'sniper' ? 'one set, each position in turn' : 'one set per position, every combination'}
        </span>
      </div>

      <div className="space-y-2">
        {attackType === 'sniper' ? (
          <PayloadSetEditor index={0} label="Payload set" />
        ) : positions.length === 0 ? (
          <div className="text-2xs text-muted-foreground italic px-1">Add positions to configure payload sets.</div>
        ) : (
          payloadSets.map((_, i) => (
            <PayloadSetEditor
              key={i}
              index={i}
              label={`Position ${i + 1}${positions[i] ? ` · ${positions[i].field.replace(/^(param|header):/, '')}` : ''}`}
            />
          ))
        )}
      </div>

      <div className="space-y-1.5">
        <div className="flex items-center justify-between">
          <span className="text-2xs font-semibold text-muted-foreground uppercase tracking-wider">Match rules</span>
          <button
            onClick={() => setConfig({ matchRules: [...config.matchRules, { name: `match${config.matchRules.length + 1}`, kind: 'contains', pattern: '' }] })}
            className="flex items-center gap-1 text-2xs text-muted-foreground hover:text-foreground cursor-pointer"
          >
            <MingCuteIcon name="plus_line" size={12} />
            Add
          </button>
        </div>
        {config.matchRules.map((rule, i) => (
          <div key={i} className="flex items-center gap-1.5 font-mono text-2xs">
            <input
              type="text"
              value={rule.name}
              onChange={(e) => setConfig({ matchRules: config.matchRules.map((r, j) => (j === i ? { ...r, name: e.target.value } : r)) })}
              placeholder="column"
              className="w-24 bg-background border border-border rounded px-1.5 py-0.5 text-foreground focus:outline-none focus:border-primary"
            />
            <SegmentedControl
              sizeVariant="xs"
              value={rule.kind}
              onChange={(kind) => setConfig({ matchRules: config.matchRules.map((r, j) => (j === i ? { ...r, kind: kind as 'contains' | 'regex' } : r)) })}
              options={[
                { value: 'contains', label: 'count' },
                { value: 'regex', label: 'regex' },
              ]}
            />
            <input
              type="text"
              value={rule.pattern}
              onChange={(e) => setConfig({ matchRules: config.matchRules.map((r, j) => (j === i ? { ...r, pattern: e.target.value } : r)) })}
              placeholder={rule.kind === 'regex' ? '"token":"(\\w+)"' : 'error'}
              className="flex-1 bg-background border border-border rounded px-1.5 py-0.5 text-foreground focus:outline-none focus:border-primary"
            />
            <button
              onClick={() => setConfig({ matchRules: config.matchRules.filter((_, j) => j !== i) })}
              className="p-0.5 rounded text-muted-foreground hover:text-rose-500 cursor-pointer"
            >
              <MingCuteIcon name="delete_2_line" size={12} />
            </button>
          </div>
        ))}
        {config.matchRules.length === 0 && (
          <p className="text-3xs text-muted-foreground italic px-1">
            Optional: add a column that counts a string (count) or extracts a regex group from each response.
          </p>
        )}
      </div>

      <div className="flex items-center gap-4 font-mono text-2xs pt-1 border-t border-border">
        <label className="flex items-center gap-1.5">
          <span className="text-muted-foreground">Concurrency</span>
          <input
            type="number"
            min={1}
            max={200}
            value={config.concurrency}
            onChange={(e) => setConfig({ concurrency: Math.max(1, Math.min(200, Number(e.target.value) || 1)) })}
            className="w-14 bg-background border border-border rounded px-1.5 py-0.5 text-foreground focus:outline-none focus:border-primary"
          />
        </label>
        <label className="flex items-center gap-1.5">
          <span className="text-muted-foreground">Delay ms</span>
          <input
            type="number"
            min={0}
            value={config.delayMs}
            onChange={(e) => setConfig({ delayMs: Math.max(0, Number(e.target.value) || 0) })}
            className="w-16 bg-background border border-border rounded px-1.5 py-0.5 text-foreground focus:outline-none focus:border-primary"
          />
        </label>
      </div>
    </div>
  );
};
