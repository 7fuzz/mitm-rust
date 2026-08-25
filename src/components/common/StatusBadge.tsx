import React from 'react';

interface StatusBadgeProps {
  code: number;
  className?: string;
}

export const StatusBadge: React.FC<StatusBadgeProps> = ({ code, className = '' }) => {
  let colorStyle = 'bg-neutral-subtle text-neutral-foreground border-border';
  if (code >= 200 && code < 300) {
    colorStyle = 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20';
  } else if (code >= 300 && code < 400) {
    colorStyle = 'bg-sky-500/10 text-sky-600 dark:text-sky-400 border-sky-500/20';
  } else if (code >= 400 && code < 500) {
    colorStyle = 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20';
  } else if (code >= 500) {
    colorStyle = 'bg-rose-500/10 text-rose-600 dark:text-rose-400 border-rose-500/20';
  }

  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 rounded text-[11px] font-mono font-medium border ${colorStyle} ${className}`}
    >
      {code}
    </span>
  );
};
