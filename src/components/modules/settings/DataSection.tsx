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
  const [failed, setFailed] = useState<{ action: Action; message: string } | null>(null);

  const runArmed = async (action: Action, run: () => Promise<void>) => {
    if (armed !== action) {
      setArmed(action);
      return;
    }
    setArmed(null);
    setFailed(null);
    try {
      await run();
    } catch (err) {
      setFailed({ action, message: String(err) });
      return;
    }
    setDone(action);
    setTimeout(() => setDone((d) => (d === action ? null : d)), 2500);
  };

  const actionButton = (action: Action, label: string, run: () => Promise<void>, icon = 'delete_2_line') => (
    <button
      onClick={() => runArmed(action, run)}
      onMouseLeave={() => setArmed((a) => (a === action ? null : a))}
      className={armed === action ? settingsButtonClass.dangerSolid : action === 'reset' ? settingsButtonClass.danger : settingsButtonClass.secondary}
    >
      <MingCuteIcon
        name={done === action ? 'check_line' : failed?.action === action ? 'alert_line' : icon}
        size={14}
        className={done === action ? 'text-emerald-500' : failed?.action === action ? 'text-rose-500' : ''}
      />
      <span>
        {armed === action ? 'Click again to confirm' : done === action ? 'Cleared' : failed?.action === action ? 'Failed, retry' : label}
      </span>
    </button>
  );

  /** Row help text, followed by the backend error when the last attempt failed */
  const describe = (action: Action, text: string) => (
    <>
      {text}
      {failed?.action === action && <span className="block text-rose-500 font-mono break-all">{failed.message}</span>}
    </>
  );

  return (
    <SettingsSection title="Data" description="Captured data is stored in the local SQLite database (mitm.db).">
      <SettingsGroup label="Captured data">
        <SettingsRow label="Traffic logs" description={describe('traffic', 'Every request and response in History.')}>
          {actionButton('traffic', 'Clear', clearTraffic)}
        </SettingsRow>
        <SettingsRow label="Webhook hits" description={describe('webhooks', 'Deliveries received by the Webhooks module.')}>
          {actionButton('webhooks', 'Clear', clearDeliveries)}
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup label="Danger zone" tone="danger">
        <SettingsRow
          label="Factory reset"
          description={describe(
            'reset',
            'Moves mitm.db aside as a timestamped backup (mitm.db.bak_…), starts a fresh empty database, and reloads. Workspaces, collections, environments, and logs all start over; the root CA is kept.'
          )}
        >
          {actionButton(
            'reset',
            'Factory reset',
            async () => {
              await backupAndResetDatabase();
              window.location.reload();
            },
            'alert_line'
          )}
        </SettingsRow>
      </SettingsGroup>
    </SettingsSection>
  );
};
