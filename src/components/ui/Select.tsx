import { useState, useRef, useEffect } from 'react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { ChevronDown } from 'lucide-react';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

interface SelectOption {
  value?: string;
  label?: string;
  isHeader?: boolean;
  isDivider?: boolean;
  color?: string;
}

interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
  className?: string;
  placeholder?: string;
  disabled?: boolean;
}

export function Select({ value, onChange, options, className, placeholder, disabled }: SelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find(opt => !opt.isHeader && !opt.isDivider && opt.value === value);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div className={cn("relative inline-block w-full", className, disabled && "opacity-50 pointer-events-none")} ref={containerRef}>
      <button
        type="button"
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled}
        className={cn(
          "flex items-center justify-between w-full px-3 py-1.5",
          "bg-zinc-950 border border-zinc-800 rounded",
          "text-zinc-300 text-[10px] uppercase font-bold tracking-widest",
          "transition-all outline-none focus:border-emerald-500/50 hover:border-zinc-700",
          isOpen && "border-emerald-500/50 ring-1 ring-emerald-500/10",
          disabled && "cursor-not-allowed border-zinc-900"
        )}
      >
        <span className={cn("truncate text-left flex-1", selectedOption?.color)}>
          {selectedOption ? selectedOption.label : placeholder || 'Select...'}
        </span>
        <ChevronDown className={cn("ml-2 h-3 w-3 text-zinc-600 transition-transform shrink-0", isOpen && "rotate-180")} />
      </button>

      {isOpen && (
        <div className="absolute z-50 w-full mt-1 bg-zinc-950 border border-zinc-800 rounded shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-100 min-w-[200px]">
          <div className="max-h-80 overflow-y-auto scrollbar-thin scrollbar-thumb-zinc-800 p-1">
            {options.map((option, idx) => {
              if (option.isDivider) {
                return <div key={`divider-${idx}`} className="h-px bg-zinc-800 my-1 mx-2" />;
              }
              
              if (option.isHeader) {
                return (
                  <div key={`header-${idx}`} className="px-3 py-1.5 text-[8px] font-black uppercase tracking-[0.2em] text-zinc-600">
                    {option.label}
                  </div>
                );
              }

              return (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => {
                    if (option.value !== undefined) {
                      onChange(option.value);
                      setIsOpen(false);
                    }
                  }}
                  className={cn(
                    "flex items-center w-full px-3 py-2 text-[10px] uppercase font-bold tracking-widest transition-all rounded text-left",
                    option.value === value 
                      ? "bg-emerald-500/10 text-emerald-400" 
                      : cn("text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200", option.color)
                  )}
                >
                  <span className="truncate">{option.label}</span>
                  {option.value === value && (
                    <div className="ml-auto w-1 h-1 rounded-full bg-emerald-500" />
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
