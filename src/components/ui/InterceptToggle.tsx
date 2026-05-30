export function InterceptToggle({ active, onToggle }: { active: boolean; onToggle: () => void }) {
  return (
    <button
      onClick={onToggle}
      className={`flex items-center gap-2 px-4 py-2 rounded font-bold text-[10px] uppercase tracking-widest transition-all border ${active
          ? 'bg-rose-500/20 border-rose-500 text-rose-500 shadow-[0_0_15px_rgba(244,63,94,0.3)]'
          : 'bg-zinc-900 border-zinc-700 text-zinc-500'
        }`}
    >
      <span className={`h-2 w-2 rounded-full ${active ? 'bg-rose-500 animate-pulse' : 'bg-zinc-700'}`} />
      {active ? 'Intercept_On' : 'Intercept_Off'}
    </button>
  );
}
