import React, { useEffect, useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { SqliteBrowser } from './SqliteBrowser';
import { purgeAllData, purgeSelectiveData, getRootCaPem, isTauriAvailable } from '../../../services/tauri/ipc';
import { getProxyState } from '../../../services/tauri/bridge';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const SettingsView: React.FC = () => {
  const {
    theme,
    toggleTheme,
    layoutMode,
    setLayoutMode,
    fontSize,
    setFontSize,
    caPem,
    isCaLoading,
    fetchCaCert,
    exportCaCert,
    regenerateCaCert,
  } = useSettingsStore();
  const { proxyStatus, updateProxyBindings, clearTraffic } = useProxyStore();
  const { clearDeliveries } = useWebhookStore();

  const [bindingIp, setBindingIp] = useState(proxyStatus.bindings[0] || '0.0.0.0:8080');
  const [isUpdatingBinding, setIsUpdatingBinding] = useState(false);
  const [bindingFeedback, setBindingFeedback] = useState<{ type: 'success' | 'error'; message: string } | null>(null);

  const [caSuccessMsg, setCaSuccessMsg] = useState<string | null>(null);
  const [copiedPem, setCopiedPem] = useState(false);
  const [showPemPreview, setShowPemPreview] = useState(false);
  const [showTrustGuide, setShowTrustGuide] = useState(false);
  const [isSavingCa, setIsSavingCa] = useState(false);

  useEffect(() => {
    fetchCaCert();
    if (isTauriAvailable()) {
      getProxyState()
        .then((cfg) => {
          if (cfg) {
            const addr = `${cfg.host || '0.0.0.0'}:${cfg.port || 8080}`;
            setBindingIp(addr);
          }
        })
        .catch((err) => console.warn('Failed to load proxy state:', err));
    }
  }, [fetchCaCert]);

  const handleUpdateBindings = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!bindingIp.trim()) return;

    setIsUpdatingBinding(true);
    setBindingFeedback(null);
    try {
      await updateProxyBindings([bindingIp.trim()]);
      setBindingFeedback({
        type: 'success',
        message: `Proxy listener successfully updated to ${bindingIp.trim()}`,
      });
      setTimeout(() => setBindingFeedback(null), 5000);
    } catch (err: any) {
      console.error('Failed to update bindings:', err);
      setBindingFeedback({
        type: 'error',
        message: `Failed to update listener binding: ${err?.message || err}`,
      });
    } finally {
      setIsUpdatingBinding(false);
    }
  };

  const handleApplyPreset = (preset: string) => {
    setBindingIp(preset);
  };

  const handleSaveCa = async () => {
    try {
      setIsSavingCa(true);
      if (isTauriAvailable()) {
        const selectedPath = await save({
          title: 'Save Root CA Certificate',
          defaultPath: 'mitm-ca.crt',
          filters: [
            { name: 'Certificate Files (*.crt, *.pem, *.cer)', extensions: ['crt', 'pem', 'cer'] },
            { name: 'All Files (*.*)', extensions: ['*'] },
          ],
        });

        if (selectedPath) {
          await exportCaCert(selectedPath);
          setCaSuccessMsg(`Saved to ${selectedPath}`);
          setTimeout(() => setCaSuccessMsg(null), 4000);
        }
      } else {
        // Fallback for browser / preview mode
        const pem = caPem || (await getRootCaPem());
        const blob = new Blob([pem], { type: 'application/x-x509-ca-cert' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'mitm-ca.crt';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        setCaSuccessMsg('Downloaded mitm-ca.crt');
        setTimeout(() => setCaSuccessMsg(null), 4000);
      }
    } catch (err: any) {
      console.error('Failed to save CA certificate:', err);
      alert(`Failed to save CA certificate: ${err?.message || err}`);
    } finally {
      setIsSavingCa(false);
    }
  };

  const handleCopyPem = async () => {
    try {
      let pem = caPem;
      if (!pem) {
        pem = await getRootCaPem();
      }
      await navigator.clipboard.writeText(pem);
      setCopiedPem(true);
      setTimeout(() => setCopiedPem(false), 2500);
    } catch (err) {
      console.error('Failed to copy CA PEM:', err);
    }
  };

  const handleRegenerateCa = async () => {
    if (confirm('Regenerate Root CA Certificate? Existing browser & OS certificate trusts will need to be re-imported.')) {
      await regenerateCaCert();
      setCaSuccessMsg('New Root CA Generated!');
      setTimeout(() => setCaSuccessMsg(null), 3000);
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
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs flex flex-col justify-between">
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <MingCuteIcon name="settings_3_line" size={16} className="text-primary" />
                <span className="font-semibold text-foreground text-sm">Network & Proxy Listener Settings</span>
              </div>
              <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[10px] font-mono bg-primary/10 text-primary border border-primary/20 font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                Active: {proxyStatus.bindings[0] || '0.0.0.0:8080'}
              </span>
            </div>

            <p className="text-muted-foreground text-xs leading-relaxed">
              Configure the network interface and port where the MITM proxy listens for incoming HTTP/HTTPS traffic.
              <br />
              <span className="text-[11px] text-foreground/80 font-mono">
                <strong className="text-primary">0.0.0.0</strong> = All network interfaces (allows mobile/LAN devices to proxy), <strong className="text-primary">127.0.0.1</strong> = Localhost only.
              </span>
            </p>

            <form onSubmit={handleUpdateBindings} className="space-y-3">
              <div>
                <label className="block text-muted-foreground mb-1 font-medium text-xs">Proxy Listener Address / Port:</label>
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    value={bindingIp}
                    onChange={(e) => setBindingIp(e.target.value)}
                    placeholder="0.0.0.0:8080 or 8080"
                    className="flex-1 bg-background border border-border rounded px-3 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
                  />
                  <button
                    type="submit"
                    disabled={isUpdatingBinding}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-primary text-primary-foreground font-semibold rounded hover:bg-primary-hover transition-colors shadow-xs cursor-pointer disabled:opacity-50 text-xs shrink-0"
                  >
                    <MingCuteIcon name={isUpdatingBinding ? "loading_line" : "check_line"} size={13} className={isUpdatingBinding ? "animate-spin" : ""} />
                    <span>{isUpdatingBinding ? 'Updating...' : 'Update Listener'}</span>
                  </button>
                </div>
              </div>

              {/* Quick Presets */}
              <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
                <span className="text-[10px] text-muted-foreground font-mono mr-1">Presets:</span>
                <button
                  type="button"
                  onClick={() => handleApplyPreset('0.0.0.0:8080')}
                  className={`px-2 py-0.5 rounded text-[10px] font-mono border transition-colors cursor-pointer ${
                    bindingIp === '0.0.0.0:8080'
                      ? 'bg-primary/20 text-primary border-primary/40 font-bold'
                      : 'bg-background hover:bg-neutral-subtle border-border text-muted-foreground hover:text-foreground'
                  }`}
                  title="Listen on all interfaces (port 8080) - Recommended"
                >
                  0.0.0.0:8080 (All Interfaces)
                </button>
                <button
                  type="button"
                  onClick={() => handleApplyPreset('127.0.0.1:8080')}
                  className={`px-2 py-0.5 rounded text-[10px] font-mono border transition-colors cursor-pointer ${
                    bindingIp === '127.0.0.1:8080'
                      ? 'bg-primary/20 text-primary border-primary/40 font-bold'
                      : 'bg-background hover:bg-neutral-subtle border-border text-muted-foreground hover:text-foreground'
                  }`}
                  title="Listen on localhost only (port 8080)"
                >
                  127.0.0.1:8080 (Localhost)
                </button>
                <button
                  type="button"
                  onClick={() => handleApplyPreset('0.0.0.0:8888')}
                  className={`px-2 py-0.5 rounded text-[10px] font-mono border transition-colors cursor-pointer ${
                    bindingIp === '0.0.0.0:8888'
                      ? 'bg-primary/20 text-primary border-primary/40 font-bold'
                      : 'bg-background hover:bg-neutral-subtle border-border text-muted-foreground hover:text-foreground'
                  }`}
                  title="Listen on all interfaces (port 8888)"
                >
                  0.0.0.0:8888
                </button>
              </div>

              {/* Status Feedback Banner */}
              {bindingFeedback && (
                <div
                  className={`flex items-center gap-2 p-2.5 rounded-lg border font-mono text-[11px] animate-fade-in ${
                    bindingFeedback.type === 'success'
                      ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
                      : 'bg-rose-500/10 border-rose-500/30 text-rose-400'
                  }`}
                >
                  <MingCuteIcon
                    name={bindingFeedback.type === 'success' ? 'check_line' : 'alert_line'}
                    size={14}
                    className="shrink-0"
                  />
                  <span className="truncate">{bindingFeedback.message}</span>
                </div>
              )}
            </form>
          </div>
        </div>

        {/* Root CA Certificate Manager */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs flex flex-col justify-between">
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <MingCuteIcon name="key_line" size={16} className="text-primary" />
                <span className="font-semibold text-foreground text-sm">Root CA Certificate Manager</span>
              </div>
              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 font-medium">
                <MingCuteIcon name="lock_line" size={11} />
                TLS Active
              </span>
            </div>

            <p className="text-muted-foreground text-xs leading-relaxed">
              Save or export the Root CA certificate to install and trust it on your device, browser, or CLI for intercepting HTTPS traffic cleanly.
            </p>

            {/* Certificate metadata chips */}
            <div className="grid grid-cols-2 gap-2 bg-background border border-border/80 rounded-md p-2 text-[11px] font-mono">
              <div>
                <span className="text-muted-foreground block text-[10px]">Common Name (CN):</span>
                <span className="text-foreground font-medium">MITM Rust CA</span>
              </div>
              <div>
                <span className="text-muted-foreground block text-[10px]">Organization (O):</span>
                <span className="text-foreground font-medium">MITM Rust</span>
              </div>
              <div>
                <span className="text-muted-foreground block text-[10px]">Format:</span>
                <span className="text-foreground font-medium">X.509 / PEM (.crt)</span>
              </div>
              <div>
                <span className="text-muted-foreground block text-[10px]">Validity:</span>
                <span className="text-emerald-500 font-medium">10 Years (Trusted)</span>
              </div>
            </div>
          </div>

          {/* Action Buttons: Save CA, Copy PEM, Regenerate, View PEM */}
          <div className="space-y-2 pt-1">
            <div className="flex items-center gap-2 flex-wrap">
              <button
                onClick={handleSaveCa}
                disabled={isSavingCa || isCaLoading}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover transition-colors shadow-xs disabled:opacity-50 cursor-pointer"
                title="Save Root CA certificate file to disk"
              >
                <MingCuteIcon name="download_line" size={14} />
                <span>{isSavingCa ? 'Saving...' : 'Save CA Certificate (.crt)'}</span>
              </button>

              <button
                onClick={handleCopyPem}
                disabled={isCaLoading}
                className="flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-neutral-subtle border border-border text-foreground font-medium hover:bg-surface transition-colors cursor-pointer"
                title="Copy certificate PEM to clipboard"
              >
                <MingCuteIcon name={copiedPem ? 'check_line' : 'copy_line'} size={14} className={copiedPem ? 'text-emerald-500' : ''} />
                <span>{copiedPem ? 'Copied!' : 'Copy PEM'}</span>
              </button>

              <button
                onClick={() => setShowPemPreview(!showPemPreview)}
                className="flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-neutral-subtle border border-border text-foreground font-medium hover:bg-surface transition-colors cursor-pointer"
                title="Toggle certificate PEM text preview"
              >
                <MingCuteIcon name={showPemPreview ? 'eye_close_line' : 'eye_line'} size={14} />
                <span>{showPemPreview ? 'Hide PEM' : 'View PEM'}</span>
              </button>

              <button
                onClick={handleRegenerateCa}
                disabled={isCaLoading}
                className="flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-rose-500/10 border border-rose-500/20 text-rose-500 font-semibold hover:bg-rose-500/20 transition-colors cursor-pointer"
                title="Regenerate a brand new Root CA certificate"
              >
                <MingCuteIcon name="refresh_line" size={14} />
                <span>Regenerate CA</span>
              </button>
            </div>

            {/* Success Message Banner */}
            {caSuccessMsg && (
              <div className="flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 font-mono text-[11px] animate-fade-in">
                <MingCuteIcon name="check_line" size={13} className="text-emerald-400 shrink-0" />
                <span className="truncate">{caSuccessMsg}</span>
              </div>
            )}

            {/* Collapsible PEM Preview */}
            {showPemPreview && (
              <div className="mt-2 p-2 bg-background border border-border rounded-md text-[10px] font-mono text-muted-foreground relative">
                <div className="flex justify-between items-center pb-1 mb-1 border-b border-border/50 text-[10px] text-foreground font-semibold">
                  <span>Certificate PEM Preview</span>
                  <button
                    onClick={handleCopyPem}
                    className="text-primary hover:underline flex items-center gap-1 cursor-pointer"
                  >
                    <MingCuteIcon name="copy_line" size={11} />
                    {copiedPem ? 'Copied' : 'Copy'}
                  </button>
                </div>
                <textarea
                  readOnly
                  rows={5}
                  value={caPem || 'Loading Root CA certificate...'}
                  className="w-full bg-transparent text-foreground/80 font-mono text-[10px] resize-none focus:outline-none select-all"
                />
              </div>
            )}

            {/* Quick Trust Guide Toggle */}
            <div className="pt-1">
              <button
                onClick={() => setShowTrustGuide(!showTrustGuide)}
                className="text-[11px] text-muted-foreground hover:text-primary transition-colors flex items-center gap-1 cursor-pointer"
              >
                <MingCuteIcon name={showTrustGuide ? 'chevron_down_line' : 'chevron_right_line'} size={12} />
                <span>How to trust this CA on OS & Browsers</span>
              </button>

              {showTrustGuide && (
                <div className="mt-1.5 p-2 bg-background border border-border/80 rounded space-y-1.5 text-[11px] text-muted-foreground">
                  <div>
                    <strong className="text-foreground">macOS:</strong> Double-click the saved <code className="text-primary">.crt</code> file &rarr; Keychain Access &rarr; System &rarr; Always Trust.
                  </div>
                  <div>
                    <strong className="text-foreground">Windows:</strong> Double-click <code className="text-primary">.crt</code> &rarr; Install Certificate &rarr; Trusted Root Certification Authorities.
                  </div>
                  <div>
                    <strong className="text-foreground">Linux:</strong> Copy to <code className="text-primary">/usr/local/share/ca-certificates/mitm-ca.crt</code> and run <code className="text-primary">sudo update-ca-certificates</code>.
                  </div>
                  <div>
                    <strong className="text-foreground">Firefox:</strong> Settings &rarr; Privacy & Security &rarr; Certificates &rarr; View Certificates &rarr; Import.
                  </div>
                  <div>
                    <strong className="text-foreground">CLI / Node.js:</strong> <code className="text-primary">NODE_EXTRA_CA_CERTS=/path/to/mitm-ca.crt</code> or <code className="text-primary">curl --cacert mitm-ca.crt</code>.
                  </div>
                </div>
              )}
            </div>
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
