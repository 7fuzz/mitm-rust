import { useEffect } from "react";
import { create } from "zustand";
import { getUiPreferences, setUiPreference } from "../services/tauri/bridges/uiPrefsBridge";
import { UI_PREFS, type UiPrefKey, type UiPrefSpec, type UiPrefValue } from "./uiPrefs/registry";

const PERSIST_DELAY_MS = 400;

interface UiPrefsState {
  /** Values set or loaded so far; missing keys fall back to the registry default */
  values: Partial<{ [K in UiPrefKey]: UiPrefValue<K> }>;
  loaded: boolean;
  load: () => Promise<void>;
  setPref: <K extends UiPrefKey>(key: K, value: UiPrefValue<K>) => void;
  resetPref: (key: UiPrefKey) => void;
}

const persistTimers: Partial<Record<UiPrefKey, ReturnType<typeof setTimeout>>> = {};

// Debounced per key so rapid updates (e.g. dragging a column edge) write once
const schedulePersist = (key: UiPrefKey, value: unknown) => {
  clearTimeout(persistTimers[key]);
  persistTimers[key] = setTimeout(() => {
    setUiPreference(key, value).catch((err) => console.warn(`Failed to save UI preference "${key}":`, err));
  }, PERSIST_DELAY_MS);
};

let loadPromise: Promise<void> | null = null;

export const useUiPrefsStore = create<UiPrefsState>((set, get) => ({
  values: {},
  loaded: false,

  load: () => {
    loadPromise ??= (async () => {
      try {
        const stored = await getUiPreferences();
        const values: UiPrefsState["values"] = {};
        for (const key of Object.keys(UI_PREFS) as UiPrefKey[]) {
          if (!(key in stored)) continue;
          const spec = UI_PREFS[key] as UiPrefSpec<unknown>;
          (values as Record<string, unknown>)[key] = spec.normalize
            ? spec.normalize(stored[key], spec.default)
            : stored[key];
        }
        // Values changed before loading finished win over the stored ones
        set({ values: { ...values, ...get().values }, loaded: true });
      } catch (err) {
        console.warn("Failed to load UI preferences:", err);
        loadPromise = null;
        set({ loaded: true });
      }
    })();
    return loadPromise;
  },

  setPref: (key, value) => {
    set({ values: { ...get().values, [key]: value } });
    schedulePersist(key, value);
  },

  resetPref: (key) => {
    const values = { ...get().values };
    delete values[key];
    set({ values });
    schedulePersist(key, null);
  },
}));

/** Non-React read of a UI preference (for stores); call after `load()` for the stored value. */
export function getUiPref<K extends UiPrefKey>(key: K): UiPrefValue<K> {
  return (useUiPrefsStore.getState().values[key] ?? UI_PREFS[key].default) as UiPrefValue<K>;
}

/** Reads a persisted UI preference, returning its default until a stored value is loaded. */
export function useUiPref<K extends UiPrefKey>(key: K) {
  const value = useUiPrefsStore((s) => s.values[key]) as UiPrefValue<K> | undefined;
  const setPref = useUiPrefsStore((s) => s.setPref);
  const resetPref = useUiPrefsStore((s) => s.resetPref);
  const load = useUiPrefsStore((s) => s.load);

  // No-op once loaded; retries if the startup load failed
  useEffect(() => {
    load();
  }, [load]);

  return [
    (value ?? UI_PREFS[key].default) as UiPrefValue<K>,
    (next: UiPrefValue<K>) => setPref(key, next),
    () => resetPref(key),
  ] as const;
}
