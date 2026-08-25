import React from 'react';

export interface CheckboxProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Checkbox: React.FC<CheckboxProps> = ({ label, className = '', id, ...props }) => {
  const generatedId = id || React.useId();

  return (
    <label htmlFor={generatedId} className="inline-flex items-center gap-1.5 cursor-pointer select-none text-xs text-foreground">
      <input
        type="checkbox"
        id={generatedId}
        className={`w-3.5 h-3.5 rounded border-slate-300 dark:border-zinc-700 text-primary focus:ring-primary/50 cursor-pointer ${className}`}
        {...props}
      />
      {label && <span className="font-medium">{label}</span>}
    </label>
  );
};
