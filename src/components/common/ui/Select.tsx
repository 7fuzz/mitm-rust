import React from 'react';
import { MingCuteIcon } from '../MingCuteIcon';

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'children'> {
  options: SelectOption[] | readonly SelectOption[];
  sizeVariant?: 'xs' | 'sm' | 'md';
}

export const Select: React.FC<SelectProps> = ({
  options,
  value,
  onChange,
  className = '',
  sizeVariant = 'xs',
  disabled,
  ...props
}) => {
  const sizeClasses =
    sizeVariant === 'xs'
      ? 'px-2 py-1 text-xs'
      : sizeVariant === 'sm'
      ? 'px-2.5 py-1.5 text-xs'
      : 'px-3 py-2 text-sm';

  return (
    <div className="relative inline-flex items-center">
      <select
        value={value}
        onChange={onChange}
        disabled={disabled}
        className={`bg-white dark:bg-zinc-900 text-slate-900 dark:text-zinc-100 border border-slate-300 dark:border-zinc-700 rounded font-mono font-medium focus:outline-none focus:border-primary cursor-pointer pr-7 transition-colors appearance-none disabled:opacity-50 disabled:cursor-not-allowed ${sizeClasses} ${className}`}
        {...props}
      >
        {options.map((opt) => (
          <option
            key={opt.value}
            value={opt.value}
            disabled={opt.disabled}
            className="bg-white dark:bg-zinc-900 text-slate-900 dark:text-zinc-100 font-sans py-1"
          >
            {opt.label}
          </option>
        ))}
      </select>
      <MingCuteIcon
        name="chevron_down_line"
        size={13}
        className="absolute right-2 text-slate-500 dark:text-zinc-400 pointer-events-none"
      />
    </div>
  );
};
