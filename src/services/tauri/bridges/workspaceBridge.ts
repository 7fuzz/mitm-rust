import { invoke } from "@tauri-apps/api/core";
import type { HeaderItem, ParamItem, ExtractRuleItem } from "./repeaterBridge";

export interface Workspace {
  id: string;
  name: string;
  description?: string;
  activeEnvironmentId?: string;
  createdAtMs: number;
  updatedAtMs: number;
}

export interface VariableVariant {
  name: string;
  value: string;
}

export interface EnvironmentVariable {
  key: string;
  value: string;
  enabled: boolean;
  type: "default" | "secret";
  activeIndex?: number;
  variants?: VariableVariant[];
}

export interface Environment {
  id: string;
  workspaceId: string;
  name: string;
  isActive: boolean;
  variables: EnvironmentVariable[];
  createdAtMs: number;
  updatedAtMs: number;
}

export interface ImportSummary {
  workspaceId: string;
  workspaceName: string;
  environmentsImported: number;
  collectionsImported: number;
  requestsImported: number;
}

export interface Collection {
  id: string;
  workspaceId: string;
  parentId?: string;
  name: string;
  description?: string;
  orderIndex: number;
  createdAtMs: number;
  updatedAtMs: number;
}

export interface RequestItem {
  id: string;
  collectionId: string;
  name: string;
  method: string;
  url: string;
  headers: HeaderItem[];
  params: ParamItem[];
  bodyType: string;
  bodyJson?: string;
  bodyRaw?: string;
  bodyFormData?: string;
  bodyUrlencoded?: string;
  extractRules: ExtractRuleItem[];
  description?: string;
  orderIndex: number;
  createdAtMs: number;
  updatedAtMs: number;
}

export interface CollectionTreeItem {
  id: string;
  workspaceId: string;
  parentId?: string;
  name: string;
  description?: string;
  orderIndex: number;
  createdAtMs: number;
  updatedAtMs: number;
  children: CollectionTreeItem[];
  requests: RequestItem[];
}

export interface ExecutionResult {
  historyId: number;
  requestId: string;
  statusCode: number;
  statusText: string;
  responseHeaders: HeaderItem[];
  responseBody: string;
  durationMs: number;
  responseSize: number;
}

export interface RequestHistoryItem {
  id: number;
  requestId: string;
  method: string;
  url: string;
  requestHeaders: HeaderItem[];
  requestBody?: string;
  statusCode: number;
  statusText?: string;
  responseHeaders: HeaderItem[];
  responseBody?: string;
  durationMs: number;
  executedAtMs: number;
}

export interface RequestPreview {
  method: string;
  url: string;
  host: string;
  path: string;
  headers: [string, string][];
  body?: string;
  bodyType: string;
  fullUrlRequest: string;
  fullRequest: string;
  curlCommand: string;
}

export const getWorkspaces = async (): Promise<Workspace[]> => {
  return await invoke<Workspace[]>("get_workspaces");
};

export const createWorkspace = async (
  name: string,
  description?: string
): Promise<Workspace> => {
  return await invoke<Workspace>("create_workspace", { name, description: description || null });
};

export const updateWorkspace = async (workspace: Workspace): Promise<void> => {
  return await invoke<void>("update_workspace", { workspace });
};

export const deleteWorkspace = async (id: string): Promise<void> => {
  return await invoke<void>("delete_workspace", { id });
};

export const setActiveWorkspace = async (id: string): Promise<void> => {
  return await invoke<void>("set_active_workspace", { id });
};

export const importWorkspaceJson = async (
  jsonContent: string,
  targetWorkspaceId?: string,
  customWorkspaceName?: string
): Promise<ImportSummary> => {
  return await invoke<ImportSummary>("import_workspace_json", {
    jsonContent,
    targetWorkspaceId: targetWorkspaceId || null,
    customWorkspaceName: customWorkspaceName || null,
  });
};

export const exportWorkspaceJson = async (
  workspaceId: string
): Promise<string> => {
  return await invoke<string>("export_workspace_json", { workspaceId });
};

export const exportWorkspaceFile = async (
  workspaceId: string,
  destinationPath: string
): Promise<void> => {
  return await invoke<void>("export_workspace_file", {
    workspaceId,
    destinationPath,
  });
};

/** Reserved workspace holding the Repeater's tabs and environments; hidden from the workspace list. */
export const REPEATER_WORKSPACE_ID = '00000000-0000-4000-8000-000000000001';

export const getWorkspaceEnvironments = async (
  workspaceId: string
): Promise<Environment[]> => {
  return await invoke<Environment[]>("get_workspace_environments", { workspaceId });
};

export const saveWorkspaceEnvironment = async (
  environment: Environment
): Promise<void> => {
  return await invoke<void>("save_workspace_environment", { environment });
};

export const deleteWorkspaceEnvironment = async (id: string): Promise<void> => {
  return await invoke<void>("delete_workspace_environment", { id });
};

export const getCollections = async (
  workspaceId: string
): Promise<CollectionTreeItem[]> => {
  return await invoke<CollectionTreeItem[]>("get_collections", { workspaceId });
};

export const createCollection = async (
  workspaceId: string,
  parentId: string | null,
  name: string
): Promise<Collection> => {
  return await invoke<Collection>("create_collection", { workspaceId, parentId, name });
};

export const updateCollection = async (collection: Collection): Promise<void> => {
  return await invoke<void>("update_collection", { collection });
};

export const deleteCollection = async (id: string): Promise<void> => {
  return await invoke<void>("delete_collection", { id });
};

export const moveCollection = async (
  collectionId: string,
  targetParentId: string | null
): Promise<void> => {
  return await invoke<void>("move_collection", { collectionId, targetParentId });
};

export const duplicateCollection = async (collectionId: string): Promise<string> => {
  return await invoke<string>("duplicate_collection", { collectionId });
};

export const createRequest = async (
  collectionId: string,
  name: string
): Promise<RequestItem> => {
  return await invoke<RequestItem>("create_request", { collectionId, name });
};

export const updateRequest = async (request: RequestItem): Promise<void> => {
  return await invoke<void>("update_request", { request });
};

export const deleteRequest = async (id: string): Promise<void> => {
  return await invoke<void>("delete_request", { id });
};

export const moveRequest = async (
  requestId: string,
  targetCollectionId: string
): Promise<void> => {
  return await invoke<void>("move_request", { requestId, targetCollectionId });
};

export const duplicateRequest = async (requestId: string): Promise<RequestItem> => {
  return await invoke<RequestItem>("duplicate_request", { requestId });
};

export const executeCollectionRequest = async (
  requestId: string
): Promise<ExecutionResult> => {
  return await invoke<ExecutionResult>("execute_collection_request", { requestId });
};

export const getRequestHistories = async (
  requestId: string
): Promise<RequestHistoryItem[]> => {
  return await invoke<RequestHistoryItem[]>("get_request_histories", { requestId });
};

export const clearRequestHistories = async (
  requestId: string
): Promise<void> => {
  return await invoke<void>("clear_request_histories", { requestId });
};

export const readFileAsBase64 = async (
  filePath: string
): Promise<string> => {
  return await invoke<string>("read_file_as_base64", { filePath });
};

export const previewCollectionRequest = async (
  requestId: string
): Promise<RequestPreview> => {
  return await invoke<RequestPreview>("preview_collection_request", { requestId });
};
