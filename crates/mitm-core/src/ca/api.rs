use std::sync::Arc;
use crate::{ca, MitmCore};

impl MitmCore {
    pub fn get_ca(&self) -> Arc<ca::CA> {
        Arc::new(ca::get_ca(ca::get_default_ca_dir()))
    }

    pub fn ca_info(&self) -> Result<ca::CaInfo, String> {
        let ca_dir = ca::get_default_ca_dir();
        self.get_ca().info(&ca_dir)
    }

    pub fn ca_pem(&self) -> Result<String, String> {
        Ok(self.get_ca().cert_pem.clone())
    }

    pub fn export_ca(&self, path: &std::path::Path, format: ca::CaFormat) -> Result<(), String> {
        self.get_ca().export(path, format)
    }

    /// Tells control-socket subscribers the CA was replaced, so the browser can import the new one.
    fn announce_new_ca(&self, old_fingerprint: Option<String>) {
        #[cfg(unix)]
        if let Ok(mut data) = crate::control::ca_snapshot(self) {
            data["old_fingerprint"] = old_fingerprint.into();
            let _ = self.state.control_events.send(serde_json::json!({ "event": "ca_rotated", "data": data }));
        }
        #[cfg(not(unix))]
        let _ = old_fingerprint;
    }

    pub fn ca_scope(&self) -> ca::CaScope {
        ca::CaScope::load(&ca::get_default_ca_dir())
    }

    /// Changing the scope means a new CA, which clients must trust again.
    pub async fn set_ca_scope(&self, scope: ca::CaScope) -> Result<(), String> {
        scope.save(&ca::get_default_ca_dir())?;
        self.regenerate_ca().await
    }

    /// Keeps the current scope.
    pub async fn regenerate_ca(&self) -> Result<(), String> {
        let old_fingerprint = self.ca_info().ok().map(|info| info.fingerprint);
        let ca_dir = ca::get_default_ca_dir();
        ca::delete_ca(ca_dir.clone());
        let _ = ca::get_ca(ca_dir);
        self.announce_new_ca(old_fingerprint);
        if self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = self.start_all_listeners().await?;
        }
        Ok(())
    }
}
