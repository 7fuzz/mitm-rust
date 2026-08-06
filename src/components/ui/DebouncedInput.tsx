import React, { useState, useEffect, useRef, KeyboardEvent } from 'react';

interface DebouncedInputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'onChange'> {
  value: string;
  onChange: (value: string) => void;
  debounce?: number;
  showIcon?: boolean;
  onTypingChange?: (isTyping: boolean) => void;
  inputClassName?: string;
  enableUndo?: boolean;
}

export const DebouncedInput = React.forwardRef<HTMLInputElement, DebouncedInputProps>(({
  value: initialValue,
  onChange,
  debounce = 400,
  showIcon = true,
  onTypingChange,
  className,
  inputClassName = '',
  enableUndo = true,
  onKeyDown,
  ...props
}, ref) => {
  const [value, setValue] = useState(initialValue);
  const [isTyping, setIsTyping] = useState(false);
  const isFirstRender = useRef(true);

  const historyRef = useRef<string[]>([initialValue || '']);
  const pointerRef = useRef<number>(0);
  const undoDebounceRef = useRef<NodeJS.Timeout | null>(null);

  // Sync with external value changes
  useEffect(() => {
    setValue(initialValue);
  }, [initialValue]);

  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      return;
    }

    if (value !== initialValue) {
      setIsTyping(true);
      onTypingChange?.(true);
    }

    const timeout = setTimeout(() => {
      if (value !== initialValue) {
        onChange(value);
        setIsTyping(false);
        onTypingChange?.(false);
      }
    }, debounce);

    return () => clearTimeout(timeout);
  }, [value, debounce, onChange, onTypingChange, initialValue]);

  const handleKeyDownInternal = (e: KeyboardEvent<HTMLInputElement>) => {
    onKeyDown?.(e);

    if (!enableUndo || e.defaultPrevented) return;

    const isMac = typeof navigator !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const isCmdOrCtrl = isMac ? e.metaKey : e.ctrlKey;

    if (!isCmdOrCtrl) return;

    const key = e.key.toLowerCase();
    if (key === 'z') {
      const history = historyRef.current;
      if (e.shiftKey) {
        // Redo
        if (pointerRef.current < history.length - 1) {
          e.preventDefault();
          pointerRef.current += 1;
          const nextVal = history[pointerRef.current];
          setValue(nextVal);
          onChange(nextVal);
        }
      } else {
        // Undo
        if (pointerRef.current > 0) {
          e.preventDefault();
          pointerRef.current -= 1;
          const prevVal = history[pointerRef.current];
          setValue(prevVal);
          onChange(prevVal);
        }
      }
    } else if (key === 'y' && !isMac) {
      // Redo
      const history = historyRef.current;
      if (pointerRef.current < history.length - 1) {
        e.preventDefault();
        pointerRef.current += 1;
        const nextVal = history[pointerRef.current];
        setValue(nextVal);
        onChange(nextVal);
      }
    }
  };

  const handleChangeInternal = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setValue(val);

    if (enableUndo) {
      if (undoDebounceRef.current) clearTimeout(undoDebounceRef.current);
      undoDebounceRef.current = setTimeout(() => {
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

  const handleClear = () => {
    setValue("");
    onChange("");
    setIsTyping(false);
    onTypingChange?.(false);
  };

  return (
    <div className={`relative flex items-center bg-input-bg border border-zinc-800 rounded px-2 focus-within:border-emerald-500 transition-colors shrink-0 ${className}`}>
      {showIcon && (
        <svg 
          width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" 
          className={`transition-colors shrink-0 ${isTyping ? 'text-amber-500 animate-pulse' : 'text-zinc-500'}`}
        >
          <circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
      )}
      <input
        {...props}
        ref={ref}
        value={value}
        onKeyDown={handleKeyDownInternal}
        onChange={handleChangeInternal}
        className={`w-full bg-transparent outline-none text-[10px] font-mono text-foreground px-2 py-1 placeholder:text-zinc-600 ${inputClassName}`}
      />
      {value && (
        <button 
          onClick={handleClear} 
          className="text-zinc-500 hover:text-rose-400 ml-1 flex items-center justify-center shrink-0"
        >
          ✕
        </button>
      )}
    </div>
  );
});

DebouncedInput.displayName = 'DebouncedInput';
