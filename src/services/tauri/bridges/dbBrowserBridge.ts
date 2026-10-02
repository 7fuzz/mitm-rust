import { invoke } from "@tauri-apps/api/core";

export type DbCell = string | number | null;

export interface DbColumn {
  name: string;
  declType: string;
  primaryKey: boolean;
  notNull: boolean;
}

export interface DbTable {
  name: string;
  rowCount: number;
  columns: DbColumn[];
  /** CREATE statement as stored in sqlite_master */
  sql: string;
}

export interface DbRows {
  columns: string[];
  rows: DbCell[][];
  /** rowid per row, absent for WITHOUT ROWID tables */
  rowids: number[] | null;
  total: number;
}

export interface DbQueryResult {
  columns: string[];
  rows: DbCell[][];
  truncated: boolean;
  elapsedMs: number;
}

export interface DbTableQuery {
  table: string;
  offset: number;
  limit: number;
  orderBy?: string | null;
  orderDesc?: boolean;
  search?: string;
}

export const listDbTables = async (): Promise<DbTable[]> => {
  return await invoke<DbTable[]>("list_db_tables");
};

export const queryDbTable = async (q: DbTableQuery): Promise<DbRows> => {
  return await invoke<DbRows>("query_db_table", {
    table: q.table,
    offset: q.offset,
    limit: q.limit,
    orderBy: q.orderBy ?? null,
    orderDesc: q.orderDesc ?? false,
    search: q.search || null,
  });
};

/** Full (untruncated) values of one row */
export const getDbRow = async (table: string, rowid: number): Promise<DbCell[]> => {
  return await invoke<DbCell[]>("get_db_row", { table, rowid });
};

/** Runs one statement on a read-only connection */
export const runDbQuery = async (sql: string): Promise<DbQueryResult> => {
  return await invoke<DbQueryResult>("run_db_query", { sql });
};
