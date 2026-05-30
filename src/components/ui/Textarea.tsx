import React, { forwardRef } from 'react';

type TextareaVariant = 'default' | 'emerald';

interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  variant?: TextareaVariant;
  className?: string;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(({
  variant = 'default',
  className = '',
  ...props
}, ref) => {
  
  const baseStyles = 'w-full bg-input-bg border border-zinc-700 p-2 rounded outline-none transition-colors text-xs font-mono disabled:opacity-50 disabled:cursor-not-allowed resize-y';
  
  const variantStyles = {
    default: 'text-foreground focus:border-emerald-500',
    emerald: 'text-emerald-text focus:border-emerald-500',
  };

  const combinedClassName = `${baseStyles} ${variantStyles[variant]} ${className}`;

  return (
    <textarea ref={ref} className={combinedClassName} {...props} />
  );
});

Textarea.displayName = 'Textarea';
