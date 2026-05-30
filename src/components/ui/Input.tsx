import React, { forwardRef } from 'react';

type InputVariant = 'default' | 'fuchsia' | 'sky' | 'amber' | 'emerald';

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  variant?: InputVariant;
  className?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(({
  variant = 'default',
  className = '',
  ...props
}, ref) => {
  
  const baseStyles = 'w-full bg-input-bg border border-zinc-700 p-2 rounded outline-none transition-colors text-xs font-mono disabled:opacity-50 disabled:cursor-not-allowed';
  
  const variantStyles = {
    default: 'text-foreground focus:border-emerald-500',
    fuchsia: 'text-fuchsia-600 dark:text-fuchsia-400 focus:border-emerald-500',
    sky: 'text-sky-text focus:border-emerald-500',
    amber: 'text-amber-600 dark:text-amber-400 focus:border-emerald-500',
    emerald: 'text-emerald-text focus:border-emerald-500',
  };

  const combinedClassName = `${baseStyles} ${variantStyles[variant]} ${className}`;

  return (
    <input ref={ref} className={combinedClassName} {...props} />
  );
});

Input.displayName = 'Input';
