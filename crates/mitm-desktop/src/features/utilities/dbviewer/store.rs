use dbviewer_core::{Profile, SavedQuery};
use mitm_core::db::dbviewer_db::{self, StoredRow};
use mitm_core::db::Db;
use serde::{de::DeserializeOwned, Serialize};

fn parse<T: DeserializeOwned>(rows: Vec<String>) -> Vec<T> {
    rows.iter().filter_map(|data| serde_json::from_str(data).ok()).collect()
}

fn rows<T: Serialize>(items: &[T], key: impl Fn(&T) -> Vec<String>) -> Result<Vec<StoredRow>, String> {
    items
        .iter()
        .map(|item| Ok(StoredRow { key: key(item), data: serde_json::to_string(item).map_err(|e| e.to_string())? }))
        .collect()
}

pub fn load_profiles(db: &Db) -> Result<Vec<Profile>, String> {
    dbviewer_db::load_connections(db).map(parse)
}

pub fn save_profiles(db: &Db, profiles: &[Profile]) -> Result<(), String> {
    dbviewer_db::save_connections(db, &rows(profiles, |p| vec![p.name.clone()])?)
}

pub fn load_queries(db: &Db) -> Result<Vec<SavedQuery>, String> {
    dbviewer_db::load_queries(db).map(parse)
}

pub fn save_queries(db: &Db, queries: &[SavedQuery]) -> Result<(), String> {
    dbviewer_db::save_queries(db, &rows(queries, |q| vec![q.connection.clone(), q.name.clone()])?)
}
