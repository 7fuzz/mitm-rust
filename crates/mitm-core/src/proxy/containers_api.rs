use crate::proxy::containers::ContainerInfo;
use crate::{db, events::CoreEvent, MitmCore};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub run_id: i64,
    pub flows: u64,
}

/// A container as the Containers tab shows it: registered by the fork, or only seen in traffic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRow {
    /// The proxy username; for an unregistered row, the label the flows carry.
    pub id: String,
    pub name: String,
    pub registered: bool,
    pub run_id: i64,
    pub flows: u64,
    pub last_seen: Option<i64>,
}

impl MitmCore {
    /// Registers a container, or renames it; stored flows follow the new name.
    pub fn register_container(&self, id: &str, name: &str) -> Result<(), String> {
        if id.trim().is_empty() || name.trim().is_empty() {
            return Err("Container id and name are required".to_string());
        }
        if let Some(old) = self.state.containers.register(id, name) {
            db::relabel_container_flows(&self.db, &old, name)?;
            self.event_bus.emit(CoreEvent::ContainerFlowsDeleted);
        }
        self.event_bus.emit(CoreEvent::ContainersChanged);
        self.state.containers.save(&self.db)
    }

    pub fn remove_container(&self, id: &str) -> Result<(), String> {
        self.state.containers.remove(id);
        self.event_bus.emit(CoreEvent::ContainersChanged);
        self.state.containers.save(&self.db)
    }

    pub fn start_container_run(&self, id: &str) -> Result<i64, String> {
        let run_id = self.state.containers.start_run(id);
        self.state.containers.save(&self.db)?;
        self.event_bus.emit(CoreEvent::ContainersChanged);
        Ok(run_id)
    }

    /// All of a container's flows, or only one run's; returns how many history rows went.
    pub fn delete_container_flows(&self, id: &str, run_id: Option<i64>) -> Result<usize, String> {
        let label = self.state.containers.tag(id).label;
        let removed = db::delete_container_flows(&self.db, &label, run_id)?;
        self.event_bus.emit(CoreEvent::ContainerFlowsDeleted);
        Ok(removed)
    }

    pub fn list_containers(&self) -> Vec<ContainerSummary> {
        self.state
            .containers
            .list()
            .into_iter()
            .map(|(id, ContainerInfo { name, run_id })| {
                let flows = db::count_container_flows(&self.db, &name).unwrap_or(0);
                ContainerSummary { id, name, run_id, flows }
            })
            .collect()
    }

    /// Registered containers first, then labels that flows carry without a registration (raw ids, or names of removed containers).
    pub fn container_overview(&self) -> Vec<ContainerRow> {
        let mut activity: std::collections::HashMap<String, db::ContainerActivity> =
            db::container_activity(&self.db).into_iter().map(|a| (a.label.clone(), a)).collect();
        let mut rows: Vec<ContainerRow> = self
            .state
            .containers
            .list()
            .into_iter()
            .map(|(id, ContainerInfo { name, run_id })| {
                let seen = activity.remove(&name);
                ContainerRow { id, registered: true, run_id, flows: seen.as_ref().map_or(0, |a| a.flows), last_seen: seen.and_then(|a| a.last_seen), name }
            })
            .collect();
        let mut unregistered: Vec<ContainerRow> = activity
            .into_values()
            .map(|a| ContainerRow { id: a.label.clone(), name: a.label, registered: false, run_id: 0, flows: a.flows, last_seen: a.last_seen })
            .collect();
        unregistered.sort_by(|a, b| a.name.cmp(&b.name));
        rows.extend(unregistered);
        rows
    }

    pub fn container_proxy_token(&self) -> String {
        self.state.containers.token().to_string()
    }
}
