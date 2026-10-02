use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::fuzzer::{
    self, execute::run_attack, FuzzConfig, FuzzPosition, FuzzResult, FuzzRunMeta, FuzzTemplate,
};
use crate::state::AppState;

#[tauri::command]
pub fn parse_fuzz_positions(template: FuzzTemplate) -> Vec<FuzzPosition> {
    fuzzer::parse_positions(&template)
}

/// Number of requests this config would send, or an error (e.g. over the cap).
#[tauri::command]
pub fn count_fuzz_requests(config: FuzzConfig) -> Result<u32, String> {
    Ok(fuzzer::generate_combos(&config)?.len() as u32)
}

#[tauri::command]
pub async fn start_fuzz(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    config: FuzzConfig,
    save: bool,
) -> Result<String, String> {
    if state.fuzz_running.load(Ordering::SeqCst) {
        return Err("A fuzz run is already in progress.".to_string());
    }
    // Fail fast on an invalid config before spawning
    fuzzer::generate_combos(&config)?;

    let run_id = format!("fuzz-{}", Uuid::new_v4());
    let state_arc = Arc::new((*state).clone());
    let run_id_spawn = run_id.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_attack(app, (*state_arc).clone(), run_id_spawn, name, config, save).await {
            eprintln!("[Fuzzer] run failed: {}", e);
        }
    });
    Ok(run_id)
}

#[tauri::command]
pub fn stop_fuzz(state: State<'_, AppState>) {
    state.fuzz_cancel.store(true, Ordering::SeqCst);
}

/// Full request/response of one result, from the in-memory run or a saved run.
#[tauri::command]
pub async fn get_fuzz_result(
    state: State<'_, AppState>,
    run_id: String,
    idx: u32,
) -> Result<Option<FuzzResult>, String> {
    {
        let buf = state.fuzz_buffer.read().await;
        if let Some(b) = buf.as_ref() {
            if b.run_id == run_id {
                return Ok(b.results.iter().find(|r| r.idx == idx).cloned());
            }
        }
    }
    let results = fuzzer::get_run_results_db(&state.db_path, &run_id)?;
    Ok(results.into_iter().find(|r| r.idx == idx))
}

/// Persists the current in-memory run (for a temporary run the user chose to keep).
#[tauri::command]
pub async fn save_current_fuzz(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let (run_id, config, mut results) = {
        let buf = state.fuzz_buffer.read().await;
        let b = buf.as_ref().ok_or("No fuzz run to save.")?;
        (b.run_id.clone(), b.config.clone(), b.results.clone())
    };
    results.sort_by_key(|r| r.idx);
    fuzzer::save_run_db(&state.db_path, &run_id, &name, &config, &results)?;
    if let Some(b) = state.fuzz_buffer.write().await.as_mut() {
        b.saved = true;
    }
    Ok(run_id)
}

#[tauri::command]
pub fn list_fuzz_runs(state: State<'_, AppState>) -> Result<Vec<FuzzRunMeta>, String> {
    fuzzer::list_runs_db(&state.db_path)
}

#[tauri::command]
pub fn get_fuzz_run_config(state: State<'_, AppState>, run_id: String) -> Result<FuzzConfig, String> {
    fuzzer::get_run_config_db(&state.db_path, &run_id)
}

#[tauri::command]
pub fn get_fuzz_run_results(state: State<'_, AppState>, run_id: String) -> Result<Vec<FuzzResult>, String> {
    fuzzer::get_run_results_db(&state.db_path, &run_id)
}

#[tauri::command]
pub fn delete_fuzz_run(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    fuzzer::delete_run_db(&state.db_path, &run_id)
}
