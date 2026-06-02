import { useTraffic } from '@/hooks/traffic';
import { DEFAULT_SHORTCUTS } from '@/hooks/traffic/useConfig';

interface ShortcutHintProps {
  isOpen: boolean;
  simpleMode: boolean;
}

export function ShortcutHint({ isOpen, simpleMode }: ShortcutHintProps) {
  const { prefs } = useTraffic();
  if (!isOpen) return null;

  const s = prefs.shortcuts || DEFAULT_SHORTCUTS;

  const shortcuts = [
    { key: s.goto_history, label: 'History' },
    { key: s.goto_intercept, label: 'Intercept' },
    { key: s.goto_repeater, label: 'Repeater' },
  ];

  if (!simpleMode) {
    shortcuts.push({ key: s.goto_workspace, label: 'Workspace' });
    shortcuts.push({ key: s.goto_utilities, label: 'Utilities' });
    shortcuts.push({ key: s.open_variable_switcher, label: 'Variables' });
    shortcuts.push({ key: s.open_environment_switcher, label: 'Environments' });
  }

  shortcuts.push({ key: s.goto_options, label: 'Options' });

  return (
    <div className="fixed bottom-6 right-6 z-[100] animate-in fade-in slide-in-from-right-2 duration-200">
      <div className="bg-zinc-950/90 backdrop-blur-xl border border-zinc-800 rounded-lg shadow-[0_0_50px_rgba(0,0,0,0.5)] overflow-hidden min-w-[320px]">
        {/* Header / Title Bar */}
        <div className="bg-zinc-900/50 border-b border-zinc-800 px-4 py-2 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-black text-emerald-500 uppercase tracking-[0.2em]">Which_Key</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">Prefix:</span>
            <kbd className="px-1.5 py-0.5 bg-zinc-900 border border-zinc-700 rounded text-[10px] font-bold text-sky-text font-mono shadow-sm uppercase">
              {s.prefix_key}
            </kbd>
          </div>
        </div>

        {/* Shortcut Grid */}
        <div className="p-4 grid grid-cols-2 gap-x-8 gap-y-3">
          {shortcuts.map((sh) => (
            <div key={sh.key} className="flex items-center gap-3 group">
              <kbd className="min-w-6 h-6 flex items-center justify-center bg-zinc-900 border border-zinc-800 rounded text-[11px] font-black text-emerald-text font-mono group-hover:border-emerald-500/50 group-hover:bg-emerald-500/5 transition-all shadow-inner">
                {sh.key}
              </kbd>
              <div className="flex flex-col">
                <span className="text-[10px] font-bold text-zinc-300 uppercase tracking-widest group-hover:text-white transition-colors">
                  {sh.label}
                </span>
                <div className="h-0.5 w-0 group-hover:w-full bg-emerald-500/30 transition-all duration-300"></div>
              </div>
            </div>
          ))}
        </div>

        {/* Footer / Status Line */}
        <div className="bg-zinc-900/30 px-4 py-1.5 border-t border-zinc-800/50 flex items-center justify-between">
          <span className="text-[8px] text-zinc-600 font-bold uppercase tracking-widest">MITM_REAL_SHORTCUTS</span>
          <div className="flex items-center gap-1">
            <div className="w-1 h-1 rounded-full bg-emerald-500 animate-pulse"></div>
            <span className="text-[8px] text-zinc-500 font-bold uppercase tracking-widest">Waiting...</span>
          </div>
        </div>
      </div>
    </div>
  );
}
