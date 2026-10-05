import React from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import { Select, type SelectOption } from './ui';

interface StepListProps {
  title: string;
  ids: string[];
  options: SelectOption[];
  onChange: (ids: string[]) => void;
}

const StepList: React.FC<StepListProps> = ({ title, ids, options, onChange }) => {
  const labelOf = (id: string) => options.find((o) => o.value === id)?.label ?? id;

  const move = (from: number, to: number) => {
    const next = [...ids];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    onChange(next);
  };

  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-2xs font-semibold uppercase tracking-wider text-muted-foreground">{title}</span>
      {ids.map((id, idx) => (
        <div
          key={`${id}-${idx}`}
          className="flex items-center gap-2 px-2 py-1 rounded border border-border bg-background text-xs"
        >
          <span className="text-muted-foreground font-mono w-4 text-right shrink-0">{idx + 1}</span>
          <span className="flex-1 truncate font-mono" title={labelOf(id)}>{labelOf(id)}</span>
          <button
            type="button"
            onClick={() => move(idx, idx - 1)}
            disabled={idx === 0}
            className="p-0.5 text-muted-foreground hover:text-foreground rounded cursor-pointer disabled:opacity-30 disabled:cursor-default"
            title="Move up"
          >
            <MingCuteIcon name="up_line" size={14} />
          </button>
          <button
            type="button"
            onClick={() => move(idx, idx + 1)}
            disabled={idx === ids.length - 1}
            className="p-0.5 text-muted-foreground hover:text-foreground rounded cursor-pointer disabled:opacity-30 disabled:cursor-default"
            title="Move down"
          >
            <MingCuteIcon name="down_line" size={14} />
          </button>
          <button
            type="button"
            onClick={() => onChange(ids.filter((_, i) => i !== idx))}
            className="p-0.5 text-muted-foreground hover:text-rose-400 rounded cursor-pointer"
            title="Remove"
          >
            <MingCuteIcon name="delete_2_line" size={14} />
          </button>
        </div>
      ))}
      <Select
        value=""
        onChange={(e) => e.target.value && onChange([...ids, e.target.value])}
        options={[{ value: '', label: 'Add request…' }, ...options]}
        sizeVariant="sm"
        className="min-w-72 max-w-xl"
      />
    </div>
  );
};

interface RequestStepsEditorProps {
  preRequests: string[];
  postRequests: string[];
  options: SelectOption[];
  onChange: (steps: { preRequests: string[]; postRequests: string[] }) => void;
}

export const RequestStepsEditor: React.FC<RequestStepsEditorProps> = ({ preRequests, postRequests, options, onChange }) => (
  <div className="flex flex-col gap-5 max-w-2xl">
    <StepList
      title="Pre-requests"
      ids={preRequests}
      options={options}
      onChange={(ids) => onChange({ preRequests: ids, postRequests })}
    />
    <StepList
      title="Post-requests"
      ids={postRequests}
      options={options}
      onChange={(ids) => onChange({ preRequests, postRequests: ids })}
    />
  </div>
);
