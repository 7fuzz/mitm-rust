import { safeInvoke } from "../ipc";

/** All stored UI preferences, keyed without the backend `ui.` prefix. */
export const getUiPreferences = async (): Promise<Record<string, unknown>> => {
  return (await safeInvoke<Record<string, unknown> | undefined>("get_ui_preferences")) ?? {};
};

/** Persists one UI preference; `null` deletes it so the default applies again. */
export const setUiPreference = async (key: string, value: unknown): Promise<void> => {
  return await safeInvoke<void>("set_ui_preference", { key, value }, undefined);
};
