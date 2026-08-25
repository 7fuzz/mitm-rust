import React, { useState } from 'react';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { SqliteBrowser } from './SqliteBrowser';
import { updateNetworkSettings, purgeAllData, purgeSelectiveData } from '../../../services/tauri/ipc';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const SettingsView: React.FC = () => {
  const { theme, toggleTheme, layoutMode, setLayoutMode, fontSize, setFontSize, regenerateCaCert } =
    useSettingsStore();
  const { proxyStatus, clearTraffic } = useProxyStore();
  const { clearDeliveries } = useWebhookStore();

  const [bindingIp, setBindingIp] = useState(proxyStatus.bindings[0] || '127.0.0.1:8080');
  const [caSuccessMsg, setCaSuccessMsg] = useState(false);

  const handleUpdateBindings = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await updateNetworkSettings([bindingIp]);
      alert(`Proxy network binding updated to ${bindingIp}`);
    } catch (err) {
      console.error('Failed to update bindings:', err);
    }
  };

  const handleRegenerateCa = async () => {
    if (confirm('Regenerate Root CA Certificate? Existing browser trusts will need to be re-imported.')) {
      await regenerateCaCert();
      setCaSuccessMsg(true);
      setTimeout(() => setCaSuccessMsg(false), 3000);
    }
  };

  const handleClearTraffic = async () => {
    clearTraffic();
    await purgeSelectiveData('traffic');
    alert('Traffic logs cleared.');
  };

  const handleFactoryReset = async () => {
    if (confirm('DANGER: Perform factory reset? This will wipe all logs, requests, environments, and CA certificates.')) {
      await purgeAllData();
      alert('Factory reset completed.');
      window.location.reload();
    }
  };

  return (
    <div className="h-full flex flex-col bg-background p-4 overflow-y-auto space-y-4 text-xs">
      {/* Network Settings & Root CA */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Proxy Binding Settings */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="settings_3_line" size={16} className="text-primary" />
            <span className="font-semibold text-foreground text-sm">Network & Proxy Listener Settings</span>
          </div>

          <form onSubmit={handleUpdateBindings} className="space-y-3">
            <div>
              <label className="block text-muted-foreground mb-1 font-medium">Proxy Listener Address / Port:</label>
              <input
                type="text"
                value={bindingIp}
                onChange={(e) => setBindingIp(e.target.value)}
                placeholder="127.0.0.1:8080 or 8080"
                className="w-full bg-background border border-border rounded px-3 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
              />
            </div>
            <button
              type="submit"
              className="px-4 py-1.5 bg-primary text-primary-foreground font-semibold rounded hover:bg-primary-hover transition-colors shadow-xs"
            >
              Update Listener Bindings
            </button>
          </form>
        </div>

        {/* Root CA Certificate Manager */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="key_line" size={16} className="text-primary" />
            <span className="font-semibold text-foreground text-sm">Root CA Certificate Manager</span>
          </div>

          <p className="text-muted-foreground text-xs">
            Regenerate or reinstall Root CA certificates for intercepting TLS/HTTPS connections cleanly.
          </p>

          <div className="flex items-center gap-3 pt-1">
            <button
              onClick={handleRegenerateCa}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-rose-500/10 border border-rose-500/20 text-rose-500 font-semibold hover:bg-rose-500/20 transition-colors"
            >
              <MingCuteIcon name="refresh_line" size={14} />
              <span>Regenerate Root CA</span>
            </button>
            {caSuccessMsg && (
              <span className="text-emerald-500 font-bold font-mono text-xs">New Root CA Generated!</span>
            )}
          </div>
        </div>
      </div>

      {/* Embedded SQLite Browser */}
      <SqliteBrowser />

      {/* Maintenance, Data Purging & UI Customization */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Data Purging & Emergency Reset Controls */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="delete_2_line" size={16} className="text-rose-500" />
            <span className="font-semibold text-foreground text-sm">Data Maintenance & Purging</span>
          </div>

          <div className="flex items-center gap-2 flex-wrap">
            <button
              onClick={handleClearTraffic}
              className="px-3 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium"
            >
              Clear Traffic Logs
            </button>
            <button
              onClick={clearDeliveries}
              className="px-3 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium"
            >
              Clear Webhook Hits
            </button>
          </div>

          <div className="pt-2 border-t border-border">
            <button
              onClick={handleFactoryReset}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-rose-600 text-white font-bold hover:bg-rose-500 transition-colors shadow-xs"
            >
              <MingCuteIcon name="delete_2_line" size={14} />
              <span>Emergency Factory Reset</span>
            </button>
          </div>
        </div>

        {/* UI Customization & Shortcuts */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="tool_line" size={16} className="text-primary" />
            <span className="font-semibold text-foreground text-sm">UI Customization & Display</span>
          </div>

          <div className="space-y-2 font-mono">
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">Default Split Mode:</span>
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setLayoutMode('vertical')}
                  className={`px-2 py-0.5 rounded text-xs ${
                    layoutMode === 'vertical' ? 'bg-primary text-primary-foreground font-bold' : 'bg-neutral-subtle text-muted-foreground'
                  }`}
                >
                  Vertical
                </button>
                <button
                  onClick={() => setLayoutMode('horizontal')}
                  className={`px-2 py-0.5 rounded text-xs ${
                    layoutMode === 'horizontal' ? 'bg-primary text-primary-foreground font-bold' : 'bg-neutral-subtle text-muted-foreground'
                  }`}
                >
                  Horizontal
                </button>
              </div>
            </div>

            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">UI Font Scale:</span>
              <div className="flex items-center gap-1">
                {(['sm', 'md', 'lg'] as const).map((s) => (
                  <button
                    key={s}
                    onClick={() => setFontSize(s)}
                    className={`px-2 py-0.5 rounded text-xs uppercase ${
                      fontSize === s ? 'bg-primary text-primary-foreground font-bold' : 'bg-neutral-subtle text-muted-foreground'
                    }`}
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">Theme Mode:</span>
              <button
                onClick={toggleTheme}
                className="px-3 py-1 bg-neutral-subtle border border-border rounded text-foreground font-semibold uppercase text-xs"
              >
                {theme}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
