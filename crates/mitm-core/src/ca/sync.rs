use std::path::Path;

const SYNCED_FOLDERS: [&str; 6] = ["dropbox", "google drive", "onedrive", "nextcloud", "mobile documents", "icloud drive"];

/// A reason the CA key probably gets copied somewhere else, if there is one.
pub fn sync_warning(ca_dir: &Path) -> Option<String> {
    for ancestor in ca_dir.ancestors() {
        let name = ancestor.file_name()?.to_string_lossy().to_lowercase();
        if SYNCED_FOLDERS.contains(&name.as_str()) {
            return Some(format!("The CA folder is inside {}, which syncs its files", ancestor.display()));
        }
        if ancestor.join(".git").exists() {
            return Some(format!("The CA folder is inside the git repository {}", ancestor.display()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synced_folders_and_repositories_are_flagged() {
        let root = std::env::temp_dir().join(format!("mitm-sync-{}", uuid::Uuid::new_v4()));
        let synced = root.join("Dropbox").join("dotfiles").join("mitm").join("ca");
        let repo = root.join("dotfiles");
        std::fs::create_dir_all(&synced).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("mitm").join("ca")).unwrap();
        let plain = root.join("plain").join("ca");
        std::fs::create_dir_all(&plain).unwrap();

        assert!(sync_warning(&synced).unwrap().contains("Dropbox"));
        assert!(sync_warning(&repo.join("mitm").join("ca")).unwrap().contains("git repository"));
        assert_eq!(sync_warning(&plain), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
