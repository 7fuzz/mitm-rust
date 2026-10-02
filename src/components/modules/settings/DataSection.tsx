import React, { useState } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { backupAndResetDatabase } from '../../../services/tauri/bridge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SettingsGroup, SettingsRow, SettingsSection, settingsButtonClass } from './SettingsSection';

type Action = 'traffic' | 'webhooks' | 'reset';

export const DataSection: React.FC = () => {
  const { clearTraffic } = useProxyStore();
  const { clearDeliveries } = useWebhookStore();

  // Destructive actions take two clicks: the first arms the button, the second runs it
  const [armed, setArmed] = useState<Action | null>(null);
  const [done, setDone] = useState<Action | null>(null);

  const runArmed = async (action: Action, run: () => Promise<void>) => {
    if (armed !== action) {
      setArmed(action);
      return;
    }
    setArmed(null);
    await run();
    setDone(action);
    setTimeout(() => setDone((d) => (d === action ? null : d)), 2500);
  };

  const actionButton = (action: Action, label: string, run: () => Promise<void>, icon = 'delete_2_line') => (
    <button
      onClick={() => runArmed(action, run)}
      onMouseLeave={() => setArmed((a) => (a === action ? null : a))}
      className={armed === action ? settingsButtonClass.dangerSolid : action === 'reset' ? settingsButtonClass.danger : settingsButtonClass.secondary}
    >
      <MingCuteIcon name={done === action ? 'check_line' : icon} size={14} className={done === action ? 'text-emerald-500' : ''} />
      <span>{armed === action ? 'Click again to confirm' : done === action ? 'Cleared' : label}</span>
    </button>
  );

  return (
    <SettingsSection title="Data" description="Captured data is stored in the local SQLite database (mitm.db).">
      <SettingsGroup label="Captured data">
        <SettingsRow label="Traffic logs" description="Every request and response in History.">
          {actionButton('traffic', 'Clear', clearTraffic)}
        </SettingsRow>
        <SettingsRow label="Webhook hits" description="Deliveries received by the Webhooks module.">
          {actionButton('webhooks', 'Clear', clearDeliveries)}
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup label="Danger zone" tone="danger">
        <SettingsRow
          label="Factory reset"
          description="Moves mitm.db aside as a timestamped backup (mitm.db.bak_…), starts a fresh empty database, and reloads. Workspaces, collections, environments, and logs all start over; the root CA is kept."
        >
          {actionButton(
            'reset',
            'Factory reset',
            async () => {
              try {
                await backupAndResetDatabase();
                window.location.reload();
              } catch (err) {
                console.error('Factory reset failed:', err);
                alert(`Factory reset failed: ${err}`);
              }
            },
            'alert_line'
          )}
        </SettingsRow>
      </SettingsGroup>
    </SettingsSection>
  );
};
