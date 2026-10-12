use std::path::PathBuf;
use std::sync::Arc;
use mitm_core::state::{ListenerConfig, ListenerStatus, ProxyMode};
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use crate::features::traffic::TrafficState;
use crate::ui::theme::Tone;
use super::{CertificateState, DataSectionState, ProxyState};

#[derive(Clone)]
pub enum Cmd {
    ToggleProxy,
    SetProxyMode(ProxyMode),
    SaveListeners(Vec<ListenerConfig>),
    SetListenerEnabled { id: u32, enabled: bool },
    ExportCa(mitm_core::CaFormat),
    CopyCaPem,
    OpenCaFolder,
    RegenerateCa,
    ClearData(mitm_core::DataCategory),
    RefreshDataCounts,
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Settings(cmd)
    }
}

/// Result of a proxy job that ran off the UI thread, with the listener statuses read after it.
pub enum ProxyChange {
    Mode(ProxyMode, Result<Option<Vec<String>>, String>),
    Listeners(Result<Option<Vec<String>>, String>),
    CaRegenerated(Result<(), String>),
}

pub type ProxyJob = (ProxyChange, Vec<ListenerStatus>);
pub type CaExport = Option<Result<PathBuf, String>>;

/// The parts of app state the Settings commands touch.
pub struct Settings<'a> {
    pub proxy: &'a mut ProxyState,
    pub cert: &'a mut CertificateState,
    pub data: &'a mut DataSectionState,
    pub traffic: &'a mut TrafficState,
}

pub fn handle(s: Settings, cx: &Cx, cmd: Cmd) {
    let core = Arc::clone(cx.core);
    match cmd {
        Cmd::ToggleProxy | Cmd::SetProxyMode(_) | Cmd::SaveListeners(_) | Cmd::SetListenerEnabled { .. } | Cmd::RegenerateCa
            if s.proxy.busy() => {}
        Cmd::ToggleProxy => {
            let mode = if s.proxy.mode.is_enabled() { ProxyMode::Off } else { ProxyMode::On };
            spawn_proxy_job(s.proxy, cx, async move { ProxyChange::Mode(mode, core.set_proxy_mode(mode).await) });
        }
        Cmd::SetProxyMode(mode) => {
            spawn_proxy_job(s.proxy, cx, async move { ProxyChange::Mode(mode, core.set_proxy_mode(mode).await) });
        }
        Cmd::SaveListeners(listeners) => {
            let was_running = s.proxy.running;
            spawn_proxy_job(s.proxy, cx, async move {
                // Stopped first so set_listener_configs doesn't restart them itself; the restart below reports what bound.
                core.stop_all_listeners().await;
                let result = match core.set_listener_configs(listeners).await {
                    Ok(()) if was_running => core.start_all_listeners().await.map(Some),
                    Ok(()) => Ok(None),
                    Err(e) => Err(e),
                };
                ProxyChange::Listeners(result)
            });
        }
        Cmd::SetListenerEnabled { id, enabled } => {
            spawn_proxy_job(s.proxy, cx, async move {
                ProxyChange::Listeners(core.set_listener_enabled(id, enabled).await.map(|_| None))
            });
        }
        Cmd::RegenerateCa => {
            s.cert.regenerating = true;
            spawn_proxy_job(s.proxy, cx, async move { ProxyChange::CaRegenerated(core.regenerate_ca().await) });
        }
        Cmd::ExportCa(format) => {
            let (file_name, filter_name, ext) = match format {
                mitm_core::CaFormat::Pem => ("mitm-ca.crt", "PEM certificate", "crt"),
                mitm_core::CaFormat::Der => ("mitm-ca.cer", "DER certificate", "cer"),
            };
            s.cert.exports.spawn(cx.rt, cx.ctx, async move {
                let path = crate::util::files::pick_save(file_name, &[(filter_name, &[ext])]).await?;
                Some(core.export_ca(&path, format).map(|_| path))
            });
        }
        Cmd::CopyCaPem => {
            if let Some(pem) = cx.report(cx.core.ca_pem()) {
                cx.ctx.copy_text(pem);
            }
        }
        Cmd::OpenCaFolder => {
            if let Some(info) = cx.report(cx.core.ca_info()) {
                cx.report(crate::util::files::open_external(&info.ca_dir));
            }
        }
        Cmd::ClearData(category) => match cx.core.clear_data_category(category) {
            Ok(()) => {
                if category == mitm_core::DataCategory::Traffic {
                    s.traffic.clear();
                }
                s.data.counts = cx.core.data_counts().ok();
                s.data.status = Some((format!("{} cleared", category.display_name()), Tone::Success));
            }
            Err(e) => s.data.status = Some((e, Tone::Danger)),
        },
        Cmd::RefreshDataCounts => s.data.counts = cx.core.data_counts().ok(),
    }
}

pub fn poll(s: Settings, cx: &Cx) {
    let finished: Vec<_> = s.proxy.jobs.drain().collect();
    for (change, statuses) in finished {
        apply_proxy_change(s.proxy, s.cert, cx, change, &statuses);
    }
    for result in s.cert.exports.drain().flatten() {
        s.cert.status = Some(match result {
            Ok(path) => (format!("Exported to {}", path.display()), Tone::Success),
            Err(e) => (e, Tone::Danger),
        });
    }
}

fn spawn_proxy_job(proxy: &mut ProxyState, cx: &Cx, job: impl std::future::Future<Output = ProxyChange> + Send + 'static) {
    proxy.pending += 1;
    let core = Arc::clone(cx.core);
    proxy.jobs.spawn(cx.rt, cx.ctx, async move {
        let change = job.await;
        (change, core.listener_status().await)
    });
}

fn apply_proxy_change(proxy: &mut ProxyState, cert: &mut CertificateState, cx: &Cx, change: ProxyChange, statuses: &[ListenerStatus]) {
    proxy.pending = proxy.pending.saturating_sub(1);
    match change {
        ProxyChange::Mode(mode, Ok(started)) => {
            proxy.mode = mode;
            match started {
                Some(started) => proxy.set_started(Ok(started)),
                None => proxy.running &= mode.is_enabled(),
            }
        }
        ProxyChange::Listeners(Ok(Some(started))) => proxy.set_started(Ok(started)),
        ProxyChange::Listeners(Ok(None)) => {}
        ProxyChange::Mode(_, Err(e)) | ProxyChange::Listeners(Err(e)) => proxy.error = Some(e),
        ProxyChange::CaRegenerated(result) => {
            cert.regenerating = false;
            cert.info = cx.core.ca_info().ok();
            cert.status = Some(match result {
                Ok(()) => ("CA certificate regenerated".to_string(), Tone::Success),
                Err(e) => (e, Tone::Danger),
            });
        }
    }
    proxy.apply_statuses(statuses);
}
