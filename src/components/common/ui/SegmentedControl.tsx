
export interface SegmentOption<T extends string = string> {
  value: T;
  label: string;
}

export interface SegmentedControlProps<T extends string = string> {
  options: SegmentOption<T>[] | readonly SegmentOption<T>[];
  value?: T;
  selectedValues?: T[];
  onChange?: (value: T) => void;
  onToggle?: (value: T) => void;
  multiSelect?: boolean;
  sizeVariant?: 'xs' | 'sm';
  label?: string;
  className?: string;
}

export function SegmentedControl<T extends string = string>({
  options,
  value,
  selectedValues,
  onChange,
  onToggle,
  multiSelect = false,
  sizeVariant = 'xs',
  label,
  className = '',
}: SegmentedControlProps<T>) {
  const sizeClasses = sizeVariant === 'xs' ? 'px-2 py-0.5 text-[10px]' : 'px-2.5 py-1 text-xs';

  return (
    <div className={`inline-flex items-center gap-1 bg-surface border border-border p-0.5 rounded-lg shadow-2xs font-mono select-none ${className}`}>
      {label && (
        <span className="text-[10px] text-muted-foreground px-1.5 uppercase font-sans font-medium">
          {label}:
        </span>
      )}
      {options.map((opt) => {
        const isSelected = multiSelect
          ? selectedValues?.includes(opt.value)
          : value === opt.value;

        return (
          <button
            key={opt.value}
            type="button"
            onClick={() => {
              if (multiSelect && onToggle) {
                onToggle(opt.value);
              } else if (onChange) {
                onChange(opt.value);
              }
            }}
            className={`rounded font-bold uppercase transition-colors cursor-pointer ${sizeClasses} ${
              isSelected
                ? 'bg-primary text-primary-foreground font-extrabold shadow-2xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
