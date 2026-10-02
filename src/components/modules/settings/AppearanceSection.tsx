import React from 'react';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { SegmentedControl } from '../../common/ui';
import { SettingsGroup, SettingsRow, SettingsSection } from './SettingsSection';

const THEME_OPTIONS = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
] as const;

const FONT_SIZE_OPTIONS = [
  { value: 'sm', label: 'Small' },
  { value: 'md', label: 'Medium' },
  { value: 'lg', label: 'Large' },
] as const;

export const AppearanceSection: React.FC = () => {
  const { theme, setTheme, fontSize, setFontSize } = useSettingsStore();

  return (
    <SettingsSection title="Appearance" description="How the app looks on this machine.">
      <SettingsGroup>
        <SettingsRow label="Theme" description="Also switchable from the sun/moon button in the top bar.">
          <SegmentedControl sizeVariant="sm" value={theme} onChange={setTheme} options={THEME_OPTIONS} />
        </SettingsRow>
        <SettingsRow label="Font size" description="Scales text across every module.">
          <SegmentedControl sizeVariant="sm" value={fontSize} onChange={setFontSize} options={FONT_SIZE_OPTIONS} />
        </SettingsRow>
      </SettingsGroup>
    </SettingsSection>
  );
};
