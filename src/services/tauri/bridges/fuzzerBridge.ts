import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { HeaderItem, ParamItem } from "./repeaterBridge";
import { isTauriAvailable } from "../ipc";

export type AttackType = "sniper" | "clusterbomb";
export type PayloadKind = "list" | "numbers";
export type MatchKind = "contains" | "regex";

export interface FuzzTemplate {
  method: string;
  url: string;
  headers: HeaderItem[];
  params: ParamItem[];
  bodyType: string;
  body?: string;
}

export interface PayloadSet {
  kind: PayloadKind;
  list: string[];
  from: number;
  to: number;
  step: number;
  pad: number;
  urlEncode: boolean;
  prefix: string;
  suffix: string;
}

export interface MatchRule {
  name: string;
  kind: MatchKind;
  pattern: string;
}

export interface FuzzConfig {
  template: FuzzTemplate;
  attackType: AttackType;
  payloadSets: PayloadSet[];
  matchRules: MatchRule[];
  concurrency: number;
  delayMs: number;
}

export interface FuzzPosition {
  index: number;
  field: string;
  base: string;
}

export interface MatchResult {
  name: string;
  value: string;
}

export interface FuzzResult {
  idx: number;
  payloads: string[];
  method: string;
  url: string;
  requestHeaders: HeaderItem[];
  requestBody?: string;
  statusCode: number;
  responseHeaders: HeaderItem[];
  responseBody?: string;
  responseSize: number;
  durationMs: number;
  error?: string;
  matches: MatchResult[];
}

export interface FuzzRunMeta {
  id: string;
  name: string;
  attackType: AttackType;
  total: number;
  createdAtMs: number;
}

export interface FuzzResultEvent {
  runId: string;
  idx: number;
  payloads: string[];
  statusCode: number;
  responseSize: number;
  durationMs: number;
  error?: string;
  matches: MatchResult[];
}

export interface FuzzStartedEvent {
  runId: string;
  total: number;
}

export interface FuzzDoneEvent {
  runId: string;
  completed: number;
  total: number;
  stopped: boolean;
}

export const countFuzzRequests = (config: FuzzConfig): Promise<number> =>
  invoke<number>("count_fuzz_requests", { config });

export const startFuzz = (name: string, config: FuzzConfig, save: boolean): Promise<string> =>
  invoke<string>("start_fuzz", { name, config, save });

export const stopFuzz = (): Promise<void> => invoke<void>("stop_fuzz");

export const getFuzzResult = (runId: string, idx: number): Promise<FuzzResult | null> =>
  invoke<FuzzResult | null>("get_fuzz_result", { runId, idx });

export const saveCurrentFuzz = (name: string): Promise<string> =>
  invoke<string>("save_current_fuzz", { name });

export const listFuzzRuns = (): Promise<FuzzRunMeta[]> => invoke<FuzzRunMeta[]>("list_fuzz_runs");

export const getFuzzRunConfig = (runId: string): Promise<FuzzConfig> =>
  invoke<FuzzConfig>("get_fuzz_run_config", { runId });

export const getFuzzRunResults = (runId: string): Promise<FuzzResult[]> =>
  invoke<FuzzResult[]>("get_fuzz_run_results", { runId });

export const deleteFuzzRun = (runId: string): Promise<void> =>
  invoke<void>("delete_fuzz_run", { runId });

export const listenFuzzStarted = async (cb: (e: FuzzStartedEvent) => void): Promise<UnlistenFn> =>
  isTauriAvailable() ? listen<FuzzStartedEvent>("fuzz-started", (e) => cb(e.payload)) : () => {};

export const listenFuzzResult = async (cb: (e: FuzzResultEvent) => void): Promise<UnlistenFn> =>
  isTauriAvailable() ? listen<FuzzResultEvent>("fuzz-result", (e) => cb(e.payload)) : () => {};

export const listenFuzzDone = async (cb: (e: FuzzDoneEvent) => void): Promise<UnlistenFn> =>
  isTauriAvailable() ? listen<FuzzDoneEvent>("fuzz-done", (e) => cb(e.payload)) : () => {};
