use std::ffi::OsStr;
use std::path::PathBuf;

pub async fn pick_save(file_name: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let dialog = filters
        .iter()
        .fold(rfd::AsyncFileDialog::new().set_file_name(file_name), |dialog, (name, extensions)| {
            dialog.add_filter(*name, extensions)
        });
    dialog.save_file().await.map(|file| file.path().to_path_buf())
}

pub async fn pick_open(filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let dialog = filters
        .iter()
        .fold(rfd::AsyncFileDialog::new(), |dialog, (name, extensions)| dialog.add_filter(*name, extensions));
    dialog.pick_file().await.map(|file| file.path().to_path_buf())
}

pub fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

pub fn open_external(path_or_url: impl AsRef<OsStr>) -> Result<(), String> {
    open::that(path_or_url).map_err(|e| e.to_string())
}
