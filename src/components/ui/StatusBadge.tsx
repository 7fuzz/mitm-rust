export function StatusBadge({ code }: { code: number }) {
  const isError = code >= 400;
  return (
    <span className={`px-2 py-0.5 rounded text-[10px] font-bold border ${isError
        ? 'bg-rose-500/10 border-rose-500/20 text-rose-500'
        : 'bg-emerald-500/10 border-emerald-500/20 text-emerald-text'
      }`}>
      {code}
    </span>
  );
}
