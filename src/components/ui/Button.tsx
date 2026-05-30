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
    primary: 'bg-btn-primary hover:opacity-90 text-btn-primary-text shadow-lg shadow-emerald-500/20',
    secondary: 'bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 text-zinc-300 hover:text-zinc-100',
    destructive: 'bg-btn-destructive hover:opacity-90 text-btn-destructive-text shadow-lg shadow-rose-500/20',
    purple: 'bg-btn-purple hover:opacity-90 text-btn-purple-text shadow-lg shadow-purple-500/20',
    sky: 'bg-btn-sky hover:opacity-90 text-btn-sky-text shadow-lg shadow-sky-500/20',
    amber: 'bg-btn-amber hover:opacity-90 text-btn-amber-text shadow-lg shadow-amber-500/20',
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
