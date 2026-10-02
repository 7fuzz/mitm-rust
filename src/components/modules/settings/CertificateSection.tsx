import React, { useEffect, useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { getRootCaPem, isTauriAvailable } from '../../../services/tauri/ipc';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { SegmentedControl } from '../../common/ui';
import { SettingsGroup, SettingsRow, SettingsSection, settingsButtonClass } from './SettingsSection';

const CA_DETAILS = [
  { label: 'Common name', value: 'MITM Rust CA' },
  { label: 'Organization', value: 'MITM Rust' },
  { label: 'Format', value: 'X.509 / PEM (.crt)' },
  { label: 'Validity', value: '10 years' },
];

const code = (text: string) => <code className="font-mono text-primary">{text}</code>;

const TRUST_GUIDES: Array<{ value: string; label: string; steps: React.ReactNode }> = [
  {
    value: 'macos',
    label: 'macOS',
    steps: <>Double-click the saved {code('.crt')} file → Keychain Access → System → set it to Always Trust.</>,
  },
  {
    value: 'windows',
    label: 'Windows',
    steps: <>Double-click the {code('.crt')} file → Install Certificate → Trusted Root Certification Authorities.</>,
  },
  {
    value: 'linux',
    label: 'Linux',
    steps: (
      <>
        Copy it to {code('/usr/local/share/ca-certificates/mitm-ca.crt')} and run {code('sudo update-ca-certificates')}.
      </>
    ),
  },
  {
    value: 'firefox',
    label: 'Firefox',
    steps: <>Settings → Privacy & Security → Certificates → View Certificates → Authorities → Import.</>,
  },
  {
    value: 'cli',
    label: 'CLI / Node',
    steps: (
      <>
        Set {code('NODE_EXTRA_CA_CERTS=/path/to/mitm-ca.crt')}, or pass {code('curl --cacert mitm-ca.crt')}.
      </>
    ),
  },
];

export const CertificateSection: React.FC = () => {
  const { caPem, isCaLoading, fetchCaCert, exportCaCert, regenerateCaCert } = useSettingsStore();

  const [notice, setNotice] = useState<string | null>(null);
  const [copiedPem, setCopiedPem] = useState(false);
  const [showPem, setShowPem] = useState(false);
  const [isSavingCa, setIsSavingCa] = useState(false);
  const [confirmRegenerate, setConfirmRegenerate] = useState(false);
  const [guide, setGuide] = useState(TRUST_GUIDES[0].value);

  useEffect(() => {
    fetchCaCert();
  }, [fetchCaCert]);

  const flash = (message: string) => {
    setNotice(message);
    setTimeout(() => setNotice(null), 4000);
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
          flash(`Saved to ${selectedPath}`);
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
        flash('Downloaded mitm-ca.crt');
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
      await navigator.clipboard.writeText(caPem || (await getRootCaPem()));
      setCopiedPem(true);
      setTimeout(() => setCopiedPem(false), 2500);
    } catch (err) {
      console.error('Failed to copy CA PEM:', err);
    }
  };

  const handleRegenerate = async () => {
    if (!confirmRegenerate) {
      setConfirmRegenerate(true);
      return;
    }
    setConfirmRegenerate(false);
    await regenerateCaCert();
    flash('New root CA generated. Re-install it wherever the old one was trusted.');
  };

  const activeGuide = TRUST_GUIDES.find((g) => g.value === guide) ?? TRUST_GUIDES[0];

  return (
    <SettingsSection
      title="Certificate"
      description="Install and trust this root CA on a device, browser, or CLI so its HTTPS traffic can be decrypted."
      aside={
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-3xs font-mono bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 font-medium">
          <MingCuteIcon name="lock_line" size={11} />
          TLS active
        </span>
      }
    >
      <SettingsGroup label="Root CA">
        <dl className="px-3 py-2.5 grid grid-cols-2 sm:grid-cols-4 gap-3">
          {CA_DETAILS.map((d) => (
            <div key={d.label} className="min-w-0">
              <dt className="text-muted-foreground text-3xs uppercase tracking-wider">{d.label}</dt>
              <dd className="text-foreground font-mono text-2xs truncate">{d.value}</dd>
            </div>
          ))}
        </dl>

        <div className="px-3 py-2.5 flex items-center gap-2 flex-wrap">
          <button onClick={handleSaveCa} disabled={isSavingCa || isCaLoading} className={settingsButtonClass.primary}>
            <MingCuteIcon name="download_line" size={14} />
            <span>{isSavingCa ? 'Saving...' : 'Save certificate (.crt)'}</span>
          </button>
          <button onClick={handleCopyPem} disabled={isCaLoading} className={settingsButtonClass.secondary}>
            <MingCuteIcon name={copiedPem ? 'check_line' : 'copy_line'} size={14} className={copiedPem ? 'text-emerald-500' : ''} />
            <span>{copiedPem ? 'Copied' : 'Copy PEM'}</span>
          </button>
          <button onClick={() => setShowPem(!showPem)} className={settingsButtonClass.secondary}>
            <MingCuteIcon name={showPem ? 'eye_close_line' : 'eye_line'} size={14} />
            <span>{showPem ? 'Hide PEM' : 'View PEM'}</span>
          </button>
          {notice && (
            <span className="flex items-center gap-1 text-emerald-500 text-2xs font-mono min-w-0 animate-fade-in">
              <MingCuteIcon name="check_line" size={13} className="shrink-0" />
              <span className="truncate">{notice}</span>
            </span>
          )}
        </div>

        {showPem && (
          <div className="px-3 py-2.5">
            <textarea
              readOnly
              rows={8}
              value={caPem || 'Loading Root CA certificate...'}
              className="w-full bg-background border border-border rounded p-2 text-foreground/80 font-mono text-3xs resize-none focus:outline-none select-all"
            />
          </div>
        )}
      </SettingsGroup>

      <SettingsGroup label="How to trust it">
        <div className="px-3 py-2.5 space-y-2">
          <SegmentedControl value={guide} onChange={setGuide} options={TRUST_GUIDES} />
          <p className="text-muted-foreground text-2xs leading-relaxed">{activeGuide.steps}</p>
        </div>
      </SettingsGroup>

      <SettingsGroup label="Danger zone" tone="danger">
        <SettingsRow
          label="Regenerate root CA"
          description="Creates a new certificate. Every device and browser that trusts the current one has to re-import it."
        >
          <button
            onClick={handleRegenerate}
            onMouseLeave={() => setConfirmRegenerate(false)}
            disabled={isCaLoading}
            className={confirmRegenerate ? settingsButtonClass.dangerSolid : settingsButtonClass.danger}
          >
            <MingCuteIcon name="refresh_line" size={14} />
            <span>{confirmRegenerate ? 'Click again to regenerate' : 'Regenerate'}</span>
          </button>
        </SettingsRow>
      </SettingsGroup>
    </SettingsSection>
  );
};
