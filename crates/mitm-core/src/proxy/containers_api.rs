use crate::proxy::containers::ContainerInfo;
use crate::{db, events::CoreEvent, MitmCore};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub run_id: i64,
    pub flows: u64,
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
        self.state.containers.save(&self.db)
    }

    pub fn remove_container(&self, id: &str) -> Result<(), String> {
        self.state.containers.remove(id);
        self.state.containers.save(&self.db)
    }

    pub fn start_container_run(&self, id: &str) -> Result<i64, String> {
        let run_id = self.state.containers.start_run(id);
        self.state.containers.save(&self.db)?;
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

    pub fn container_proxy_token(&self) -> String {
        self.state.containers.token().to_string()
    }
}
