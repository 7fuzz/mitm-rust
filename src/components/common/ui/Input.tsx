import React from 'react';
import { MingCuteIcon, MingCuteIconName } from '../MingCuteIcon';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  leftIcon?: MingCuteIconName;
  rightIcon?: MingCuteIconName;
  onRightIconClick?: () => void;
  sizeVariant?: 'xs' | 'sm' | 'md';
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ leftIcon, rightIcon, onRightIconClick, sizeVariant = 'xs', className = '', ...props }, ref) => {
    const sizeClasses =
      sizeVariant === 'xs'
        ? 'py-1 text-xs'
        : sizeVariant === 'sm'
        ? 'py-1.5 text-xs'
        : 'py-2 text-sm';

    const leftPadding = leftIcon ? 'pl-7' : 'pl-2.5';
    const rightPadding = rightIcon ? 'pr-7' : 'pr-2.5';

    return (
      <div className="relative inline-flex items-center w-full">
        {leftIcon && (
          <MingCuteIcon
            name={leftIcon}
            size={13}
            className="absolute left-2.5 text-muted-foreground pointer-events-none"
          />
        )}
        <input
          ref={ref}
          className={`w-full bg-white dark:bg-zinc-900 text-slate-900 dark:text-zinc-100 border border-slate-300 dark:border-zinc-700 rounded font-mono placeholder:text-muted-foreground focus:outline-none focus:border-primary transition-colors disabled:opacity-50 ${sizeClasses} ${leftPadding} ${rightPadding} ${className}`}
          {...props}
        />
        {rightIcon && (
          <button
            type="button"
            onClick={onRightIconClick}
            className={`absolute right-2 text-muted-foreground hover:text-foreground p-0.5 rounded ${
              onRightIconClick ? 'cursor-pointer' : 'pointer-events-none'
            }`}
          >
            <MingCuteIcon name={rightIcon} size={13} />
          </button>
        )}
      </div>
    );
  }
);
Input.displayName = 'Input';
