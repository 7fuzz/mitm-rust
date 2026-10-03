import React from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl, Switch } from '../../common/ui';
import type { PayloadSet } from '../../../services/tauri/bridge';
import { FuzzerSection, fieldInputClass } from './FuzzerSection';

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
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <span className="flex-1 truncate text-2xs font-semibold text-foreground">{label}</span>
        <span className="text-3xs font-mono text-muted-foreground tabular-nums">{countValues(set)} values</span>
        <SegmentedControl
          sizeVariant="xs"
          value={set.kind}
          onChange={(kind) => setPayloadSet(index, { kind: kind as PayloadSet['kind'] })}
          options={[
            { value: 'list', label: 'List' },
            { value: 'numbers', label: 'Numbers' },
          ]}
        />
      </div>

      {set.kind === 'list' ? (
        <textarea
          value={set.list.join('\n')}
          onChange={(e) => setPayloadSet(index, { list: e.target.value.split('\n').filter((l) => l.length > 0) })}
          placeholder={'admin\nroot\ntest'}
          rows={5}
          className={`${fieldInputClass} p-2 resize-y`}
        />
      ) : (
        <div className="grid grid-cols-4 gap-2">
          {(['from', 'to', 'step', 'pad'] as const).map((field) => (
            <label key={field} className="flex flex-col gap-1">
              <span className="text-3xs text-muted-foreground uppercase tracking-wider">{field}</span>
              <input
                type="number"
                value={set[field]}
                onChange={(e) => setPayloadSet(index, { [field]: Number(e.target.value) } as Partial<PayloadSet>)}
                className={fieldInputClass}
              />
            </label>
          ))}
        </div>
      )}

      <div className="flex items-center gap-2">
        <input
          type="text"
          value={set.prefix}
          onChange={(e) => setPayloadSet(index, { prefix: e.target.value })}
          placeholder="Prefix"
          className={`${fieldInputClass} flex-1 min-w-0`}
        />
        <input
          type="text"
          value={set.suffix}
          onChange={(e) => setPayloadSet(index, { suffix: e.target.value })}
          placeholder="Suffix"
          className={`${fieldInputClass} flex-1 min-w-0`}
        />
        <label className="flex items-center gap-1.5 text-2xs text-muted-foreground cursor-pointer select-none shrink-0">
          <Switch checked={set.urlEncode} onChange={() => setPayloadSet(index, { urlEncode: !set.urlEncode })} />
          URL-encode
        </label>
      </div>
    </div>
  );
};

export const FuzzerPayloadPanel: React.FC = () => {
  const { attackType, setAttackType, positions, payloadSets, config, setConfig } = useFuzzerStore();

  const updateRule = (i: number, patch: Partial<(typeof config.matchRules)[number]>) =>
    setConfig({ matchRules: config.matchRules.map((r, j) => (j === i ? { ...r, ...patch } : r)) });

  return (
    <>
      <FuzzerSection
        title="Payloads"
        right={
          <SegmentedControl
            sizeVariant="xs"
            value={attackType}
            onChange={(v) => setAttackType(v as typeof attackType)}
            options={[
              { value: 'sniper', label: 'Sniper' },
              { value: 'clusterbomb', label: 'Cluster bomb' },
            ]}
          />
        }
        bodyClassName="p-2.5 space-y-3"
      >
        <p className="text-3xs text-muted-foreground">
          {attackType === 'sniper'
            ? 'Sniper: one payload set, applied to each position in turn.'
            : 'Cluster bomb: one set per position, every combination.'}
        </p>
        {attackType === 'sniper' ? (
          <PayloadSetEditor index={0} label="Payload set" />
        ) : positions.length === 0 ? (
          <p className="text-2xs text-muted-foreground italic">Add positions to configure payload sets.</p>
        ) : (
          payloadSets.map((_, i) => (
            <div key={i} className={i > 0 ? 'pt-3 border-t border-border' : ''}>
              <PayloadSetEditor
                index={i}
                label={`Position ${i + 1}${positions[i] ? ` · ${positions[i].field.replace(/^(param|header):/, '')}` : ''}`}
              />
            </div>
          ))
        )}
      </FuzzerSection>

      <FuzzerSection
        title="Match rules"
        right={
          <button
            onClick={() => setConfig({ matchRules: [...config.matchRules, { name: `match${config.matchRules.length + 1}`, kind: 'contains', pattern: '' }] })}
            className="flex items-center gap-1 px-1.5 py-0.5 rounded text-2xs text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer"
          >
            <MingCuteIcon name="plus_line" size={12} />
            Add
          </button>
        }
      >
        {config.matchRules.map((rule, i) => (
          <div key={i} className="flex items-center gap-1.5">
            <input
              type="text"
              value={rule.name}
              onChange={(e) => updateRule(i, { name: e.target.value })}
              placeholder="column"
              className={`${fieldInputClass} w-24 shrink-0`}
            />
            <SegmentedControl
              sizeVariant="xs"
              value={rule.kind}
              onChange={(kind) => updateRule(i, { kind: kind as 'contains' | 'regex' })}
              options={[
                { value: 'contains', label: 'count' },
                { value: 'regex', label: 'regex' },
              ]}
            />
            <input
              type="text"
              value={rule.pattern}
              onChange={(e) => updateRule(i, { pattern: e.target.value })}
              placeholder={rule.kind === 'regex' ? '"token":"(\\w+)"' : 'error'}
              className={`${fieldInputClass} flex-1 min-w-0`}
            />
            <button
              onClick={() => setConfig({ matchRules: config.matchRules.filter((_, j) => j !== i) })}
              className="p-1 rounded text-muted-foreground hover:text-rose-500 hover:bg-neutral-subtle cursor-pointer shrink-0"
              title="Remove rule"
            >
              <MingCuteIcon name="delete_2_line" size={12} />
            </button>
          </div>
        ))}
        {config.matchRules.length === 0 && (
          <p className="text-3xs text-muted-foreground">
            Optional. Adds a results column that counts a string or extracts a regex group from each response.
          </p>
        )}
      </FuzzerSection>

      <FuzzerSection title="Options" bodyClassName="p-2.5 grid grid-cols-2 gap-3">
        <label className="flex flex-col gap-1">
          <span className="text-3xs text-muted-foreground uppercase tracking-wider">Concurrency</span>
          <input
            type="number"
            min={1}
            max={200}
            value={config.concurrency}
            onChange={(e) => setConfig({ concurrency: Math.max(1, Math.min(200, Number(e.target.value) || 1)) })}
            className={fieldInputClass}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-3xs text-muted-foreground uppercase tracking-wider">Delay (ms)</span>
          <input
            type="number"
            min={0}
            value={config.delayMs}
            onChange={(e) => setConfig({ delayMs: Math.max(0, Number(e.target.value) || 0) })}
            className={fieldInputClass}
          />
        </label>
      </FuzzerSection>
    </>
  );
};
