use std::path::Path;
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::state::RewriteHistoryEntry;
use crate::app::command::Commands;
use crate::util::format::headers_to_text;
use crate::ui::theme;
use crate::widgets::confirm::confirm_button;
use crate::widgets::diff::{self, DiffLine};
use crate::widgets::split;
use super::{action_tone, RewriteState};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::section::section;

const LOG_LIMIT: u32 = 2000;

pub struct EntryDiff {
    id: i64,
    headers: Vec<DiffLine>,
    body: Vec<DiffLine>,
}

pub fn reload(state: &mut RewriteState, db_path: &Path) {
    state.log = mitm_core::db::get_rewrite_history_logs(&db_path.to_path_buf(), LOG_LIMIT).unwrap_or_default();
    state.log_loaded = true;
}

pub fn show(ui: &mut egui::Ui, state: &mut RewriteState, db_path: &Path, cmds: &mut Commands) {
    if !state.log_loaded {
        reload(state, db_path);
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} entries", state.log.len())).weak());
        if icon_button(ui, icons::ARROW_CLOCKWISE, "Reload the log").clicked() {
            reload(state, db_path);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if confirm_button(ui, ui.id().with("clear_rewrite_log"), RichText::new(icons::TRASH), "Clear log?", "Clear the rewrite log") {
                cmds.push(super::Cmd::ClearHistory);
            }
        });
    });
    ui.add_space(4.0);

    let Some(selected) = state.selected_log.and_then(|id| state.log.iter().position(|e| e.id == id)) else {
        list(ui, state);
        return;
    };
    let mut fraction = state.prefs.log_fraction;
    let [mut list_ui, mut detail_ui] = split::side_by_side(ui, "rewrite_log_split", &mut fraction);
    state.prefs.log_fraction = fraction;
    list(&mut list_ui, state);
    let entry = &state.log[selected];
    if state.log_diff.as_ref().is_none_or(|d| d.id != entry.id) {
        state.log_diff = Some(EntryDiff {
            id: entry.id,
            headers: diff::diff_lines(&headers_to_text(&entry.original_headers), &headers_to_text(&entry.rewritten_headers)),
            body: diff::diff_lines(&entry.original_body, &entry.rewritten_body),
        });
    }
    if let Some(entry_diff) = &state.log_diff {
        egui::ScrollArea::vertical().id_salt("rewrite_log_detail").auto_shrink([false, false]).show(&mut detail_ui, |ui| {
            detail(ui, entry, entry_diff);
        });
    }
}

fn list(ui: &mut egui::Ui, state: &mut RewriteState) {
    let mut clicked = None;
    TableBuilder::new(ui)
        .striped(false)
        .resizable(true)
        .sense(egui::Sense::click())
        .auto_shrink([false, false])
        .column(Column::initial(130.0).at_least(60.0))
        .column(Column::initial(140.0).at_least(60.0).clip(true))
        .column(Column::initial(60.0))
        .column(Column::remainder().clip(true))
        .header(22.0, |mut header| {
            for title in ["Time", "Rule", "Method", "URL"] {
                header.col(|ui| {
                    ui.label(RichText::new(title).small().weak());
                });
            }
        })
        .body(|body| {
            body.rows(22.0, state.log.len(), |mut row| {
                let entry = &state.log[row.index()];
                row.set_selected(state.selected_log == Some(entry.id));
                row.col(|ui| {
                    ui.label(RichText::new(&entry.created_at).weak().small());
                });
                row.col(|ui| {
                    let tone = action_tone(entry.action_type.split(' ').next().unwrap_or_default());
                    ui.label(RichText::new(&entry.rule_name).color(theme::color(ui, tone)));
                });
                row.col(|ui| {
                    ui.monospace(&entry.method);
                });
                row.col(|ui| {
                    ui.add(egui::Label::new(RichText::new(&entry.original_url).monospace()).truncate());
                });
                if row.response().clicked() {
                    clicked = Some(entry.id);
                }
            });
        });
    if clicked.is_some() {
        state.selected_log = clicked;
    }
}

fn detail(ui: &mut egui::Ui, entry: &RewriteHistoryEntry, entry_diff: &EntryDiff) {
    ui.horizontal(|ui| {
        ui.strong(&entry.rule_name);
        ui.label(RichText::new(&entry.action_type).weak());
        if let Some(status) = entry.status_code {
            ui.label(RichText::new(status.to_string()).monospace());
        }
    });
    ui.add_space(4.0);
    ui.label(RichText::new(&entry.original_url).monospace());
    if entry.rewritten_url != entry.original_url {
        ui.label(RichText::new(format!("→ {}", entry.rewritten_url)).monospace().color(theme::color(ui, theme::Tone::Success)));
    }
    ui.add_space(6.0);
    section(ui, "Headers");
    diff::show(ui, "rewrite_headers_diff", &entry_diff.headers);
    ui.add_space(6.0);
    section(ui, "Body");
    diff::show(ui, "rewrite_body_diff", &entry_diff.body);
}
