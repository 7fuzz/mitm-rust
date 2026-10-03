import { create } from 'zustand';
import type {
  AttackType,
  FuzzConfig,
  FuzzResult,
  FuzzRunMeta,
  FuzzTemplate,
  FuzzVariable,
  MatchResult,
  PayloadSet,
} from '../services/tauri/bridge';
import {
  countFuzzRequests,
  deleteFuzzRun,
  getFuzzResult,
  getFuzzRunConfig,
  getFuzzRunResults,
  listFuzzRuns,
  listenFuzzDone,
  listenFuzzResult,
  listenFuzzStarted,
  saveCurrentFuzz,
  startFuzz,
  stopFuzz,
} from '../services/tauri/bridge';
import { isTauriAvailable } from '../services/tauri/ipc';
import { parseVariables } from '../utils/fuzzerMarkers';
import { tryPrettifyJson } from '../utils/prettifyJson';
import type { TrafficItem } from '../types';

/** One table row, filled from the lightweight result stream. */
export interface FuzzRow {
  idx: number;
  payloads: string[];
  statusCode: number;
  responseSize: number;
  durationMs: number;
  error?: string;
  matches: MatchResult[];
}

export const emptyPayloadSet = (): PayloadSet => ({
  kind: 'list',
  list: [],
  from: 0,
  to: 100,
  step: 1,
  pad: 0,
  urlEncode: false,
  prefix: '',
  suffix: '',
});

const defaultTemplate = (): FuzzTemplate => ({
  method: 'GET',
  url: '',
  headers: [],
  params: [],
  bodyType: 'json',
  body: '',
});

type RunPhase = 'idle' | 'running' | 'done' | 'stopped';

interface FuzzerState {
  template: FuzzTemplate;
  attackType: AttackType;
  variables: FuzzVariable[];
  config: Pick<FuzzConfig, 'matchRules' | 'concurrency' | 'delayMs'>;

  estimate: number | null;
  estimateError: string | null;

  runId: string | null;
  runName: string;
  phase: RunPhase;
  savedToDb: boolean;
  total: number;
  rows: FuzzRow[];
  startError: string | null;

  savedRuns: FuzzRunMeta[];

  setTemplate: (patch: Partial<FuzzTemplate>) => void;
  setAttackType: (t: AttackType) => void;
  setVariableSet: (name: string, patch: Partial<PayloadSet>) => void;
  setConfig: (patch: Partial<Pick<FuzzConfig, 'matchRules' | 'concurrency' | 'delayMs'>>) => void;
  loadFromTraffic: (item: TrafficItem) => void;

  buildConfig: () => FuzzConfig;
  refreshEstimate: () => Promise<void>;
  start: (save: boolean) => Promise<void>;
  stop: () => Promise<void>;
  saveRun: (name: string) => Promise<void>;
  fetchResultDetail: (idx: number) => Promise<FuzzResult | null>;

  fetchSavedRuns: () => Promise<void>;
  openSavedRun: (runId: string) => Promise<void>;
  removeSavedRun: (runId: string) => Promise<void>;

  /** Registers event listeners and returns a disposer; call it once per mount and dispose on unmount. */
  initListeners: () => Promise<() => void>;
}

/** Variable list for a template: one entry per unique {{name}}, reusing existing payload sets. */
const reconcileVariables = (template: FuzzTemplate, previous: FuzzVariable[]): FuzzVariable[] => {
  const byName = new Map(previous.map((v) => [v.name, v]));
  return parseVariables(template).map((name) => byName.get(name) ?? { name, set: emptyPayloadSet() });
};

export const useFuzzerStore = create<FuzzerState>((set, get) => ({
  template: defaultTemplate(),
  attackType: 'sniper',
  variables: [],
  config: { matchRules: [], concurrency: 10, delayMs: 0 },

  estimate: null,
  estimateError: null,

  runId: null,
  runName: 'Fuzz run',
  phase: 'idle',
  savedToDb: false,
  total: 0,
  rows: [],
  startError: null,

  savedRuns: [],

  setTemplate: (patch) => {
    const template = { ...get().template, ...patch };
    set({ template, variables: reconcileVariables(template, get().variables) });
    get().refreshEstimate();
  },

  setAttackType: (attackType) => {
    set({ attackType });
    get().refreshEstimate();
  },

  setVariableSet: (name, patch) => {
    set({ variables: get().variables.map((v) => (v.name === name ? { ...v, set: { ...v.set, ...patch } } : v)) });
    get().refreshEstimate();
  },

  setConfig: (patch) => set({ config: { ...get().config, ...patch } }),

  loadFromTraffic: (item) => {
    const template: FuzzTemplate = {
      method: item.method,
      url: item.url,
      headers: (item.requestHeaders || []).map((h, i) => ({ id: `h-${i}`, key: h.key, value: h.value, enabled: true })),
      params: [],
      bodyType: 'raw',
      body: tryPrettifyJson(item.requestBody || ''),
    };
    set({ template, variables: reconcileVariables(template, get().variables) });
    get().refreshEstimate();
  },

  buildConfig: () => {
    const { template, attackType, variables, config } = get();
    return { template, attackType, variables, ...config };
  },

  refreshEstimate: async () => {
    if (!isTauriAvailable()) return;
    try {
      const estimate = await countFuzzRequests(get().buildConfig());
      set({ estimate, estimateError: null });
    } catch (err) {
      set({ estimate: null, estimateError: String(err) });
    }
  },

  start: async (save) => {
    set({ startError: null });
    try {
      const runId = await startFuzz(get().runName.trim() || 'Fuzz run', get().buildConfig(), save);
      set({
        runId,
        phase: 'running',
        savedToDb: save,
        rows: [],
        total: get().estimate ?? 0,
        startError: null,
      });
    } catch (err) {
      set({ startError: String(err) });
    }
  },

  stop: async () => {
    try {
      await stopFuzz();
    } catch (err) {
      console.error('Failed to stop fuzz run:', err);
    }
  },

  saveRun: async (name) => {
    await saveCurrentFuzz(name.trim() || 'Fuzz run');
    set({ savedToDb: true });
    get().fetchSavedRuns();
  },

  fetchResultDetail: (idx) => {
    const runId = get().runId;
    if (!runId) return Promise.resolve(null);
    return getFuzzResult(runId, idx);
  },

  fetchSavedRuns: async () => {
    if (!isTauriAvailable()) return;
    try {
      set({ savedRuns: await listFuzzRuns() });
    } catch (err) {
      console.error('Failed to list fuzz runs:', err);
    }
  },

  openSavedRun: async (runId) => {
    const [config, results] = await Promise.all([getFuzzRunConfig(runId), getFuzzRunResults(runId)]);
    set({
      template: config.template,
      attackType: config.attackType,
      variables: reconcileVariables(config.template, config.variables ?? []),
      config: { matchRules: config.matchRules, concurrency: config.concurrency, delayMs: config.delayMs },
      runId,
      phase: 'done',
      savedToDb: true,
      total: results.length,
      rows: results.map((r) => ({
        idx: r.idx,
        payloads: r.payloads,
        statusCode: r.statusCode,
        responseSize: r.responseSize,
        durationMs: r.durationMs,
        error: r.error,
        matches: r.matches,
      })),
    });
    get().refreshEstimate();
  },

  removeSavedRun: async (runId) => {
    await deleteFuzzRun(runId);
    get().fetchSavedRuns();
  },

  initListeners: async () => {
    if (!isTauriAvailable()) return () => {};
    const unlisteners = await Promise.all([
      listenFuzzStarted((e) => {
        if (e.runId === get().runId) set({ total: e.total, rows: [] });
      }),
      listenFuzzResult((e) => {
        if (e.runId !== get().runId) return;
        set((state) => ({
          rows: [
            ...state.rows,
            {
              idx: e.idx,
              payloads: e.payloads,
              statusCode: e.statusCode,
              responseSize: e.responseSize,
              durationMs: e.durationMs,
              error: e.error,
              matches: e.matches,
            },
          ],
        }));
      }),
      listenFuzzDone((e) => {
        if (e.runId !== get().runId) return;
        set({ phase: e.stopped ? 'stopped' : 'done' });
        if (get().savedToDb) get().fetchSavedRuns();
      }),
    ]);
    return () => unlisteners.forEach((u) => u());
  },
}));
