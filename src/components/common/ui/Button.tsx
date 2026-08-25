import React from 'react';
import { MingCuteIcon, MingCuteIconName } from '../MingCuteIcon';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'success' | 'ghost' | 'outline' | 'subtle';
  sizeVariant?: 'xs' | 'sm' | 'md';
  icon?: MingCuteIconName;
  iconPosition?: 'left' | 'right';
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'secondary',
  sizeVariant = 'xs',
  icon,
  iconPosition = 'left',
  className = '',
  disabled,
  ...props
}) => {
  const variantClasses = {
    primary: 'bg-primary hover:bg-primary-hover text-primary-foreground font-semibold shadow-xs border border-transparent',
    secondary: 'bg-surface hover:bg-neutral-subtle text-foreground border border-border font-medium shadow-2xs',
    danger: 'bg-rose-600 hover:bg-rose-500 text-white font-semibold shadow-xs border border-transparent',
    success: 'bg-emerald-600 hover:bg-emerald-500 text-white font-semibold shadow-xs border border-transparent',
    ghost: 'bg-transparent hover:bg-neutral-subtle text-muted-foreground hover:text-foreground font-medium',
    outline: 'bg-transparent hover:bg-neutral-subtle text-foreground border border-border font-medium',
    subtle: 'bg-neutral-subtle hover:bg-surface text-foreground border border-border font-medium',
  }[variant];

  const sizeClasses = {
    xs: 'px-2 py-1 text-xs gap-1.5 rounded',
    sm: 'px-2.5 py-1.5 text-xs gap-1.5 rounded-md',
    md: 'px-3 py-2 text-sm gap-2 rounded-md',
  }[sizeVariant];

  const iconSize = sizeVariant === 'xs' ? 13 : sizeVariant === 'sm' ? 14 : 16;

  return (
    <button
      disabled={disabled}
      className={`inline-flex items-center justify-center font-sans transition-colors cursor-pointer select-none disabled:opacity-40 disabled:cursor-not-allowed ${variantClasses} ${sizeClasses} ${className}`}
      {...props}
    >
      {icon && iconPosition === 'left' && <MingCuteIcon name={icon} size={iconSize} />}
      {children && <span>{children}</span>}
      {icon && iconPosition === 'right' && <MingCuteIcon name={icon} size={iconSize} />}
    </button>
  );
};
