import React, { forwardRef } from 'react';
import { MingCuteIcon, MingCuteIconName } from '../MingCuteIcon';

export type ButtonVariant =
  | 'primary'
  | 'secondary'
  | 'destructive'
  | 'danger'
  | 'purple'
  | 'sky'
  | 'amber'
  | 'ghost'
  | 'outline'
  | 'subtle'
  | 'success';

export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  sizeVariant?: ButtonSize;
  icon?: MingCuteIconName;
  iconPosition?: 'left' | 'right';
  className?: string;
  children?: React.ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      children,
      variant = 'secondary',
      size = 'sm',
      sizeVariant,
      icon,
      iconPosition = 'left',
      className = '',
      disabled,
      ...props
    },
    ref
  ) => {
    const effectiveSize = sizeVariant || size;

    const baseStyles =
      'inline-flex items-center justify-center font-bold uppercase tracking-wider transition-all focus:outline-none disabled:opacity-40 disabled:pointer-events-none rounded cursor-pointer select-none';

    const sizeStyles: Record<ButtonSize, string> = {
      xs: 'px-2 py-0.5 text-[9px] gap-1',
      sm: 'px-2.5 py-1 text-[10px] gap-1.5',
      md: 'px-3.5 py-1.5 text-[11px] gap-2',
      lg: 'px-5 py-2.5 text-[12px] gap-2',
    };

    const variantStyles: Record<ButtonVariant, string> = {
      primary: 'bg-primary hover:opacity-90 text-primary-foreground shadow-xs border border-transparent',
      secondary: 'bg-surface hover:bg-neutral-subtle text-foreground border border-border font-medium shadow-2xs',
      destructive: 'bg-rose-600 hover:bg-rose-500 text-white shadow-xs border border-transparent',
      danger: 'bg-rose-600 hover:bg-rose-500 text-white shadow-xs border border-transparent',
      purple: 'bg-purple-600 hover:bg-purple-500 text-white shadow-xs border border-transparent',
      sky: 'bg-sky-600 hover:bg-sky-500 text-white shadow-xs border border-transparent',
      amber: 'bg-amber-600 hover:bg-amber-500 text-black font-bold shadow-xs border border-transparent',
      ghost: 'bg-transparent hover:bg-neutral-subtle text-muted-foreground hover:text-foreground font-medium',
      outline: 'bg-transparent hover:bg-neutral-subtle text-foreground border border-border font-medium',
      subtle: 'bg-neutral-subtle hover:bg-surface text-foreground border border-border font-medium',
      success: 'bg-emerald-600 hover:bg-emerald-500 text-white shadow-xs border border-transparent',
    };

    const iconSize =
      effectiveSize === 'xs' ? 11 : effectiveSize === 'sm' ? 13 : effectiveSize === 'md' ? 14 : 16;

    const combinedClassName = `${baseStyles} ${sizeStyles[effectiveSize]} ${variantStyles[variant]} ${className}`;

    return (
      <button ref={ref} disabled={disabled} className={combinedClassName} {...props}>
        {icon && iconPosition === 'left' && <MingCuteIcon name={icon} size={iconSize} />}
        {children && <span>{children}</span>}
        {icon && iconPosition === 'right' && <MingCuteIcon name={icon} size={iconSize} />}
      </button>
    );
  }
);

Button.displayName = 'Button';
