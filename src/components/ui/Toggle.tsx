import { ChangeEvent } from 'react';

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  subLabel?: string;
  variant?: 'sky' | 'emerald' | 'amber' | 'purple' | 'rose' | 'zinc';
  size?: 'sm' | 'md';
  className?: string;
}

export function Toggle({ 
  checked, 
  onChange, 
  label, 
  subLabel, 
  variant = 'emerald', 
  size = 'md',
  className = ""
}: ToggleProps) {
  
  const variants = {
    sky: {
      thumb: 'bg-sky-500 shadow-[0_0_10px_rgba(14,165,233,0.6)]',
      track: 'bg-sky-500/20 border-sky-500/30'
    },
    emerald: {
      thumb: 'bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.6)]',
      track: 'bg-emerald-500/20 border-emerald-500/30'
    },
    amber: {
      thumb: 'bg-amber-500 shadow-[0_0_10px_rgba(245,158,11,0.6)]',
      track: 'bg-amber-500/20 border-amber-500/30'
    },
    purple: {
      thumb: 'bg-purple-500 shadow-[0_0_10px_rgba(168,85,247,0.6)]',
      track: 'bg-purple-500/20 border-purple-500/30'
    },
    rose: {
      thumb: 'bg-rose-500 shadow-[0_0_10px_rgba(244,63,94,0.6)]',
      track: 'bg-rose-500/20 border-rose-500/30'
    },
    zinc: {
      thumb: 'bg-zinc-300 shadow-none',
      track: 'bg-zinc-600 border-zinc-500'
    }
  };

  const activeVariant = variants[variant];
  const s = size === 'sm' ? { h: 'h-4', w: 'w-8', t: 'h-3 w-3', x: 'translate-x-4' } : { h: 'h-5', w: 'w-10', t: 'h-4 w-4', x: 'translate-x-5' };

  return (
    <label className={`flex items-center justify-between cursor-pointer group ${className}`}>
      {(label || subLabel) && (
        <div className="flex flex-col mr-3">
          {label && <span className="text-[10px] text-zinc-300 font-black uppercase tracking-widest group-hover:text-zinc-100 transition-colors">{label}</span>}
          {subLabel && <span className="text-[8px] text-zinc-500 font-mono">{subLabel}</span>}
        </div>
      )}
      <div className={`relative inline-flex ${s.h} ${s.w} shrink-0 cursor-pointer rounded-full border transition-all duration-200 ease-in-out focus:outline-none ${checked ? activeVariant.track : 'bg-zinc-800 border-zinc-700'}`}>
        <input
          type="checkbox"
          checked={checked}
          onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.checked)}
          className="sr-only peer"
        />
        <div className={`pointer-events-none inline-block ${s.t} transform rounded-full shadow-sm ring-0 transition duration-200 ease-in-out absolute top-0.5 left-0.5 ${checked ? `${s.x} ${activeVariant.thumb}` : 'translate-x-0 bg-zinc-500'}`}></div>
      </div>
    </label>
  );
}
