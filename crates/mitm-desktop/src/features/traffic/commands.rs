use std::path::Path;
use std::sync::Arc;
use mitm_core::proxy::history::{HistoryEntry, HistorySettings};
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use crate::app::jobs::Jobs;
use crate::features::inspector::InspectorState;
use crate::util::format::{CopyFormat, CopyOptions, Exchange};
use super::TrafficState;
use mitm_core::db::Db;

#[derive(Clone)]
pub enum Cmd {
    LoadOlder,
    Select(i64),
    Clear,
    OpenHarExport,
    ExportHar(Vec<i64>),
    Delete(i64),
    Copy(i64, CopyFormat),
    CopyCustom(i64, CopyOptions),
    SetHistoryLimit(HistorySettings),
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Traffic(cmd)
    }
}

#[derive(Default)]
pub struct TrafficJobs {
    har_index: Jobs<Result<Vec<HistoryEntry>, String>>,
    exports: Jobs<Option<Result<String, String>>>,
    history_limits: Jobs<(HistorySettings, Result<(), String>)>,
}

pub fn handle(traffic: &mut TrafficState, inspector: &mut InspectorState, cx: &Cx, cmd: Cmd) {
    let db = cx.db();
    match cmd {
        Cmd::LoadOlder => traffic.load_older(db),
        Cmd::Select(id) => {
            traffic.select(id, db);
            inspector.prefs.open = true;
        }
        Cmd::Clear => {
            if cx.report(cx.core.clear_all_traffic()).is_some() {
                traffic.clear();
            }
        }
        Cmd::OpenHarExport => {
            traffic.har_export.open();
            let db = db.clone();
            traffic.jobs.har_index.spawn(cx.rt, cx.ctx, async move { mitm_core::db::get_history_summaries(&db) });
        }
        Cmd::ExportHar(ids) => {
            let db = db.clone();
            traffic.jobs.exports.spawn(cx.rt, cx.ctx, async move {
                let path = crate::util::files::pick_save("traffic.har", &[("HAR", &["har"])]).await?;
                Some(write_har(&db, &ids, &path).map(|count| {
                    format!("Exported {} requests to {}", count, crate::util::files::file_name(&path))
                }))
            });
        }
        Cmd::Delete(id) => {
            if cx.report(mitm_core::db::delete_history_entry(db, id)).is_some() {
                traffic.remove(id);
            }
        }
        Cmd::Copy(id, format) => {
            if let Some(exchange) = exchange(traffic, db, id) {
                cx.ctx.copy_text(exchange.copy_text(format));
            }
        }
        Cmd::CopyCustom(id, options) => {
            if let Some(exchange) = exchange(traffic, db, id) {
                cx.ctx.copy_text(exchange.custom_text(&options));
            }
        }
        Cmd::SetHistoryLimit(settings) => {
            let core = Arc::clone(cx.core);
            traffic.jobs.history_limits.spawn(cx.rt, cx.ctx, async move {
                let result = core.set_history_settings(settings.clone()).await;
                (settings, result)
            });
        }
    }
}

pub fn poll(traffic: &mut TrafficState, cx: &Cx) {
    let indexes: Vec<_> = traffic.jobs.har_index.drain().collect();
    for result in indexes {
        traffic.har_export.loaded(result);
    }
    for result in traffic.jobs.exports.drain().flatten() {
        cx.notify(result);
    }
    let limits: Vec<_> = traffic.jobs.history_limits.drain().collect();
    for (settings, result) in limits {
        if cx.report(result).is_some() {
            traffic.set_history_limit(settings);
            traffic.load_from_db(cx.db());
        }
    }
}

fn exchange(traffic: &TrafficState, db: &Db, id: i64) -> Option<Exchange> {
    let flow = traffic.flow(id)?;
    let detail = mitm_core::db::get_history_detail(db, id).ok().flatten();
    Some(Exchange::from_flow(flow, detail.as_ref()))
}

fn write_har(db: &Db, ids: &[i64], path: &Path) -> Result<usize, String> {
    let flows = mitm_core::db::get_history_by_ids(db, ids)?;
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &mitm_core::http::har::build_har(&flows)).map_err(|e| e.to_string())?;
    std::io::Write::flush(&mut writer).map_err(|e| e.to_string())?;
    Ok(flows.len())
}
