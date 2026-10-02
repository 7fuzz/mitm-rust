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

/** Split position as a percentage, clamped to the range the resizers allow */
const splitPercent = (fallback: number) =>
  pref<number>({
    default: fallback,
    normalize: (stored, def) =>
      typeof stored === "number" && Number.isFinite(stored) ? Math.min(Math.max(stored, 15), 85) : def,
  });

/** Pixel width clamped to a resizer's range */
const panelWidth = (fallback: number, min: number, max: number) =>
  pref<number>({
    default: fallback,
    normalize: (stored, def) =>
      typeof stored === "number" && Number.isFinite(stored) ? Math.min(Math.max(stored, min), max) : def,
  });

export const REPEATER_SIDEBAR_WIDTH = { min: 220, max: 450 };
export const REPEATER_HISTORY_DRAWER_WIDTH = { min: 200, max: 550 };
export const COLLECTIONS_SIDEBAR_WIDTH = { min: 220, max: 450 };
export const COLLECTIONS_HISTORY_DRAWER_WIDTH = { min: 200, max: 550 };

export const SETTINGS_SECTIONS = ["proxy", "certificate", "appearance", "data", "database"] as const;
export type SettingsSectionId = (typeof SETTINGS_SECTIONS)[number];

export const UI_PREFS = {
  "history.columns": pref<ColumnLayout>({
    default: buildDefaultColumnLayout(HISTORY_COLUMN_DEFAULTS),
    normalize: normalizeColumnLayout,
  }),
  /** Height of the traffic table vs the inspector below it */
  "history.tableHeightPercent": splitPercent(50),
  /** Request pane share of the inspector */
  "history.inspectorSplitPercent": splitPercent(50),
  "repeater.sidebarWidth": panelWidth(290, REPEATER_SIDEBAR_WIDTH.min, REPEATER_SIDEBAR_WIDTH.max),
  "repeater.historyDrawerWidth": panelWidth(288, REPEATER_HISTORY_DRAWER_WIDTH.min, REPEATER_HISTORY_DRAWER_WIDTH.max),
  /** Request pane share of the Repeater work area */
  "repeater.requestSplitPercent": splitPercent(50),
  "collections.sidebarWidth": panelWidth(290, COLLECTIONS_SIDEBAR_WIDTH.min, COLLECTIONS_SIDEBAR_WIDTH.max),
  "collections.historyDrawerWidth": panelWidth(288, COLLECTIONS_HISTORY_DRAWER_WIDTH.min, COLLECTIONS_HISTORY_DRAWER_WIDTH.max),
  /** Request pane share of the Collections work area */
  "collections.requestSplitPercent": splitPercent(50),
  /** Settings page last opened */
  "settings.section": pref<SettingsSectionId>({
    default: "proxy",
    normalize: (stored, fallback) =>
      SETTINGS_SECTIONS.includes(stored as SettingsSectionId) ? (stored as SettingsSectionId) : fallback,
  }),
  "history.filterBarOpen": pref<boolean>({
    default: false,
    normalize: (stored, fallback) => (typeof stored === "boolean" ? stored : fallback),
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
