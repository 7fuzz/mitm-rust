import React from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl, Switch } from '../../common/ui';
import type { AttackType, PayloadSet } from '../../../services/tauri/bridge';
import { fieldInputClass, inputBaseClass } from './FuzzerSection';

export const countValues = (set: PayloadSet): number => {
  if (set.kind === 'numbers') {
    const step = set.step || 1;
    if (step === 0) return 0;
    const n = Math.floor((set.to - set.from) / step) + 1;
    return Math.max(0, n);
  }
  return set.list.length;
};

const ATTACK_HELP: Record<AttackType, string> = {
  sniper: 'Each variable is fuzzed in turn while the others hold their first payload.',
  pitchfork: 'All variables advance together, stopping at the shortest list.',
  clusterbomb: 'Every combination of the variables’ payloads.',
};

const parseList = (text: string) => text.split('\n').filter((l) => l.length > 0);

const PayloadListInput: React.FC<{ list: string[]; onChange: (list: string[]) => void }> = ({ list, onChange }) => {
  // Raw text is kept locally so blank lines survive while typing; only non-empty lines are stored.
  const [text, setText] = React.useState(() => list.join('\n'));

  React.useEffect(() => {
    if (parseList(text).join('\n') !== list.join('\n')) setText(list.join('\n'));
  }, [list]);

  return (
    <textarea
      value={text}
      onChange={(e) => {
        setText(e.target.value);
        onChange(parseList(e.target.value));
      }}
      placeholder={'admin\nroot\ntest'}
      rows={5}
      className={`${fieldInputClass} p-2 resize-y`}
    />
  );
};

const PayloadSetEditor: React.FC<{ name: string; set: PayloadSet }> = ({ name, set }) => {
  const setVariableSet = useFuzzerStore((s) => s.setVariableSet);
  const update = (patch: Partial<PayloadSet>) => setVariableSet(name, patch);

  return (
    <div className="border border-border rounded-lg bg-surface p-2.5 space-y-2">
      <div className="flex items-center gap-2">
        <span className="flex-1 truncate text-2xs font-mono font-semibold text-amber-500">{`{{${name}}}`}</span>
        <span className="text-3xs font-mono text-muted-foreground tabular-nums">{countValues(set)} values</span>
        <SegmentedControl
          sizeVariant="xs"
          value={set.kind}
          onChange={(kind) => update({ kind: kind as PayloadSet['kind'] })}
          options={[
            { value: 'list', label: 'List' },
            { value: 'numbers', label: 'Numbers' },
          ]}
        />
      </div>

      {set.kind === 'list' ? (
        <PayloadListInput list={set.list} onChange={(list) => update({ list })} />
      ) : (
        <div className="grid grid-cols-4 gap-2">
          {(['from', 'to', 'step', 'pad'] as const).map((field) => (
            <label key={field} className="flex flex-col gap-1">
              <span className="text-3xs text-muted-foreground uppercase tracking-wider">{field}</span>
              <input
                type="number"
                value={set[field]}
                onChange={(e) => update({ [field]: Number(e.target.value) } as Partial<PayloadSet>)}
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
          onChange={(e) => update({ prefix: e.target.value })}
          placeholder="Prefix"
          className={`${inputBaseClass} flex-1 min-w-0`}
        />
        <input
          type="text"
          value={set.suffix}
          onChange={(e) => update({ suffix: e.target.value })}
          placeholder="Suffix"
          className={`${inputBaseClass} flex-1 min-w-0`}
        />
        <label className="flex items-center gap-1.5 text-2xs text-muted-foreground cursor-pointer select-none shrink-0">
          <Switch checked={set.urlEncode} onChange={() => update({ urlEncode: !set.urlEncode })} />
          URL-encode
        </label>
      </div>
    </div>
  );
};

export const PayloadsConfig: React.FC = () => {
  const { attackType, setAttackType, variables } = useFuzzerStore();

  return (
    <div className="space-y-2.5">
      <div className="flex items-center gap-3">
        <SegmentedControl
          sizeVariant="xs"
          value={attackType}
          onChange={(v) => setAttackType(v as AttackType)}
          options={[
            { value: 'sniper', label: 'Sniper' },
            { value: 'pitchfork', label: 'Pitchfork' },
            { value: 'clusterbomb', label: 'Cluster bomb' },
          ]}
        />
        <span className="text-3xs text-muted-foreground">{ATTACK_HELP[attackType]}</span>
      </div>

      {variables.length === 0 ? (
        <p className="text-2xs text-muted-foreground italic">
          Add a <span className="font-mono text-amber-500">{'{{variable}}'}</span> to the request to configure its payloads.
        </p>
      ) : (
        <div className="grid gap-2.5 [grid-template-columns:repeat(auto-fill,minmax(280px,1fr))]">
          {variables.map((v) => (
            <PayloadSetEditor key={v.name} name={v.name} set={v.set} />
          ))}
        </div>
      )}
    </div>
  );
};

export const MatchRulesConfig: React.FC = () => {
  const { config, setConfig } = useFuzzerStore();

  const updateRule = (i: number, patch: Partial<(typeof config.matchRules)[number]>) =>
    setConfig({ matchRules: config.matchRules.map((r, j) => (j === i ? { ...r, ...patch } : r)) });

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <p className="text-3xs text-muted-foreground">
          Optional. Each rule adds a results column counting a string or extracting a regex group.
        </p>
        <button
          onClick={() => setConfig({ matchRules: [...config.matchRules, { name: `match${config.matchRules.length + 1}`, kind: 'contains', pattern: '' }] })}
          className="flex items-center gap-1 px-1.5 py-0.5 rounded text-2xs text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer shrink-0"
        >
          <MingCuteIcon name="plus_line" size={12} />
          Add
        </button>
      </div>
      {config.matchRules.map((rule, i) => (
        <div key={i} className="flex items-center gap-1.5">
          <input
            type="text"
            value={rule.name}
            onChange={(e) => updateRule(i, { name: e.target.value })}
            placeholder="column"
            className={`${inputBaseClass} w-24 shrink-0`}
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
            className={`${inputBaseClass} flex-1 min-w-0`}
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
    </div>
  );
};

export const OptionsConfig: React.FC = () => {
  const { config, setConfig } = useFuzzerStore();

  return (
    <div className="grid grid-cols-2 gap-3 max-w-md">
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
    </div>
  );
};
