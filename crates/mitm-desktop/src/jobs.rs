use std::future::Future;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use tokio::runtime::Runtime;
use tokio::task::AbortHandle;

pub struct Jobs<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
}

impl<T> Default for Jobs<T> {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

impl<T: Send + 'static> Jobs<T> {
    pub fn spawn<F>(&self, rt: &Runtime, ctx: &egui::Context, job: F) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
    {
        let reply = self.reply(ctx);
        rt.spawn(async move { reply.send(job.await) }).abort_handle()
    }

    pub fn reply(&self, ctx: &egui::Context) -> Reply<T> {
        Reply { tx: self.tx.clone(), ctx: ctx.clone() }
    }

    pub fn drain(&self) -> impl Iterator<Item = T> + '_ {
        self.rx.try_iter()
    }
}

#[derive(Clone)]
pub struct Reply<T> {
    tx: Sender<T>,
    ctx: egui::Context,
}

impl<T> Reply<T> {
    pub fn send(&self, value: T) {
        let _ = self.tx.send(value);
        self.ctx.request_repaint();
    }
}
