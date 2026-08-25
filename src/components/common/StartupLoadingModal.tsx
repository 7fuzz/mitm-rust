import React, { useEffect, useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import {
  subscribeDbMigrationProgress,
  backupAndResetDatabase,
  exportDatabaseFile,
  quitApplication,
  runDatabaseMigrations,
  MigrationProgressPayload,
} from '../../services/tauri/bridge';
import { MingCuteIcon } from './MingCuteIcon';

export const StartupLoadingModal: React.FC = () => {
  const [visible, setVisible] = useState(true);
  const [progress, setProgress] = useState<MigrationProgressPayload | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const [infoMessage, setInfoMessage] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    subscribeDbMigrationProgress((payload) => {
      setProgress(payload);
      if (payload.isComplete && !payload.hasError) {
        setTimeout(() => {
          setVisible(false);
        }, 600);
      }
    })
      .then((fn) => {
        unlisten = fn;
        // Trigger initial migration run command
        runDatabaseMigrations().catch((err: any) => {
          const errMsg = typeof err === 'string' ? err : err?.message || 'Database migration initialization failed.';
          setProgress((prev) => ({
            step: prev?.step || 0,
            total: prev?.total || 3,
            name: 'error',
            status: 'Migration Failed',
            isComplete: false,
            hasError: true,
            errorMessage: errMsg,
          }));
        });
      })
      .catch(() => {
        // Fallback for non-Tauri browser dev mode
        setTimeout(() => {
          setVisible(false);
        }, 500);
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  if (!visible) return null;

  const hasError = progress?.hasError;
  const currentStep = progress?.step || 0;
  const totalSteps = progress?.total || 3;
  const progressPercent = Math.min(100, Math.max(5, Math.round((currentStep / totalSteps) * 100)));

  const handleBackupAndReset = async () => {
    try {
      setIsProcessing(true);
      setInfoMessage('Backing up old database and creating a clean database schema...');
      const resultMsg = await backupAndResetDatabase();
      setInfoMessage(resultMsg);
      setTimeout(() => {
        setVisible(false);
      }, 1000);
    } catch (err: any) {
      setInfoMessage(`Error resetting DB: ${err?.message || err}`);
      setIsProcessing(false);
    }
  };

  const handleExport = async () => {
    try {
      const selectedPath = await save({
        title: 'Export / Backup Current Database',
        defaultPath: 'mitm_backup.db',
        filters: [{ name: 'SQLite Database', extensions: ['db', 'sqlite'] }],
      });

      if (selectedPath) {
        setIsProcessing(true);
        await exportDatabaseFile(selectedPath);
        setInfoMessage(`Database exported successfully to: ${selectedPath}`);
        setIsProcessing(false);
      }
    } catch (err: any) {
      setInfoMessage(`Failed to export database: ${err?.message || err}`);
      setIsProcessing(false);
    }
  };

  const handleQuit = async () => {
    await quitApplication();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md transition-all duration-300">
      <div className="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl p-6 text-zinc-100 flex flex-col gap-5">
        {/* Header Branding */}
        <div className="flex items-center gap-3 border-b border-zinc-800/80 pb-4">
          <div className="w-9 h-9 rounded-lg bg-primary/10 border border-primary/30 flex items-center justify-center text-primary">
            <MingCuteIcon name="shield_shape_line" size={20} />
          </div>
          <div>
            <h2 className="text-base font-bold tracking-tight text-white">mitm-rust Studio</h2>
            <p className="text-xs text-zinc-400">Database Engine & Schema Manager</p>
          </div>
        </div>

        {/* Normal Progress State */}
        {!hasError ? (
          <div className="flex flex-col gap-4 py-2">
            <div className="flex justify-between items-center text-xs">
              <span className="font-mono text-zinc-300">
                {progress?.status || 'Initializing database connection...'}
              </span>
              <span className="font-mono text-primary font-semibold">
                {progressPercent}%
              </span>
            </div>

            {/* Glowing Animated Progress Bar */}
            <div className="w-full h-2 bg-zinc-800 rounded-full overflow-hidden relative">
              <div
                className="h-full bg-gradient-to-r from-primary/70 via-primary to-emerald-400 transition-all duration-300 rounded-full shadow-[0_0_12px_rgba(59,130,246,0.6)]"
                style={{ width: `${progressPercent}%` }}
              />
            </div>

            <div className="flex items-center justify-between text-[11px] text-zinc-500 font-mono">
              <span>Step {currentStep} of {totalSteps}</span>
              <span className="animate-pulse">Loading workspace...</span>
            </div>
          </div>
        ) : (
          /* Error & Interactive Recovery State */
          <div className="flex flex-col gap-4">
            <div className="p-3 bg-rose-500/10 border border-rose-500/30 rounded-lg flex items-start gap-3">
              <MingCuteIcon name="alert_line" size={20} className="text-rose-400 shrink-0 mt-0.5" />
              <div className="flex flex-col gap-1 text-xs">
                <span className="font-bold text-rose-400">Database Conflict Detected</span>
                <p className="text-zinc-300 leading-relaxed font-mono text-[11px]">
                  {progress?.errorMessage || 'A schema mismatch occurred on your existing database.'}
                </p>
              </div>
            </div>

            {infoMessage && (
              <div className="p-2.5 bg-emerald-500/10 border border-emerald-500/30 rounded text-xs text-emerald-300 font-mono">
                {infoMessage}
              </div>
            )}

            <p className="text-xs text-zinc-400 leading-normal">
              Your database requires an update. Choose how you would like to proceed with your data:
            </p>

            {/* Recovery Action Options */}
            <div className="flex flex-col gap-2.5 pt-1">
              <button
                type="button"
                disabled={isProcessing}
                onClick={handleBackupAndReset}
                className="w-full px-4 py-2.5 bg-primary hover:bg-primary/90 text-white rounded-lg text-xs font-semibold flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
              >
                <MingCuteIcon name="folder_archive_line" size={16} />
                Auto-Backup Old DB & Re-initialize Clean Schema
              </button>

              <button
                type="button"
                disabled={isProcessing}
                onClick={handleExport}
                className="w-full px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded-lg text-xs font-medium flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
              >
                <MingCuteIcon name="download_line" size={16} />
                Export / Save DB File to Custom Folder...
              </button>

              <button
                type="button"
                disabled={isProcessing}
                onClick={handleQuit}
                className="w-full px-4 py-2 bg-transparent hover:bg-rose-500/10 text-rose-400 hover:text-rose-300 border border-rose-500/20 rounded-lg text-xs font-medium flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
              >
                <MingCuteIcon name="close_circle_line" size={16} />
                Quit Application (Manual Inspection)
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
