use std::time::Duration;

use crate::{db, MitmCore};

const DAYS_PREFERENCE: &str = "ca_rotate_days";
const ROTATED_AT_PREFERENCE: &str = "ca_rotated_at";
const CHECK_EVERY: Duration = Duration::from_secs(3600);
const SECONDS_PER_DAY: i64 = 86_400;

fn due(days: u32, rotated_at: i64, now: i64) -> bool {
    days > 0 && now - rotated_at >= i64::from(days) * SECONDS_PER_DAY
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl MitmCore {
    /// 0 means off.
    pub fn ca_rotation_days(&self) -> u32 {
        db::get_preference(&self.db, DAYS_PREFERENCE).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    pub fn ca_rotated_at(&self) -> Option<i64> {
        db::get_preference(&self.db, ROTATED_AT_PREFERENCE).and_then(|v| v.parse().ok())
    }

    /// Turning it on counts from now when the CA has no recorded rotation yet.
    pub fn set_ca_rotation_days(&self, days: u32) -> Result<(), String> {
        db::set_preference(&self.db, DAYS_PREFERENCE, &days.to_string())?;
        if days > 0 && self.ca_rotated_at().is_none() {
            self.record_rotation()?;
        }
        Ok(())
    }

    pub(super) fn record_rotation(&self) -> Result<(), String> {
        db::set_preference(&self.db, ROTATED_AT_PREFERENCE, &now().to_string())
    }

    /// Replaces the CA if the schedule says it is time; returns whether it did.
    pub async fn rotate_ca_if_due(&self) -> Result<bool, String> {
        let days = self.ca_rotation_days();
        let rotated_at = self.ca_rotated_at().unwrap_or_else(now);
        if !due(days, rotated_at, now()) {
            return Ok(false);
        }
        self.regenerate_ca().await?;
        Ok(true)
    }

    /// Checks at startup and then hourly; replaces a previous task.
    pub async fn start_ca_rotation(&self) {
        self.stop_ca_rotation().await;
        let core = self.clone();
        let task = tokio::spawn(async move {
            loop {
                if let Err(e) = core.rotate_ca_if_due().await {
                    eprintln!("[CA] Rotation failed: {e}");
                }
                tokio::time::sleep(CHECK_EVERY).await;
            }
        });
        *self.state.ca_rotation_task.lock().await = Some(task.abort_handle());
    }

    pub async fn stop_ca_rotation(&self) {
        if let Some(task) = self.state.ca_rotation_task.lock().await.take() {
            task.abort();
        }
    }

    /// Stops the background tasks; for before this core is replaced.
    pub async fn shutdown_services(&self) {
        self.stop_ca_rotation().await;
        #[cfg(unix)]
        self.stop_control_server().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn rotation_is_due_only_when_on_and_old_enough() {
        let day = SECONDS_PER_DAY;
        assert!(!due(0, 0, 1000 * day));
        assert!(!due(30, 100 * day, 129 * day));
        assert!(due(30, 100 * day, 130 * day));
        assert!(due(1, 0, day));
    }

    #[tokio::test]
    async fn due_rotation_replaces_the_ca_keeps_the_scope_and_announces_it() {
        let _env = crate::ca::DEFAULT_CA_DIR_LOCK.lock().await;
        let dir = std::env::temp_dir().join(format!("mitm-rotate-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_DATA_HOME", &dir);
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();
        let (history_tx, _) = mpsc::channel(1);
        let (rewrite_tx, _) = mpsc::channel(1);
        let state = Arc::new(crate::state::AppState::new(db.clone(), history_tx, rewrite_tx));
        let core = MitmCore { state, event_bus: crate::EventBus::default(), db };
        let mut events = core.state.control_events.subscribe();

        core.set_ca_scope(crate::ca::CaScope::domains(&["example.com"]).unwrap()).await.unwrap();
        let first = core.ca_info().unwrap().fingerprint;
        assert!(!core.rotate_ca_if_due().await.unwrap());

        core.set_ca_rotation_days(7).unwrap();
        assert!(!core.rotate_ca_if_due().await.unwrap());
        let eight_days_ago = (now() - 8 * SECONDS_PER_DAY).to_string();
        db::set_preference(&core.db, ROTATED_AT_PREFERENCE, &eight_days_ago).unwrap();
        assert!(core.rotate_ca_if_due().await.unwrap());

        let info = core.ca_info().unwrap();
        assert_ne!(info.fingerprint, first);
        assert_eq!(info.scope.domain_list(), ["example.com"]);
        assert!(core.ca_rotated_at().unwrap() > now() - 60);
        assert!(!core.rotate_ca_if_due().await.unwrap());

        let _scope_change = events.recv().await.unwrap();
        let rotated = events.recv().await.unwrap();
        assert_eq!(rotated["data"]["old_fingerprint"], first);
        assert_eq!(rotated["data"]["fingerprint"], info.fingerprint);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
