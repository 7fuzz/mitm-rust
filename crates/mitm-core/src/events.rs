use serde::{Deserialize, Serialize};
use crate::proxy::intercept::PendingFlowPayload;
use crate::proxy::history::TrafficCapturedEvent;
use crate::webhook::types::WebhookDeliveryCapturedEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgressPayload {
    pub step: usize,
    pub total: usize,
    pub name: String,
    pub status: String,
    pub is_complete: bool,
    pub has_error: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CoreEvent {
    TrafficCaptured(TrafficCapturedEvent),
    TrafficUpdated {
        id: i64,
        status: u16,
        size: u64,
        duration_ms: u64,
    },
    TrafficCleared,
    /// Stored flows of a container (or one of its runs) were deleted; loaded lists should reload.
    ContainerFlowsDeleted,

    InterceptPrompt(PendingFlowPayload),
    InterceptResolved { flow_id: String },

    FuzzResult {
        run_id: i64,
        row: crate::fuzzer::FuzzRow,
    },
    FuzzFinished {
        run_id: i64,
        completed: usize,
        total: usize,
        stopped: bool,
    },

    WsFrameReceived {
        session_id: String,
        frame: serde_json::Value,
    },

    WebhookDelivery(WebhookDeliveryCapturedEvent),

    DbMigrationProgress(MigrationProgressPayload),
}

#[derive(Clone)]
pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<CoreEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    pub fn emit(&self, event: CoreEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<CoreEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(4096)
    }
}
