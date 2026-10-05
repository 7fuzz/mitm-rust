import React from 'react';
import { Select, type SelectOption } from './ui';

interface PreRequestPickerProps {
  value?: string | null;
  options: SelectOption[];
  onChange: (id: string | null) => void;
}

export const PreRequestPicker: React.FC<PreRequestPickerProps> = ({ value, options, onChange }) => (
  <div className="flex items-center gap-2 text-xs">
    <span className="text-muted-foreground shrink-0">Run first</span>
    <Select
      value={value || ''}
      onChange={(e) => onChange(e.target.value || null)}
      options={[{ value: '', label: 'None' }, ...options]}
      sizeVariant="sm"
      className="min-w-72 max-w-xl"
    />
  </div>
);
