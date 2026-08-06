import React, { forwardRef, useRef, KeyboardEvent } from 'react';

type TextareaVariant = 'default' | 'emerald';

interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  variant?: TextareaVariant;
  className?: string;
  enableUndo?: boolean;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(({
  variant = 'default',
  className = '',
  enableUndo = true,
  onKeyDown,
  onChange,
  value,
  ...props
}, ref) => {
  const historyRef = useRef<string[]>(typeof value === 'string' ? [value] : []);
  const pointerRef = useRef<number>(historyRef.current.length ? 0 : -1);
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const handleKeyDownInternal = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    onKeyDown?.(e);

    if (!enableUndo || e.defaultPrevented) return;

    const isMac = typeof navigator !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const isCmdOrCtrl = isMac ? e.metaKey : e.ctrlKey;

    if (!isCmdOrCtrl) return;

    const key = e.key.toLowerCase();
    if (key === 'z') {
      const history = historyRef.current;
      if (e.shiftKey) {
        // Redo (Ctrl + Shift + Z)
        if (pointerRef.current < history.length - 1) {
          e.preventDefault();
          pointerRef.current += 1;
          const nextVal = history[pointerRef.current];
          if (e.currentTarget) {
            const nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
            nativeSetter?.call(e.currentTarget, nextVal);
            e.currentTarget.dispatchEvent(new Event('input', { bubbles: true }));
          }
        }
      } else {
        // Undo (Ctrl + Z)
        if (pointerRef.current > 0) {
          e.preventDefault();
          pointerRef.current -= 1;
          const prevVal = history[pointerRef.current];
          if (e.currentTarget) {
            const nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
            nativeSetter?.call(e.currentTarget, prevVal);
            e.currentTarget.dispatchEvent(new Event('input', { bubbles: true }));
          }
        }
      }
    } else if (key === 'y' && !isMac) {
      // Redo (Ctrl + Y)
      const history = historyRef.current;
      if (pointerRef.current < history.length - 1) {
        e.preventDefault();
        pointerRef.current += 1;
        const nextVal = history[pointerRef.current];
        if (e.currentTarget) {
          const nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
          nativeSetter?.call(e.currentTarget, nextVal);
          e.currentTarget.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }
    }
  };

  const handleChangeInternal = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    onChange?.(e);

    if (enableUndo) {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        const history = historyRef.current;
        const pointer = pointerRef.current;
        if (history[pointer] !== val) {
          const newHist = history.slice(0, pointer + 1);
          newHist.push(val);
          if (newHist.length > 100) newHist.shift();
          historyRef.current = newHist;
          pointerRef.current = newHist.length - 1;
        }
      }, 200);
    }
  };

  const baseStyles = 'w-full bg-input-bg border border-zinc-700 p-2 rounded outline-none transition-colors text-xs font-mono disabled:opacity-50 disabled:cursor-not-allowed resize-y';
  
  const variantStyles = {
    default: 'text-foreground focus:border-emerald-500',
    emerald: 'text-emerald-text focus:border-emerald-500',
  };

  const combinedClassName = `${baseStyles} ${variantStyles[variant]} ${className}`;

  return (
    <textarea
      ref={ref}
      value={value}
      className={combinedClassName}
      onKeyDown={handleKeyDownInternal}
      onChange={handleChangeInternal}
      {...props}
    />
  );
});

Textarea.displayName = 'Textarea';
