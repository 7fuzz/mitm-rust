import React from 'react';

/** Sentence-case buttons used across the settings pages */
export const settingsButtonClass = {
  primary:
    'inline-flex items-center gap-1.5 px-3 py-1.5 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover transition-colors shadow-xs disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer shrink-0',
  secondary:
    'inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-background border border-border text-foreground font-medium hover:bg-neutral-subtle transition-colors disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer shrink-0',
  /** Armed state of a two-click destructive action */
  dangerSolid:
    'inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-rose-600 border border-rose-600 text-white font-semibold hover:bg-rose-500 transition-colors shadow-xs disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer shrink-0',
  danger:
    'inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-rose-500/10 border border-rose-500/30 text-rose-500 font-semibold hover:bg-rose-500/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer shrink-0',
};

interface SettingsSectionProps {
  title: string;
  description?: React.ReactNode;
  /** Right side of the title row, e.g. a status badge */
  aside?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

/** Page for one settings section: title row, short description, then its groups. */
export const SettingsSection: React.FC<SettingsSectionProps> = ({ title, description, aside, children, className = '' }) => (
  <section className={`space-y-4 ${className}`}>
    <header className="space-y-1">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-base font-semibold text-foreground">{title}</h2>
        {aside}
      </div>
      {description && <p className="text-muted-foreground text-xs leading-relaxed max-w-2xl">{description}</p>}
    </header>
    {children}
  </section>
);

interface SettingsGroupProps {
  /** Small uppercase label above the group */
  label?: string;
  tone?: 'default' | 'danger';
  children: React.ReactNode;
  className?: string;
}

/** Bordered block of rows or free content within a section */
export const SettingsGroup: React.FC<SettingsGroupProps> = ({ label, tone = 'default', children, className = '' }) => (
  <div className="space-y-1.5">
    {label && (
      <div
        className={`text-3xs font-semibold uppercase tracking-wider ${
          tone === 'danger' ? 'text-rose-500' : 'text-muted-foreground'
        }`}
      >
        {label}
      </div>
    )}
    <div
      className={`rounded-lg border bg-surface divide-y ${
        tone === 'danger' ? 'border-rose-500/30 divide-rose-500/20' : 'border-border divide-border'
      } ${className}`}
    >
      {children}
    </div>
  </div>
);

interface SettingsRowProps {
  label: React.ReactNode;
  description?: React.ReactNode;
  /** Control aligned right */
  children?: React.ReactNode;
}

/** One setting: label and help text on the left, its control on the right */
export const SettingsRow: React.FC<SettingsRowProps> = ({ label, description, children }) => (
  <div className="px-3 py-2.5 flex items-center justify-between gap-4">
    <div className="min-w-0 space-y-0.5">
      <div className="text-foreground font-medium">{label}</div>
      {description && <div className="text-muted-foreground text-2xs leading-relaxed">{description}</div>}
    </div>
    {children && <div className="shrink-0 flex items-center gap-2">{children}</div>}
  </div>
);
