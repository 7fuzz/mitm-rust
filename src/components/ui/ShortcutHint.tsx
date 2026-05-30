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
    <div className="fixed bottom-12 left-1/2 -translate-x-1/2 z-[100] animate-in fade-in slide-in-from-bottom-4 duration-200">
      <div className="bg-zinc-900/90 backdrop-blur-md border border-zinc-700/50 rounded-xl shadow-2xl p-1 flex gap-1 items-center">
        <div className="px-3 py-1.5 text-zinc-500 font-black text-[10px] uppercase tracking-widest border-r border-zinc-800 mr-1">
          {s.prefix_key.toUpperCase()} +
        </div>
        {shortcuts.map((sh) => (
          <div key={sh.key} className="flex items-center gap-2 px-3 py-1.5 rounded-lg hover:bg-zinc-900 transition-colors">
            <kbd className="min-w-5 h-5 flex items-center justify-center bg-zinc-950 border border-zinc-700 rounded text-[10px] font-bold text-emerald-text uppercase">
              {sh.key}
            </kbd>
            <span className="text-[11px] font-bold text-zinc-300 uppercase tracking-wider">
              {sh.label}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

