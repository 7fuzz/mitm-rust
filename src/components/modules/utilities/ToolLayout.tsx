import React, { useCallback, useState } from 'react';
import { MingCuteIcon, MingCuteIconName } from '../../common/MingCuteIcon';

export interface AlgorithmGroup<T extends string> {
  label: string;
  items: { id: T; name: string; hint?: string }[];
}

interface AlgorithmListProps<T extends string> {
  groups: AlgorithmGroup<T>[];
  value: T;
  onChange: (id: T) => void;
}

export function AlgorithmList<T extends string>({ groups, value, onChange }: AlgorithmListProps<T>) {
  return (
    <nav className="w-44 shrink-0 bg-surface border border-border rounded-lg overflow-y-auto py-1.5 select-none">
      {groups.map((group) => (
        <div key={group.label} className="mb-1.5 last:mb-0">
          <div className="px-3 pt-1 pb-0.5 text-3xs font-semibold uppercase tracking-wider text-muted-foreground">
            {group.label}
          </div>
          {group.items.map((item) => {
            const active = item.id === value;
            return (
              <button
                key={item.id}
                onClick={() => onChange(item.id)}
                className={`w-full flex items-center justify-between gap-2 px-3 py-1.5 text-left text-xs transition-colors cursor-pointer border-l-2 ${
                  active
                    ? 'border-primary bg-neutral-subtle text-foreground font-semibold'
                    : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
                }`}
              >
                <span className="truncate">{item.name}</span>
                {item.hint && <span className="text-3xs font-mono text-muted-foreground shrink-0">{item.hint}</span>}
              </button>
            );
          })}
        </div>
      ))}
    </nav>
  );
}

interface ToolPaneProps {
  title: string;
  status?: React.ReactNode;
  actions?: React.ReactNode;
  footer?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

export const ToolPane: React.FC<ToolPaneProps> = ({ title, status, actions, footer, children, className = '' }) => (
  <section className={`flex flex-col min-h-0 bg-surface border border-border rounded-lg overflow-hidden ${className}`}>
    <header className="h-8 px-2.5 bg-header border-b border-border flex items-center gap-2 shrink-0 select-none">
      <span className="text-3xs font-semibold uppercase tracking-wider text-muted-foreground">{title}</span>
      {status}
      <div className="ml-auto flex items-center gap-0.5">{actions}</div>
    </header>
    <div className="flex-1 min-h-0 bg-background flex flex-col">{children}</div>
    {footer && (
      <footer className="h-6 px-2.5 border-t border-border bg-surface flex items-center gap-2 shrink-0 text-3xs font-mono text-muted-foreground select-none">
        {footer}
      </footer>
    )}
  </section>
);

interface PaneActionProps {
  icon: MingCuteIconName;
  label?: string;
  title: string;
  onClick: () => void;
  disabled?: boolean;
  active?: boolean;
}

export const PaneAction: React.FC<PaneActionProps> = ({ icon, label, title, onClick, disabled, active }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    title={title}
    className={`h-6 px-1.5 inline-flex items-center gap-1 rounded text-2xs font-medium transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${
      active ? 'text-emerald-500' : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
    }`}
  >
    <MingCuteIcon name={active ? 'check_line' : icon} size={13} />
    {label && <span>{label}</span>}
  </button>
);

export const StatusPill: React.FC<{ tone: 'ok' | 'error' | 'warn'; children: React.ReactNode }> = ({ tone, children }) => {
  const toneClass =
    tone === 'ok'
      ? 'bg-emerald-500/15 text-emerald-500'
      : tone === 'error'
      ? 'bg-rose-500/15 text-rose-500'
      : 'bg-amber-500/15 text-amber-500';
  return <span className={`px-1.5 py-px rounded text-3xs font-semibold ${toneClass}`}>{children}</span>;
};

export const ErrorStrip: React.FC<{ message: string }> = ({ message }) => (
  <div className="px-2.5 py-1.5 border-t border-rose-500/30 bg-rose-500/10 text-rose-500 text-2xs font-mono flex items-start gap-1.5 shrink-0">
    <MingCuteIcon name="warning_line" size={13} className="mt-px shrink-0" />
    <span className="break-all">{message}</span>
  </div>
);

export const editorClass =
  'flex-1 min-h-0 w-full bg-transparent text-foreground font-mono text-xs resize-none focus:outline-none p-2.5 leading-relaxed selection:bg-primary/20 placeholder:text-muted-foreground/60';

export const textStats = (str: string) => {
  const bytes = new TextEncoder().encode(str).length;
  return `${str.length} chars · ${bytes} B`;
};

export function useCopyFeedback() {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const copy = useCallback((text: string, key: string) => {
    if (!text) return;
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 1500);
  }, []);
  return { copiedKey, copy };
}

export const readClipboard = async (): Promise<string | null> => {
  try {
    return (await navigator.clipboard.readText()) || null;
  } catch {
    return null;
  }
};
