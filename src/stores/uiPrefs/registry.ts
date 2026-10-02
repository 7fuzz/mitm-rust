import { buildDefaultColumnLayout, normalizeColumnLayout, type ColumnLayout } from "../../utils/columnLayout";

/**
 * Registry of persisted UI preferences. To add one, add an entry here with its default
 * (and a `normalize` if the stored shape can drift), then read it with `useUiPref(key)`.
 */
export interface UiPrefSpec<T> {
  default: T;
  /** Turns a stored value (possibly from an older version) into a valid one */
  normalize?: (stored: unknown, fallback: T) => T;
}

const pref = <T>(spec: UiPrefSpec<T>) => spec;

export const HISTORY_COLUMN_DEFAULTS = [
  { id: "id", width: 56 },
  { id: "method", width: 104 },
  { id: "source", width: 88 },
  { id: "host", width: 200 },
  { id: "path", width: 320 },
  { id: "status", width: 72 },
  { id: "contentType", width: 140 },
  { id: "size", width: 80 },
  { id: "duration", width: 80 },
  { id: "requestAt", width: 104 },
  { id: "responseAt", width: 104 },
];

type TriStateMap = Record<string, "include" | "exclude">;

/** History filter bar state (search is deliberately not persisted) */
export interface HistoryFiltersPref {
  methods: TriStateMap;
  statuses: TriStateMap;
  flags: TriStateMap;
  listener: string;
}

const normalizeTriStateMap = (value: unknown): TriStateMap => {
  if (!value || typeof value !== "object") return {};
  return Object.fromEntries(
    Object.entries(value).filter(([, st]) => st === "include" || st === "exclude")
  ) as TriStateMap;
};

export const UI_PREFS = {
  "history.columns": pref<ColumnLayout>({
    default: buildDefaultColumnLayout(HISTORY_COLUMN_DEFAULTS),
    normalize: normalizeColumnLayout,
  }),
  "history.filters": pref<HistoryFiltersPref>({
    default: { methods: {}, statuses: {}, flags: {}, listener: "" },
    normalize: (stored) => {
      const s = (stored && typeof stored === "object" ? stored : {}) as Partial<HistoryFiltersPref>;
      return {
        methods: normalizeTriStateMap(s.methods),
        statuses: normalizeTriStateMap(s.statuses),
        flags: normalizeTriStateMap(s.flags),
        listener: typeof s.listener === "string" ? s.listener : "",
      };
    },
  }),
};

export type UiPrefKey = keyof typeof UI_PREFS;
export type UiPrefValue<K extends UiPrefKey> = (typeof UI_PREFS)[K]["default"];
