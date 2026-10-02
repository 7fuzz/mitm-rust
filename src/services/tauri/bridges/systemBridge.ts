import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface MigrationProgressPayload {
  step: number;
  total: number;
  name: string;
  status: string;
  isComplete: boolean;
  hasError: boolean;
  errorMessage?: string;
}

export const subscribeDbMigrationProgress = async (
  callback: (event: MigrationProgressPayload) => void
): Promise<UnlistenFn> => {
  return await listen<MigrationProgressPayload>("db-migration-progress", (e) => {
    callback(e.payload);
  });
};

export const runDatabaseMigrations = async (): Promise<void> => {
  return await invoke<void>("run_database_migrations");
};

export const backupAndResetDatabase = async (): Promise<string> => {
  return await invoke<string>("backup_and_reset_database");
};

export const exportDatabaseFile = async (destinationPath: string): Promise<void> => {
  return await invoke<void>("export_database_file", { destinationPath });
};

export const getRootCaPem = async (): Promise<string> => {
  return await invoke<string>("get_root_ca_pem");
};

export const exportRootCa = async (destinationPath: string): Promise<void> => {
  return await invoke<void>("export_root_ca", { destinationPath });
};

export const regenerateRootCa = async (): Promise<string> => {
  return await invoke<string>("regenerate_root_ca");
};

export const quitApplication = async (): Promise<void> => {
  return await invoke<void>("quit_application");
};

export const isTauriAvailable = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface OtherInstance {
  pid: number;
  startedAtMs: number;
}

/** The already-running instance this one is waiting on, or null when this is the only one */
export const getInstanceConflict = async (): Promise<OtherInstance | null> => {
  return await invoke<OtherInstance | null>("get_instance_conflict");
};

export const takeOverInstance = async (): Promise<void> => {
  return await invoke<void>("take_over_instance");
};
