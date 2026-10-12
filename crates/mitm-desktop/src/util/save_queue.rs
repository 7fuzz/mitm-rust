use std::collections::HashSet;

const SAVE_DELAY_SECS: f64 = 0.6;
const RETRY_DELAY_SECS: f64 = 5.0;

/// Ids edited since their last save, written once edits pause. A failed save stays queued and is
/// retried later; its error is reported once per failure streak rather than on every retry.
#[derive(Default)]
pub struct SaveQueue {
    dirty: HashSet<String>,
    due_at: f64,
    failing: HashSet<String>,
    errors: Vec<String>,
}

impl SaveQueue {
    pub fn edited(&mut self, id: &str, now: f64) {
        self.dirty.insert(id.to_string());
        self.due_at = now + SAVE_DELAY_SECS;
    }

    pub fn insert(&mut self, id: &str) {
        self.dirty.insert(id.to_string());
    }

    pub fn remove(&mut self, id: &str) {
        self.dirty.remove(id);
        self.failing.remove(id);
    }

    pub fn is_due(&self, now: f64) -> bool {
        !self.dirty.is_empty() && now >= self.due_at
    }

    pub fn take(&mut self) -> HashSet<String> {
        std::mem::take(&mut self.dirty)
    }

    pub fn saved(&mut self, id: &str) {
        self.failing.remove(id);
    }

    pub fn failed(&mut self, id: String, error: String) {
        if self.failing.insert(id.clone()) {
            self.errors.push(error);
        }
        self.dirty.insert(id);
    }

    /// Call after an idle flush, so whatever failed waits before the next attempt.
    pub fn retry_later(&mut self, now: f64) {
        if !self.dirty.is_empty() {
            self.due_at = now + RETRY_DELAY_SECS;
        }
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_edits_to_pause() {
        let mut q = SaveQueue::default();
        q.edited("a", 10.0);
        assert!(!q.is_due(10.3));
        assert!(q.is_due(10.6));
        assert_eq!(q.take().len(), 1);
        assert!(!q.is_due(100.0));
    }

    #[test]
    fn failed_saves_retry_later_and_report_once() {
        let mut q = SaveQueue::default();
        q.edited("a", 0.0);
        q.take();
        q.failed("a".into(), "locked".into());
        q.retry_later(1.0);
        assert!(!q.is_due(2.0));
        assert!(q.is_due(6.0));
        q.take();
        q.failed("a".into(), "locked".into());
        assert_eq!(q.take_errors(), vec!["locked".to_string()]);

        q.take();
        q.saved("a");
        q.failed("a".into(), "locked again".into());
        assert_eq!(q.take_errors().len(), 1);
    }
}
