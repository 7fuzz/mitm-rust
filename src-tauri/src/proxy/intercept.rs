use tokio::sync::broadcast;
use crate::state::TrafficCapturedEvent;

#[derive(Debug, Clone, Default)]
pub struct InterceptConfig {
    pub intercept_requests: bool,
    pub intercept_responses: bool,
}

pub struct FlowController {
    pub broadcast_tx: broadcast::Sender<TrafficCapturedEvent>,
    pub config: InterceptConfig,
}

impl FlowController {
    pub fn new(broadcast_tx: broadcast::Sender<TrafficCapturedEvent>) -> Self {
        Self {
            broadcast_tx,
            config: InterceptConfig::default(),
        }
    }

    pub fn broadcast_traffic(&self, event: TrafficCapturedEvent) {
        let _ = self.broadcast_tx.send(event);
    }
}
