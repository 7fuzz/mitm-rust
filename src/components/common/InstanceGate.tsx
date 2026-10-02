import React, { useEffect, useState } from 'react';
import { getInstanceConflict, quitApplication, takeOverInstance, type OtherInstance } from '../../services/tauri/bridge';
import { isTauriAvailable } from '../../services/tauri/ipc';
import { MingCuteIcon } from './MingCuteIcon';

type GateState = { phase: 'checking' } | { phase: 'clear' } | { phase: 'conflict'; other: OtherInstance };

/** Holds the whole app back while another instance is running, until the user picks which one to keep. */
export const InstanceGate: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [gate, setGate] = useState<GateState>(() => (isTauriAvailable() ? { phase: 'checking' } : { phase: 'clear' }));
  const [isTakingOver, setIsTakingOver] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (gate.phase !== 'checking') return;
    getInstanceConflict()
      .then((other) => setGate(other ? { phase: 'conflict', other } : { phase: 'clear' }))
      .catch(() => setGate({ phase: 'clear' }));
  }, [gate.phase]);

  if (gate.phase === 'clear') return <>{children}</>;
  if (gate.phase === 'checking') return <div className="h-screen w-screen bg-background" />;

  const handleTakeOver = async () => {
    setIsTakingOver(true);
    setError(null);
    try {
      await takeOverInstance();
      setGate({ phase: 'clear' });
    } catch (err) {
      setError(String(err));
    } finally {
      setIsTakingOver(false);
    }
  };

  return (
    <div className="h-screen w-screen bg-background flex items-center justify-center p-6 text-xs font-sans">
      <div className="w-full max-w-md bg-surface border border-border rounded-xl shadow-xl p-5 space-y-4">
        <div className="flex items-start gap-3">
          <div className="w-9 h-9 rounded-lg bg-amber-500/10 text-amber-500 flex items-center justify-center shrink-0">
            <MingCuteIcon name="alert_line" size={18} />
          </div>
          <div className="space-y-1">
            <h1 className="text-sm font-semibold text-foreground">mitm is already running</h1>
            <p className="text-muted-foreground leading-relaxed">
              Another instance (PID <span className="font-mono text-foreground">{gate.other.pid}</span>, started{' '}
              {new Date(gate.other.startedAtMs).toLocaleString()}) owns the proxy ports. Only one instance can run at a
              time. Nothing has been started in this window yet.
            </p>
          </div>
        </div>

        {error && (
          <div className="flex items-start gap-1.5 px-2.5 py-2 rounded border border-rose-500/30 bg-rose-500/10 text-rose-500 text-2xs">
            <MingCuteIcon name="alert_line" size={13} className="shrink-0 mt-px" />
            <span>{error}</span>
          </div>
        )}

        <div className="flex flex-col gap-2">
          <button
            onClick={handleTakeOver}
            disabled={isTakingOver}
            className="flex items-center justify-center gap-1.5 px-3 py-2 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover disabled:opacity-60 cursor-pointer"
          >
            <MingCuteIcon name={isTakingOver ? 'loading_line' : 'refresh_line'} size={14} className={isTakingOver ? 'animate-spin' : ''} />
            <span>{isTakingOver ? 'Closing the other instance...' : 'Close the other instance and use this one'}</span>
          </button>
          <button
            onClick={() => quitApplication()}
            disabled={isTakingOver}
            className="flex items-center justify-center gap-1.5 px-3 py-2 rounded border border-border bg-background text-foreground font-medium hover:bg-neutral-subtle disabled:opacity-60 cursor-pointer"
          >
            <MingCuteIcon name="close_line" size={14} />
            <span>Close this window</span>
          </button>
        </div>
      </div>
    </div>
  );
};
