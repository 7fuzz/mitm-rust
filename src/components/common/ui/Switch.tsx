import React from 'react';

export interface SwitchProps {
  checked: boolean;
  onChange: () => void;
  title?: string;
}

export const Switch: React.FC<SwitchProps> = ({ checked, onChange, title }) => (
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    onClick={onChange}
    title={title}
    className={`relative inline-flex h-4 w-7 shrink-0 items-center rounded-full transition-colors cursor-pointer ${
      checked ? 'bg-primary' : 'bg-neutral-subtle border border-border'
    }`}
  >
    <span
      className={`inline-block h-3 w-3 rounded-full bg-white shadow transition-transform ${
        checked ? 'translate-x-3.5' : 'translate-x-0.5'
      }`}
    />
  </button>
);
