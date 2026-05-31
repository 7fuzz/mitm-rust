import { useState } from 'react';
import { useTraffic } from '@/hooks/traffic';
import { DEFAULT_SHORTCUTS } from '@/hooks/traffic/useConfig';
import { Button, Toggle } from '../ui';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { KeyboardShortcuts } from '@/hooks/traffic/types';
import { invoke } from '@/lib/utils/tauri';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { ConfirmModal } from '../Modals/ConfirmModal';
import { SyncSection } from '../modules/workspace/SyncSection';

const OPTION_SECTIONS = [
  { key: 'network', label: 'Network Binding', description: 'Configure proxy listeners and network capture settings.' },
  { key: 'ssl', label: 'SSL Certificates', description: 'Download or regenerate the local root CA.' },
  { key: 'database', label: 'Database', description: 'Control persistence, storage, and local save preferences.' },
  { key: 'sync', label: 'Sync', description: 'Manual sync status and history overview.' },
  { key: 'shortcuts', label: 'Keyboard Shortcuts', description: 'Customize hotkeys for app navigation.' },
] as const;

type OptionSectionKey = typeof OPTION_SECTIONS[number]['key'];

export function OptionsView() {
  const { prefs, updatePrefs, isProxyActive, setIsProxyActive, purgeAllData, uiLayout, updateUILayout } = useTraffic();

  const [bindings, setBindings] = useState<string[]>(['8080']);
  const [isSaving, setIsSaving] = useState(false);
  const [isRegenerating, setIsRegenerating] = useState(false);
  const [saveMessage, setSaveMessage] = useState('');
  const [selectedSection, setSelectedSection] = useState<OptionSectionKey>('network');
  const [confirmConfig, setConfirmConfig] = useState({ isOpen: false, title: '', message: '', action: () => {} });

  const openConfirm = (title: string, message: string, action: () => void) => {
    setConfirmConfig({ isOpen: true, title, message, action });
  };

  const closeConfirm = () => setConfirmConfig(prev => ({ ...prev, isOpen: false }));

  const handlePurge = async () => {
    setConfirmConfig(prev => ({ ...prev, isOpen: false }));
    try {
      await purgeAllData();
      setSaveMessage('All workspace data has been purged.');
    } catch (_err) {
      setSaveMessage('Error: Failed to purge database');
    }
    setTimeout(() => setSaveMessage(''), 3000);
  };

  const handleBindingChange = (index: number, value: string) => {
    const newBindings = [...bindings];
    newBindings[index] = value;
    setBindings(newBindings);
  };

  const addBinding = () => setBindings([...bindings, '']);

  const removeBinding = (index: number) => {
    if (bindings.length > 1) {
      setBindings(bindings.filter((_, i) => i !== index));
    }
  };

  const handleToggleProxy = async () => {
    const nextValue = !isProxyActive;
    try {
      await invoke('toggle_proxy', { enabled: nextValue });
      setIsProxyActive(nextValue);
      setSaveMessage(nextValue ? 'Proxy engine started' : 'Proxy engine stopped');
    } catch (e) {
      setSaveMessage(`Error: ${e}`);
    }
    setTimeout(() => setSaveMessage(''), 3000);
  };

  const handleSaveSettings = async () => {
    setIsSaving(true);
    setSaveMessage('');
    try {
      const cleanBindings = bindings.filter(b => b.trim() !== '');
      await invoke('update_network_settings', { bindings: cleanBindings });
      setSaveMessage('Network listeners updated successfully!');
      setBindings(cleanBindings.length > 0 ? cleanBindings : ['8080']);
    } catch (e) {
      setSaveMessage(`Error: ${e}`);
    }
    setIsSaving(false);
    setTimeout(() => setSaveMessage(''), 3000);
  };

  const handleDownloadCert = async () => {
    try {
      const pem = await invoke<string>('get_root_ca_pem');
      const filePath = await save({
        filters: [{
          name: 'Certificate',
          extensions: ['pem', 'crt']
        }],
        defaultPath: 'mitm-ca.pem'
      });

      if (filePath) {
        await writeTextFile(filePath, pem);
        setSaveMessage('Certificate saved successfully!');
      }
    } catch (e) {
      console.error(e);
      setSaveMessage('Error: Failed to save certificate');
    }
    setTimeout(() => setSaveMessage(''), 3000);
  };

  const handleRegenerateCert = async () => {
    if (!confirm('This will invalidate all current interceptions. Devices will need to re-trust the new certificate. Proceed?')) return;
    
    setIsRegenerating(true);
    try {
      await invoke('regenerate_root_ca');
      setSaveMessage('CA regenerated and proxy restarted!');
    } catch (e) {
      console.error(e);
      setSaveMessage('Error: Failed to regenerate CA');
    }
    setIsRegenerating(false);
    setTimeout(() => setSaveMessage(''), 3000);
  };

  const togglePref = (key: keyof typeof prefs) => {
    updatePrefs({ ...prefs, [key]: !prefs[key] });
  };

  const handleShortcutChange = (key: keyof KeyboardShortcuts, value: string) => {
    if (!prefs.shortcuts) return;
    updatePrefs({
      ...prefs,
      shortcuts: { ...prefs.shortcuts, [key]: value.toLowerCase() }
    });
  };

  const resetShortcuts = () => {
    updatePrefs({ ...prefs, shortcuts: DEFAULT_SHORTCUTS });
  };

  const shortcuts = prefs.shortcuts || DEFAULT_SHORTCUTS;

  const renderSection = () => {
    switch (selectedSection) {
      case 'network':
        return (
          <>
            <div className="grid grid-cols-3 gap-4 mb-8">
              <div className="p-4 bg-zinc-900 border border-zinc-800 rounded-lg shadow-sm hover:border-sky-500/30 transition-all">
                <Toggle
                  checked={isProxyActive}
                  onChange={handleToggleProxy}
                  label="Proxy_Engine"
                  subLabel={isProxyActive ? 'ACTIVE_LISTENING' : 'OFFLINE'}
                  variant="sky"
                />
              </div>

              <div className="p-4 bg-zinc-900 border border-zinc-800 rounded-lg shadow-sm hover:border-emerald-500/30 transition-all">
                <Toggle
                  checked={prefs.debugMode}
                  onChange={() => togglePref('debugMode')}
                  label="Debug_Mode"
                  subLabel={prefs.debugMode ? 'LOGGING_COMM' : 'SILENT'}
                  variant="emerald"
                />
              </div>

              <div className="p-4 bg-zinc-900 border border-zinc-800 rounded-lg shadow-sm hover:border-amber-500/30 transition-all">
                <Toggle
                  checked={prefs.simpleMode}
                  onChange={() => togglePref('simpleMode')}
                  label="Simple_Mode"
                  subLabel={prefs.simpleMode ? 'MINIMAL_UI' : 'FULL_SUITE'}
                  variant="amber"
                />
              </div>
            </div>

            <div className="p-6 border border-zinc-800 rounded bg-zinc-900/30 space-y-6">
              <h2 className="text-sky-text font-bold uppercase tracking-widest text-[10px] flex items-center gap-2">
                <span className="opacity-50">#</span> 1. Network_Binding
              </h2>

              <div className="space-y-4">
                <div className="flex items-center justify-between text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
                  <span>Listen Addresses (IP:PORT)</span>
                  <Button variant="ghost" size="xs" onClick={addBinding} className="text-sky-text hover:text-sky-300">+ Add Binding</Button>
                </div>

                <div className="space-y-2">
                  {bindings.map((bindStr, idx) => (
                    <div key={idx} className="flex items-center gap-2 group">
                      <input
                        type="text"
                        value={bindStr}
                        onChange={(e) => handleBindingChange(idx, e.target.value)}
                        placeholder="e.g. 8080 (All Interfaces) OR 127.0.0.1:8080 (Localhost)"
                        className="w-full bg-zinc-950 border border-zinc-700 p-3 rounded text-amber-400 font-black outline-none focus:border-sky-500 transition-colors text-xs font-mono"
                      />
                      {bindings.length > 1 && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => removeBinding(idx)}
                          className="p-3 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 opacity-0 group-hover:opacity-100 transition-all"
                        >
                          ✕
                        </Button>
                      )}
                    </div>
                  ))}
                </div>

                <p className="text-zinc-500 text-[10px] font-mono leading-relaxed bg-zinc-950 p-3 border border-zinc-800 rounded">
                  <span className="text-sky-text font-bold">Pro-tip:</span> Type just a port (e.g. <strong className="text-zinc-300">8080</strong>) to listen on all interfaces. Type an IP and port (e.g. <strong className="text-zinc-300">127.0.0.1:8888</strong>) to restrict access to a specific network.
                </p>
              </div>

              <div className="flex items-center justify-between pt-4 border-t border-zinc-800/50">
                <span className={`text-xs font-mono ${saveMessage.includes('Error') ? 'text-rose-400' : 'text-emerald-text'}`}>
                  {saveMessage}
                </span>
                <Button
                  variant="sky"
                  size="md"
                  onClick={handleSaveSettings}
                  disabled={isSaving}
                  className="min-w-32"
                >
                  {isSaving ? 'Rebinding...' : 'Apply & Restart'}
                </Button>
              </div>
            </div>
          </>
        );

      case 'ssl':
        return (
          <div className="p-6 border border-zinc-800 rounded bg-zinc-900/30 space-y-6">
            <h2 className="text-emerald-text font-bold uppercase tracking-widest text-[10px] flex items-center gap-2">
              <span className="opacity-50">#</span> 2. SSL_Certificates
            </h2>

            <div className="space-y-4">
              <p className="text-zinc-400 text-xs font-mono leading-relaxed">
                To intercept HTTPS traffic on your physical devices (iOS/Android) or external browsers, you must install and trust the root Certificate Authority (CA) generated by this proxy.
              </p>

              <div className="flex flex-col gap-3 bg-zinc-950 p-4 border border-zinc-800 border-dashed rounded">
                <h4 className="text-zinc-300 text-[10px] font-bold uppercase tracking-widest">Setup Instructions:</h4>
                <ol className="list-decimal list-inside text-xs text-zinc-500 font-mono space-y-2">
                  <li>Connect your device to the same Wi-Fi network.</li>
                  <li>Configure your device&apos;s proxy to point to your <strong className="text-sky-text">IP address</strong>.</li>
                  <li>Download the certificate below and transfer it to the device.</li>
                  <li>Go to device settings and explicitly <strong className="text-emerald-text">Trust the Root Certificate</strong>.</li>
                </ol>
              </div>

              <div className="flex gap-4">
                <Button
                  variant="primary"
                  size="lg"
                  onClick={handleDownloadCert}
                  className="flex-1"
                >
                  Download Root CA (.pem)
                </Button>
                <Button
                  variant="ghost"
                  size="lg"
                  onClick={handleRegenerateCert}
                  disabled={isRegenerating}
                  className="text-rose-500 hover:text-rose-400 hover:bg-rose-500/10 border border-zinc-800"
                >
                  {isRegenerating ? 'Regenerating...' : 'Regenerate'}
                </Button>
              </div>
            </div>
          </div>
        );

      case 'database':
        return (
          <div className="p-6 border border-zinc-800 rounded bg-zinc-900/30 space-y-6">
            <h2 className="text-purple-500 font-bold uppercase tracking-widest text-[10px] flex items-center gap-2">
              <span className="opacity-50">#</span> 3. Master_Database
            </h2>

            <p className="text-zinc-400 text-xs font-mono leading-relaxed mb-4">
              Select which configuration elements are permanently saved to the local SQLite database. Disabling a toggle will stop future saves, but will not erase existing data.
            </p>

            <div className="grid grid-cols-2 gap-4">
              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded hover:border-purple-500/50 transition-colors">
                <Toggle
                  checked={prefs.history}
                  onChange={() => togglePref('history')}
                  label="HTTP History"
                  subLabel="Logs traffic to DB"
                  variant="purple"
                />
              </div>

              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded hover:border-purple-500/50 transition-colors">
                <Toggle
                  checked={prefs.repeater}
                  onChange={() => togglePref('repeater')}
                  label="Repeater Workspace"
                  subLabel="Saves tabs & payloads"
                  variant="purple"
                />
              </div>

              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded hover:border-purple-500/50 transition-colors">
                <Toggle
                  checked={prefs.bindings}
                  onChange={() => togglePref('bindings')}
                  label="Network Bindings"
                  subLabel="Saves IP & Ports"
                  variant="purple"
                />
              </div>

              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded hover:border-purple-500/50 transition-colors">
                <Toggle
                  checked={prefs.intercept}
                  onChange={() => togglePref('intercept')}
                  label="Intercept Config"
                  subLabel="Saves rules & state"
                  variant="purple"
                />
              </div>

              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded hover:border-purple-500/50 transition-colors">
                <Toggle
                  checked={prefs.limits}
                  onChange={() => togglePref('limits')}
                  label="Memory Limits"
                  subLabel="Saves max history size"
                  variant="purple"
                />
              </div>

              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded hover:border-purple-500/50 transition-colors">
                <Toggle
                  checked={prefs.replacementsAutoSave}
                  onChange={() => togglePref('replacementsAutoSave')}
                  label="Auto-Save"
                  subLabel="Automatically save Workspace changes"
                  variant="purple"
                />
              </div>
            </div>
          </div>
        );

      case 'sync':
        return (
          <div className="space-y-8">
            <div className="flex items-center justify-between border-b border-zinc-800 pb-6 mb-8">
              <div>
                <h2 className="text-xl font-black tracking-tighter text-zinc-100 uppercase">Database & Sync</h2>
                <p className="text-xs text-zinc-500 font-mono">Manual synchronization controls and database overview.</p>
              </div>
            </div>

            <SyncSection
              onOpenConfirm={() => openConfirm(
                'Purge Database',
                'This will IRREVERSIBLY destroy all history, collections, and variables. The app will restart after purge.',
                handlePurge
              )}
            />
          </div>
        );

      case 'shortcuts':
        return (
          <div className="p-6 border border-zinc-800 rounded bg-zinc-900/30 space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-amber-500 font-bold uppercase tracking-widest text-[10px] flex items-center gap-2">
                <span className="opacity-50">#</span> 4. Keyboard_Shortcuts
              </h2>
              <Button variant="ghost" size="xs" onClick={resetShortcuts} className="text-amber-500 hover:text-amber-300">Reset to Default</Button>
            </div>

            <div className="grid grid-cols-2 gap-x-8 gap-y-6">
              <div className="space-y-4">
                <h4 className="text-[9px] text-zinc-500 font-black uppercase tracking-widest border-b border-zinc-800 pb-2">Global Prefix</h4>
                <div className="flex items-center justify-between gap-4">
                  <span className="text-xs text-zinc-300 font-mono">Prefix (e.g. &apos;g&apos;)</span>
                  <input 
                    type="text" maxLength={1} value={shortcuts.prefix_key} 
                    onChange={(e) => handleShortcutChange('prefix_key', e.target.value)}
                    className="w-12 bg-zinc-950 border border-zinc-700 p-2 rounded text-center text-amber-400 font-black text-xs font-mono"
                  />
                </div>
                <p className="text-[9px] text-zinc-600 font-mono leading-tight">Combine this with navigation keys below (e.g. g + h)</p>
              </div>

              <div className="space-y-4">
                <h4 className="text-[9px] text-zinc-500 font-black uppercase tracking-widest border-b border-zinc-800 pb-2">Tab Cycling</h4>
                <div className="flex items-center justify-between gap-4">
                  <span className="text-xs text-zinc-300 font-mono">Previous / Next</span>
                  <div className="flex gap-1">
                    <input 
                      type="text" maxLength={1} value={shortcuts.cycle_prev} 
                      onChange={(e) => handleShortcutChange('cycle_prev', e.target.value)}
                      className="w-8 bg-zinc-950 border border-zinc-700 p-2 rounded text-center text-emerald-text font-black text-xs font-mono"
                    />
                    <input 
                      type="text" maxLength={1} value={shortcuts.cycle_next} 
                      onChange={(e) => handleShortcutChange('cycle_next', e.target.value)}
                      className="w-8 bg-zinc-950 border border-zinc-700 p-2 rounded text-center text-emerald-text font-black text-xs font-mono"
                    />
                  </div>
                </div>
              </div>

              <div className="space-y-4">
                <h4 className="text-[9px] text-zinc-500 font-black uppercase tracking-widest border-b border-zinc-800 pb-2">Navigation (Prefix + Key)</h4>
                {[
                  { label: 'History', key: 'goto_history' },
                  { label: 'Intercept', key: 'goto_intercept' },
                  { label: 'Repeater', key: 'goto_repeater' },
                  { label: 'Workspace', key: 'goto_workspace' },
                  { label: 'Utilities', key: 'goto_utilities' },
                  { label: 'Options', key: 'goto_options' },
                ].map(item => (
                  <div key={item.key} className="flex items-center justify-between gap-4">
                    <span className="text-xs text-zinc-400 font-mono">{item.label}</span>
                    <input 
                      type="text" maxLength={1} value={(shortcuts as any)[item.key]} 
                      onChange={(e) => handleShortcutChange(item.key as any, e.target.value)}
                      className="w-10 bg-zinc-950 border border-zinc-700 p-1.5 rounded text-center text-sky-text font-black text-xs font-mono"
                    />
                  </div>
                ))}
              </div>

              <div className="space-y-4">
                <h4 className="text-[9px] text-zinc-500 font-black uppercase tracking-widest border-b border-zinc-800 pb-2">Quick Switchers (Prefix + Key)</h4>
                {[
                  { label: 'Variables', key: 'open_variable_switcher' },
                  { label: 'Environments', key: 'open_environment_switcher' },
                ].map(item => (
                  <div key={item.key} className="flex items-center justify-between gap-4">
                    <span className="text-xs text-zinc-400 font-mono">{item.label}</span>
                    <input 
                      type="text" maxLength={1} value={(shortcuts as any)[item.key]} 
                      onChange={(e) => handleShortcutChange(item.key as any, e.target.value)}
                      className="w-10 bg-zinc-950 border border-zinc-700 p-1.5 rounded text-center text-purple-400 font-black text-xs font-mono"
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <WorkspaceLayout
      uiLayout={uiLayout}
      onUpdateLayout={updateUILayout}
      listComponent={() => (
        <div className="flex flex-col gap-4 p-4 bg-zinc-950 border-b border-zinc-800">
          <div>
            <div className="text-[11px] font-black uppercase tracking-[0.3em] text-zinc-500 mb-3">Options Sections</div>
            <p className="text-xs text-zinc-400 font-mono leading-relaxed">Choose an options category to view related controls.</p>
          </div>

          <div className="flex flex-col gap-2">
            {OPTION_SECTIONS.map(section => (
              <button
                key={section.key}
                type="button"
                onClick={() => setSelectedSection(section.key)}
                className={`text-left p-3 rounded-xl transition-all border ${selectedSection === section.key ? 'border-sky-500/40 bg-sky-500/10 text-sky-text' : 'border-zinc-800 bg-zinc-950 text-zinc-300 hover:border-zinc-700 hover:bg-zinc-900'}`}
              >
                <div className="font-semibold text-sm uppercase tracking-[0.12em]">{section.label}</div>
                <div className="text-[10px] text-zinc-500 leading-snug mt-1">{section.description}</div>
              </button>
            ))}
          </div>
        </div>
      )}
      mainContent={(_splitMode) => (
        <div className="space-y-8">
          <div className="flex flex-col gap-4 border-b border-zinc-800 pb-6">
            <div>
              <h1 className="text-2xl font-black text-zinc-50 tracking-tighter uppercase mb-2">Proxy_Options</h1>
              <p className="text-zinc-500 text-xs font-mono">Configure local network bindings, certificate management, database behavior, and sync operations.</p>
            </div>
            <div className="flex flex-wrap gap-2 xl:hidden">
              {OPTION_SECTIONS.map(section => (
                <button
                  key={section.key}
                  type="button"
                  onClick={() => setSelectedSection(section.key)}
                  className={`text-[10px] uppercase tracking-[0.25em] rounded-full px-3 py-2 transition-all ${selectedSection === section.key ? 'bg-sky-500/15 text-sky-text border border-sky-500/30' : 'bg-zinc-900 text-zinc-400 border border-zinc-800 hover:border-zinc-700 hover:text-zinc-200'}`}
                >
                  {section.label}
                </button>
              ))}
            </div>
          </div>

          {renderSection()}
        </div>
      )}
      toolbarLeft={null}
      toolbarRight={null}
    >
      <ConfirmModal
        isOpen={confirmConfig.isOpen}
        title={confirmConfig.title}
        message={confirmConfig.message}
        isDestructive={true}
        onClose={closeConfirm}
        onConfirm={confirmConfig.action}
      />
    </WorkspaceLayout>
  );
}
