export interface EnvVariant { name: string; value: string; }
export interface UILayout { isListOpen: boolean; splitMode: 'vertical' | 'horizontal'; sidebarWidth: number; }
export interface Environment { id: string; name: string; is_active?: boolean; }
export interface GlobalVariableValue { id: string; name: string; value: string; }
export interface GlobalVariable {
  id: string;
  environmentId: string;
  name: string;
  activeIndex: number;
  orderIndex: number;
  values: GlobalVariableValue[];
}

export interface RepeaterGroup {
  id: string;
  name: string;
  orderIndex: number;
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
  repeaterGroups: RepeaterGroup[];
  repeaterRequests: RepeaterRequest[];
  environments: Environment[];
  variables: GlobalVariable[];
  replacements: ReplacementEntry[];
  prefs: any;
  uiLayout: any;
  toolkitJson: string;
  historyLimits: any;
  activeGroupId?: string | null;
  filterConfig: { rules: Array<{ id: string, is_active: boolean, field: string, rule_type: string, mode: 'whitelist' | 'blacklist', pattern: string }> };
}

export interface SyncStatus {
  is_syncing: boolean;
  last_sync: number | null;
  error: string | null;
}

export interface KeyboardShortcuts {
  goto_history: string;
  goto_intercept: string;
  goto_repeater: string;
  goto_workspace: string;
  goto_utilities: string;
  goto_options: string;
  open_variable_switcher: string;
  open_environment_switcher: string;
  instant_variable_switcher: string;
  instant_environment_switcher: string;
  prefix_key: string;
  cycle_prev: string;
  cycle_next: string;
}

export type TabType = 'history' | 'intercept' | 'repeater' | 'options' | 'utilities' | 'workspace' | 'debug';
