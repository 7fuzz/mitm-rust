use crossbeam_channel::{Receiver, Sender};
use mitm_core::repeater::execute::SentRequest;
use mitm_core::repeater::RepeaterTab;
use super::RepeaterState;

type SendResult = (String, Result<SentRequest, String>);

pub struct RepeaterWorker {
    tx: Sender<SendResult>,
    rx: Receiver<SendResult>,
}

impl Default for RepeaterWorker {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

impl RepeaterWorker {
    pub fn send(&self, rt: &tokio::runtime::Runtime, ctx: &eframe::egui::Context, tab_id: String, request: RepeaterTab) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rt.spawn(async move {
            let result = mitm_core::repeater::execute::send_tab_request(&request).await;
            let _ = tx.send((tab_id, result));
            ctx.request_repaint();
        });
    }

    pub fn poll(&self, state: &mut RepeaterState) {
        while let Ok((tab_id, result)) = self.rx.try_recv() {
            state.apply_result(&tab_id, result);
        }
    }
}
