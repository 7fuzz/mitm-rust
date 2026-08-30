use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_root_ca_pem(app_handle: AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let ca_dir = app_data_dir.join("ca");
    let ca = crate::ca::get_ca(ca_dir);
    Ok(ca.cert_pem)
}

#[tauri::command]
pub fn export_root_ca(app_handle: AppHandle, destination_path: String) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let ca_dir = app_data_dir.join("ca");
    let ca = crate::ca::get_ca(ca_dir);

    let target_path = Path::new(&destination_path);
    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
    }

    fs::write(target_path, &ca.cert_pem)
        .map_err(|e| format!("Failed to write CA certificate to '{}': {}", destination_path, e))?;

    Ok(())
}

#[tauri::command]
pub fn regenerate_root_ca(app_handle: AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let ca_dir = app_data_dir.join("ca");
    crate::ca::delete_ca(ca_dir.clone());
    let ca = crate::ca::get_ca(ca_dir);
    Ok(ca.cert_pem)
}
