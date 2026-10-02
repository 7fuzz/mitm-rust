import React from 'react';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { SegmentedControl } from '../../common/ui';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import { ZOOM_LEVELS } from '../../../stores/uiPrefs/registry';
import { SettingsGroup, SettingsRow, SettingsSection } from './SettingsSection';

const THEME_OPTIONS = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
] as const;

const ZOOM_OPTIONS = ZOOM_LEVELS.map((z) => ({ value: String(z), label: `${Math.round(z * 100)}%` }));

export const AppearanceSection: React.FC = () => {
  const { theme, setTheme } = useSettingsStore();
  const [zoom, setZoom] = useUiPref('appearance.zoom');

  return (
    <SettingsSection title="Appearance" description="How the app looks on this machine.">
      <SettingsGroup>
        <SettingsRow label="Theme" description="Also switchable from the sun/moon button in the top bar.">
          <SegmentedControl sizeVariant="sm" value={theme} onChange={setTheme} options={THEME_OPTIONS} />
        </SettingsRow>
        <SettingsRow
          label="Zoom"
          description="Scales text and layout across every module. Ctrl+= and Ctrl+- step it, Ctrl+0 resets to 100%."
        >
          <SegmentedControl
            sizeVariant="sm"
            value={String(zoom)}
            onChange={(v) => setZoom(Number(v))}
            options={ZOOM_OPTIONS}
          />
        </SettingsRow>
      </SettingsGroup>
    </SettingsSection>
  );
};
