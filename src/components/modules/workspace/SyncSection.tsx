import { useTraffic } from '@/hooks/traffic';
import { Button } from '../../ui/Button';
import { useNotification } from '../../ui/NotificationProvider';
import { invoke } from '@/lib/utils/tauri';

interface SyncSectionProps {
  onOpenConfirm: () => void;
  openConfirm: (title: string, message: string, action: () => void) => void;
}

export function SyncSection({ onOpenConfirm, openConfirm }: SyncSectionProps) {
  const { syncAll, syncStatus, traffic } = useTraffic();
  const { notify } = useNotification();

  const handleSelectivePurge = (target: string) => {
    let title = '';
    let message = '';
    
    switch (target) {
      case 'repeater_only':
        title = 'Clear Repeater Requests';
        message = 'This will delete all requests and executed request history in the Repeater tab. Your collection groups will be preserved.';
        break;
      case 'repeater_groups':
        title = 'Clear Repeater Collections';
        message = 'This will delete all collections (groups), requests, and executed history in the Repeater tab.';
        break;
      case 'environments':
        title = 'Clear Environments & Variables';
        message = 'This will delete all environments, environment variables, and their values.';
        break;
      case 'history':
        title = 'Clear Traffic History';
        message = 'This will delete all captured HTTP traffic history logs.';
        break;
      default:
        return;
    }

    openConfirm(title, message, async () => {
      try {
        await invoke('purge_selective_data', { target });
        await syncAll(true);
        notify.success(`Successfully cleared selected database records.`);
      } catch (error) {
        notify.error(`Failed to clear records: ${error}`);
      }
    });
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
      <div className="bg-zinc-950 border border-zinc-800 rounded-xl p-6 shadow-2xl">
        <div className="flex items-center justify-between mb-8">
          <div className="space-y-1">
            <h2 className="text-xl font-black tracking-tighter text-zinc-100 flex items-center gap-3">
              <span className="w-2 h-8 bg-sky-500 rounded-full"></span>
              DATABASE_SYNCHRONIZATION
            </h2>
            <p className="text-xs text-zinc-500 font-mono tracking-wide uppercase opacity-70">Manage internal SQLite state and manual data fetches</p>
          </div>
          <Button
            variant="sky"
            size="lg"
            onClick={() => syncAll()}
            disabled={syncStatus.is_syncing}
            className="min-w-40"
          >
            {syncStatus.is_syncing ? 'SYNCING...' : 'MANUAL_SYNC'}
          </Button>
        </div>

        <div className="grid grid-cols-3 gap-6">
          <div className="bg-zinc-900/50 border border-zinc-800/50 p-4 rounded-lg space-y-2">
            <span className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block">Sync Status</span>
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${syncStatus.is_syncing ? 'bg-amber-500 animate-pulse' : syncStatus.error ? 'bg-rose-500' : 'bg-emerald-500'}`}></div>
              <span className={`text-xs font-black uppercase tracking-wider ${syncStatus.is_syncing ? 'text-amber-500' : syncStatus.error ? 'text-rose-500' : 'text-emerald-500'}`}>
                {syncStatus.is_syncing ? 'Synchronizing' : syncStatus.error ? 'Connection Error' : 'All Systems Nominal'}
              </span>
            </div>
          </div>

          <div className="bg-zinc-900/50 border border-zinc-800/50 p-4 rounded-lg space-y-2">
            <span className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block">Last Synchronized</span>
            <span className="text-xs font-mono text-zinc-400 block">
              {syncStatus.last_sync ? new Date(syncStatus.last_sync).toLocaleString() : 'NEVER'}
            </span>
          </div>

          <div className="bg-zinc-900/50 border border-zinc-800/50 p-4 rounded-lg space-y-2">
            <span className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block">Local Records</span>
            <span className="text-xs font-mono text-zinc-400 block">
              {traffic?.length || 0} History Items
            </span>
          </div>
        </div>

        {syncStatus.error && (
          <div className="mt-6 p-4 bg-rose-500/10 border border-rose-500/30 rounded-lg flex items-center gap-4 text-rose-400">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
            <div className="flex-1">
              <span className="text-[10px] font-black uppercase tracking-widest block mb-0.5">Critical Synchronizer Failure</span>
              <span className="text-xs font-mono opacity-80">{syncStatus.error}</span>
            </div>
            <Button variant="destructive" size="xs" onClick={() => syncAll()}>Retry Sync</Button>
          </div>
        )}
      </div>

      <div className="grid grid-cols-3 gap-6">
        <div className="p-6 bg-zinc-950 border border-zinc-800 rounded-xl space-y-4">
          <h3 className="text-[10px] font-black uppercase tracking-[0.3em] text-zinc-500 flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span>
            Persistence_Information
          </h3>
          <div className="space-y-3">
            {[
              { label: 'Database Type', value: 'SQLite 3.x (Local)' },
              { label: 'Storage Mode', value: 'Application Data Directory' },
              { label: 'Sync Engine', value: 'Rust / Tauri IPC' },
              { label: 'Auto-Sync', value: 'On Event (Captured)' }
            ].map(item => (
              <div key={item.label} className="flex items-center justify-between py-2 border-b border-zinc-900 last:border-0">
                <span className="text-[11px] text-zinc-500 font-medium">{item.label}</span>
                <span className="text-[11px] text-zinc-400 font-mono">{item.value}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="p-6 bg-zinc-950 border border-zinc-800 rounded-xl space-y-4">
          <h3 className="text-[10px] font-black uppercase tracking-[0.3em] text-zinc-500 flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-rose-500"></span>
            Selective_Cleanup
          </h3>
          <div className="grid grid-cols-1 gap-2">
            <Button variant="secondary" size="sm" className="justify-start px-4 h-11 border-zinc-800/80 hover:border-rose-500/30 text-zinc-300 hover:text-rose-400 transition-all hover:pl-5" onClick={() => handleSelectivePurge('repeater_only')}>
              <span className="flex-1 text-left text-xs font-bold">Clear Repeater Requests</span>
              <span className="text-[7px] opacity-40 font-mono uppercase tracking-widest">Keeps Groups</span>
            </Button>
            <Button variant="secondary" size="sm" className="justify-start px-4 h-11 border-zinc-800/80 hover:border-rose-500/30 text-zinc-300 hover:text-rose-400 transition-all hover:pl-5" onClick={() => handleSelectivePurge('repeater_groups')}>
              <span className="flex-1 text-left text-xs font-bold">Clear Repeater Groups</span>
              <span className="text-[7px] opacity-40 font-mono uppercase tracking-widest text-rose-300">Wipes all</span>
            </Button>
            <Button variant="secondary" size="sm" className="justify-start px-4 h-11 border-zinc-800/80 hover:border-rose-500/30 text-zinc-300 hover:text-rose-400 transition-all hover:pl-5" onClick={() => handleSelectivePurge('environments')}>
              <span className="flex-1 text-left text-xs font-bold">Clear Environments</span>
              <span className="text-[7px] opacity-40 font-mono uppercase tracking-widest text-rose-300">Wipes envs</span>
            </Button>
            <Button variant="secondary" size="sm" className="justify-start px-4 h-11 border-zinc-800/80 hover:border-rose-500/30 text-zinc-300 hover:text-rose-400 transition-all hover:pl-5" onClick={() => handleSelectivePurge('history')}>
              <span className="flex-1 text-left text-xs font-bold">Clear Traffic History</span>
              <span className="text-[7px] opacity-40 font-mono uppercase tracking-widest">Wipes logs</span>
            </Button>
          </div>
        </div>

        <div className="p-6 bg-zinc-950 border border-zinc-800 rounded-xl space-y-4">
          <h3 className="text-[10px] font-black uppercase tracking-[0.3em] text-zinc-500 flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span>
            Advanced_Operations
          </h3>
          <div className="grid grid-cols-1 gap-2">
            <Button variant="secondary" size="sm" className="justify-start px-4 h-11 border-zinc-800/80 hover:border-amber-500/30 transition-all hover:pl-5" onClick={() => notify.info('Maintenance: Vacuuming database...')}>
              <span className="flex-1 text-left text-xs font-bold">Vacuum Database</span>
              <span className="text-[7px] opacity-40 font-mono uppercase tracking-widest">Optimizes Disk</span>
            </Button>
            <Button variant="secondary" size="sm" className="justify-start px-4 h-11 border-zinc-800/80 hover:border-amber-500/30 transition-all hover:pl-5" onClick={() => notify.info('Diagnostics: Running integrity check...')}>
              <span className="flex-1 text-left text-xs font-bold">Integrity Check</span>
              <span className="text-[7px] opacity-40 font-mono uppercase tracking-widest">Verify DB</span>
            </Button>
            <Button variant="destructive" size="sm" className="justify-start px-4 h-11 transition-all hover:pl-5" onClick={onOpenConfirm}>
              <span className="flex-1 text-left text-xs font-bold">Purge All Data</span>
              <span className="text-[7px] opacity-60 font-mono uppercase tracking-widest text-rose-300">Wipe Data</span>
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
