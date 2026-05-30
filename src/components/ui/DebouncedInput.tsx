import React, { useState, useEffect, useRef } from 'react';

interface DebouncedInputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'onChange'> {
  value: string;
  onChange: (value: string) => void;
  debounce?: number;
  showIcon?: boolean;
  onTypingChange?: (isTyping: boolean) => void;
  inputClassName?: string;
}

export const DebouncedInput = React.forwardRef<HTMLInputElement, DebouncedInputProps>(({
  value: initialValue,
  onChange,
  debounce = 400,
  showIcon = true,
  onTypingChange,
  className,
  inputClassName = '',
  ...props
}, ref) => {
  const [value, setValue] = useState(initialValue);
  const [isTyping, setIsTyping] = useState(false);
  const isFirstRender = useRef(true);

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
        onChange={e => setValue(e.target.value)}
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
