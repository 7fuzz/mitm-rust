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

pub struct Reply<T> {
    tx: Sender<T>,
    ctx: egui::Context,
}

impl<T> Clone for Reply<T> {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone(), ctx: self.ctx.clone() }
    }
}

impl<T> Reply<T> {
    pub fn send(&self, value: T) {
        let _ = self.tx.send(value);
        self.ctx.request_repaint();
    }
}

/// One blocking job on its own thread whose result only matters until a newer one starts:
/// starting again drops the pending result.
pub struct Latest<T> {
    rx: Option<std::sync::mpsc::Receiver<T>>,
}

impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self { rx: None }
    }
}

impl<T: Send + 'static> Latest<T> {
    pub fn start(&mut self, ctx: &egui::Context, job: impl FnOnce() -> T + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if tx.send(job()).is_ok() {
                ctx.request_repaint();
            }
        });
        self.rx = Some(rx);
    }

    pub fn take(&mut self) -> Option<T> {
        let value = self.rx.as_ref()?.try_recv().ok()?;
        self.rx = None;
        Some(value)
    }

    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }
}
