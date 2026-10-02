use std::time::Duration;

use tauri::{AppHandle, State};

use crate::instance::{ping, request_quit, try_claim, Claim, InstanceState, OtherInstance};
use crate::state::AppState;

#[tauri::command]
pub fn get_instance_conflict(instance: State<'_, InstanceState>) -> Option<OtherInstance> {
    instance.0.lock().unwrap().clone()
}

/// Asks the other instance to quit, waits for its lock, then starts what startup skipped.
#[tauri::command]
pub async fn take_over_instance(
    app_handle: AppHandle,
    instance: State<'_, InstanceState>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !request_quit() && ping().is_some() {
        return Err("The other instance did not respond to the request to close.".to_string());
    }

    let mut claimed = false;
    for _ in 0..50 {
        match try_claim(&app_handle) {
            Claim::Primary | Claim::Unguarded => {
                claimed = true;
                break;
            }
            Claim::Conflict(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    if !claimed {
        return Err("The other instance is still running after 5 seconds.".to_string());
    }

    *instance.0.lock().unwrap() = None;
    if state.is_proxy_active() {
        crate::commands::proxy_cmd::start_all_listeners(&app_handle, &state).await;
    }
    Ok(())
}
