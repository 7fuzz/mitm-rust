import React from 'react';

interface MethodBadgeProps {
  method: string;
  className?: string;
}

export const MethodBadge: React.FC<MethodBadgeProps> = ({ method, className = '' }) => {
  const m = method.toUpperCase();
  let badgeColor = 'bg-slate-500/10 text-slate-600 dark:text-slate-400 border-slate-500/20';

  switch (m) {
    case 'GET':
      badgeColor = 'bg-sky-500/15 text-sky-700 dark:text-sky-400 border-sky-500/30';
      break;
    case 'POST':
      badgeColor = 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-400 border-emerald-500/30';
      break;
    case 'PUT':
      badgeColor = 'bg-amber-500/15 text-amber-700 dark:text-amber-400 border-amber-500/30';
      break;
    case 'DELETE':
      badgeColor = 'bg-rose-500/15 text-rose-700 dark:text-rose-400 border-rose-500/30';
      break;
    case 'PATCH':
      badgeColor = 'bg-purple-500/15 text-purple-700 dark:text-purple-400 border-purple-500/30';
      break;
    case 'OPTIONS':
    case 'HEAD':
      badgeColor = 'bg-indigo-500/15 text-indigo-700 dark:text-indigo-400 border-indigo-500/30';
      break;
    case 'WS':
    case 'WEBSOCKET':
      badgeColor = 'bg-cyan-500/15 text-cyan-700 dark:text-cyan-400 border-cyan-500/30';
      break;
  }

  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 rounded text-3xs font-mono font-semibold border ${badgeColor} ${className}`}
    >
      {m}
    </span>
  );
};
