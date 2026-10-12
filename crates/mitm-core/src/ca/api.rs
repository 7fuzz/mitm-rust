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

    pub async fn regenerate_ca(&self) -> Result<(), String> {
        let ca_dir = ca::get_default_ca_dir();
        ca::delete_ca(ca_dir.clone());
        let _ = ca::get_ca(ca_dir);
        if self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = self.start_all_listeners().await?;
        }
        Ok(())
    }
}
