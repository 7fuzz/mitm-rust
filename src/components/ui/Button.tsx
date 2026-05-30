import React, { forwardRef } from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'destructive' | 'purple' | 'sky' | 'ghost' | 'outline' | 'amber';
type ButtonSize = 'xs' | 'sm' | 'md' | 'lg';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  className?: string;
  children: React.ReactNode;
}

const ButtonComponent = forwardRef<HTMLButtonElement, ButtonProps>(({
  variant = 'secondary',
  size = 'md',
  className = '',
  children,
  ...props
}, ref) => {
  
  const baseStyles = 'inline-flex items-center justify-center font-bold uppercase tracking-widest transition-all focus:outline-none disabled:opacity-40 disabled:pointer-events-none rounded';
  
  const sizeStyles = {
    xs: 'px-2 py-1 text-[9px]',
    sm: 'px-3 py-1.5 text-[10px]',
    md: 'px-4 py-2 text-[11px]',
    lg: 'px-6 py-3 text-[12px]',
  };
  
  const variantStyles = {
    primary: 'bg-emerald-600 hover:bg-emerald-500 text-zinc-950 shadow-lg shadow-emerald-500/20',
    secondary: 'bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 text-zinc-300 hover:text-zinc-100',
    destructive: 'bg-rose-600 hover:bg-rose-500 text-zinc-950 shadow-lg shadow-rose-500/20',
    purple: 'bg-purple-600 hover:bg-purple-500 text-zinc-950 shadow-lg shadow-purple-500/20',
    sky: 'bg-sky-600 hover:bg-sky-500 text-zinc-950 shadow-lg shadow-sky-500/20',
    amber: 'bg-amber-600 hover:bg-amber-500 text-zinc-950 shadow-lg shadow-amber-500/20',
    ghost: 'bg-transparent hover:bg-zinc-900/50 text-zinc-500 hover:text-zinc-300',
    outline: 'bg-transparent border border-zinc-700 hover:border-zinc-500 text-zinc-400 hover:text-zinc-200',
  };

  const combinedClassName = `${baseStyles} ${sizeStyles[size]} ${variantStyles[variant]} ${className}`;

  return (
    <button ref={ref} className={combinedClassName} {...props}>
      {children}
    </button>
  );
});

ButtonComponent.displayName = 'Button';

export { ButtonComponent as Button };
