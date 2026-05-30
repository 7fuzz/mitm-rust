export interface EnvVariant { name: string; value: string; }
export interface UILayout { isListOpen: boolean; splitMode: 'vertical' | 'horizontal'; sidebarWidth: number; }
export interface Environment { id: string; name: string; is_active?: boolean; }
export interface GlobalVariableValue { id: string; name: string; value: string; }
export interface GlobalVariable {
  id: string;
  environmentId: string;
  name: string;
  values: GlobalVariableValue[];
  activeIndex: number;
  orderIndex?: number;
}
export type TabType = 'history' | 'intercept' | 'repeater' | 'options' | 'utilities' | 'workspace';

export interface KeyboardShortcuts {
  // Navigation
  goto_history: string;
  goto_intercept: string;
  goto_repeater: string;
  goto_workspace: string;
  goto_utilities: string;
  goto_options: string;
  
  // Quick Actions
  open_variable_switcher: string;
  open_environment_switcher: string;
  
  // Instant Actions (No Prefix)
  instant_variable_switcher: string;
  instant_environment_switcher: string;
  
  // Modifiers
  prefix_key: string;
  cycle_prev: string;
  cycle_next: string;
}

export interface AppPreferences {
  history: boolean;
  repeater: boolean;
  bindings: boolean;
  limits: boolean;
  intercept: boolean;
  simpleMode: boolean;
  autoSave: boolean;
  replacementsAutoSave: boolean;
  theme: 'dark' | 'light';
  shortcuts?: KeyboardShortcuts;
}

export interface RepeaterGroup {
  id: string;
  name: string;
  orderIndex?: number;
}

export interface RepeaterRequest {
  id: string;
  name: string;
  groupId: string | null;
  method: string;
  url: string;
  headers: [string, string][];
  body: string;
  timestamp: number;
  extract?: Record<string, string>;
  hitCount?: number;
  response?: {
    status: number;
    headers: [string, string][];
    body: string;
    time?: number;
  };
}

export type ReplacementCategory = 'URL_REPLACEMENTS' | 'HEADER_REPLACEMENTS' | 'BODY_KEY_REPLACEMENTS' | 'URL_PARAM_REPLACEMENTS' | 'TEXT_REPLACEMENTS';

export interface ReplacementEntry {
  id: string;
  type: ReplacementCategory;
  pattern: string;
  replacement: string;
  description?: string;
  is_active: boolean;
  order_index: number;
}

export interface SyncData {
  history: import('@/types/traffic').Traffic[];
  repeater_groups: RepeaterGroup[];
  repeater_requests: RepeaterRequest[];
  environments: Environment[];
  variables: GlobalVariable[];
  replacements: ReplacementEntry[];
  prefs: any;
  ui_layout: any;
  toolkit_json: string;
  history_limits: any;
}

export interface SyncStatus {
  is_syncing: boolean;
  last_sync: number | null;
  error: string | null;
}

