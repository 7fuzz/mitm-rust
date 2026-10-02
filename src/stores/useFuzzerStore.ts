import { create } from 'zustand';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  AttackType,
  FuzzConfig,
  FuzzPosition,
  FuzzResult,
  FuzzRunMeta,
  FuzzTemplate,
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
import { parsePositions } from '../utils/fuzzerMarkers';
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
  payloadSets: PayloadSet[];
  config: Pick<FuzzConfig, 'matchRules' | 'concurrency' | 'delayMs'>;

  positions: FuzzPosition[];
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
  setPayloadSet: (index: number, patch: Partial<PayloadSet>) => void;
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

  initListeners: () => Promise<void>;
}

let unlisteners: UnlistenFn[] = [];

const recomputePositions = (template: FuzzTemplate, attackType: AttackType, payloadSets: PayloadSet[]) => {
  const positions = parsePositions(template);
  // Cluster bomb needs one set per position; keep the array sized to match
  let nextSets = payloadSets;
  if (attackType === 'clusterbomb') {
    if (positions.length === 0) {
      nextSets = [];
    } else if (payloadSets.length < positions.length) {
      nextSets = [...payloadSets, ...Array.from({ length: positions.length - payloadSets.length }, emptyPayloadSet)];
    } else if (payloadSets.length > positions.length) {
      nextSets = payloadSets.slice(0, positions.length);
    }
  } else {
    nextSets = payloadSets.length > 0 ? [payloadSets[0]] : [emptyPayloadSet()];
  }
  return { positions, payloadSets: nextSets };
};

export const useFuzzerStore = create<FuzzerState>((set, get) => ({
  template: defaultTemplate(),
  attackType: 'sniper',
  payloadSets: [emptyPayloadSet()],
  config: { matchRules: [], concurrency: 10, delayMs: 0 },

  positions: [],
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
    const { positions, payloadSets } = recomputePositions(template, get().attackType, get().payloadSets);
    set({ template, positions, payloadSets });
    get().refreshEstimate();
  },

  setAttackType: (attackType) => {
    const { positions, payloadSets } = recomputePositions(get().template, attackType, get().payloadSets);
    set({ attackType, positions, payloadSets });
    get().refreshEstimate();
  },

  setPayloadSet: (index, patch) => {
    const payloadSets = get().payloadSets.map((s, i) => (i === index ? { ...s, ...patch } : s));
    set({ payloadSets });
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
      body: item.requestBody || '',
    };
    const { positions, payloadSets } = recomputePositions(template, get().attackType, get().payloadSets);
    set({ template, positions, payloadSets });
    get().refreshEstimate();
  },

  buildConfig: () => {
    const { template, attackType, payloadSets, config } = get();
    return { template, attackType, payloadSets, ...config };
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
    const { positions, payloadSets } = recomputePositions(config.template, config.attackType, config.payloadSets);
    set({
      template: config.template,
      attackType: config.attackType,
      payloadSets,
      positions,
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
    if (unlisteners.length > 0 || !isTauriAvailable()) return;
    unlisteners = await Promise.all([
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
  },
}));
