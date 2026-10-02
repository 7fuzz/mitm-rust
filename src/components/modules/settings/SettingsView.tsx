import React from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import type { SettingsSectionId } from '../../../stores/uiPrefs/registry';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { ProxyListenersSection } from './ProxyListenersSection';
import { CertificateSection } from './CertificateSection';
import { AppearanceSection } from './AppearanceSection';
import { DataSection } from './DataSection';
import { DatabaseSection } from './DatabaseSection';

const NAV: Array<{ id: SettingsSectionId; label: string; icon: string; hint: string }> = [
  { id: 'proxy', label: 'Proxy', icon: 'transfer_line', hint: 'Listeners, hosts and ports' },
  { id: 'certificate', label: 'Certificate', icon: 'key_line', hint: 'Root CA for HTTPS' },
  { id: 'appearance', label: 'Appearance', icon: 'sun_line', hint: 'Theme and font size' },
  { id: 'data', label: 'Data', icon: 'delete_2_line', hint: 'Clear logs, factory reset' },
  { id: 'database', label: 'Database', icon: 'storage_line', hint: 'Browse mitm.db' },
];

export const SettingsView: React.FC = () => {
  const [section, setSection] = useUiPref('settings.section');
  const listeners = useProxyStore((s) => s.listeners);
  const failedListeners = listeners.filter((l) => l.enabled && l.error).length;

  return (
    <div className="h-full flex bg-background overflow-hidden text-xs">
      {/* Section navigation */}
      <nav className="w-52 shrink-0 bg-surface border-r border-border flex flex-col py-3 px-2 gap-0.5 select-none">
        <div className="px-2 pb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Settings</div>
        {NAV.map((item) => {
          const isActive = item.id === section;
          return (
            <button
              key={item.id}
              onClick={() => setSection(item.id)}
              className={`flex items-center gap-2.5 px-2 py-1.5 rounded text-left transition-colors cursor-pointer ${
                isActive ? 'bg-primary/10 text-foreground' : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
              }`}
            >
              <MingCuteIcon name={item.icon} size={15} className={isActive ? 'text-primary' : ''} />
              <span className="min-w-0 flex-1">
                <span className={`block ${isActive ? 'font-semibold' : 'font-medium'}`}>{item.label}</span>
                <span className="block text-[10px] text-muted-foreground truncate">{item.hint}</span>
              </span>
              {item.id === 'proxy' && failedListeners > 0 && (
                <span
                  className="w-1.5 h-1.5 rounded-full bg-rose-400 shrink-0"
                  title={`${failedListeners} listener${failedListeners > 1 ? 's' : ''} failed to bind`}
                />
              )}
            </button>
          );
        })}
      </nav>

      {/* Active section */}
      <main className="flex-1 min-w-0 overflow-y-auto">
        {section === 'database' ? (
          <div className="h-full px-6 py-5">
            <DatabaseSection />
          </div>
        ) : (
          <div className="max-w-3xl px-6 py-5">
            {section === 'proxy' && <ProxyListenersSection />}
            {section === 'certificate' && <CertificateSection />}
            {section === 'appearance' && <AppearanceSection />}
            {section === 'data' && <DataSection />}
          </div>
        )}
      </main>
    </div>
  );
};
