import React from 'react';

interface FuzzerSectionProps {
  title: string;
  right?: React.ReactNode;
  children: React.ReactNode;
  bodyClassName?: string;
}

export const FuzzerSection: React.FC<FuzzerSectionProps> = ({ title, right, children, bodyClassName = 'p-2.5 space-y-2' }) => (
  <section className="border border-border rounded-lg bg-surface overflow-hidden">
    <div className="h-8 px-2.5 bg-header border-b border-border flex items-center gap-2 select-none">
      <span className="flex-1 text-3xs font-semibold uppercase tracking-wider text-muted-foreground">{title}</span>
      {right}
    </div>
    <div className={bodyClassName}>{children}</div>
  </section>
);

export const fieldInputClass =
  'w-full bg-background border border-border rounded px-2 py-1 font-mono text-2xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary';
