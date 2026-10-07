use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder, TableRow};
use mitm_core::state::HistoryEntry;
use crate::command::{AppCommand, Commands};
use crate::format::{format_bytes, format_duration, local_clock, CopyFormat};
use crate::widgets::pills::{badge, render_method_pill, render_source_pill, tinted, AMBER, ROSE, SKY};
use super::{flow_status_badge, ColumnId, FlowFlags, TrafficState};

const ROW_HEIGHT: f32 = 26.0;

pub fn show(ui: &mut egui::Ui, state: &mut TrafficState, visible: &[usize], cmds: &mut Commands) {
    let columns: Vec<ColumnId> = ColumnId::ALL
        .into_iter()
        .filter(|c| !state.hidden_columns.contains(c))
        .collect();

    egui::ScrollArea::horizontal()
        .id_salt("traffic_table_hscroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut builder = TableBuilder::new(ui)
                // egui keeps resized widths per column index, so key them by the visible column set.
                .id_salt(("traffic_table", &columns))
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .auto_shrink([false, false])
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
            for col in &columns {
                builder = builder.column(Column::initial(col.default_width()).at_least(40.0).clip(true));
            }

            builder
                .header(24.0, |mut header| {
                    for col in &columns {
                        header.col(|ui| {
                            aligned(ui, col.align(), |ui| {
                                ui.add(egui::Label::new(RichText::new(col.label()).strong().size(11.0)).selectable(false));
                            });
                        });
                    }
                    header.response().context_menu(|ui| column_menu(ui, &mut state.hidden_columns));
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, visible.len(), |mut row| {
                        let flow = &state.flows[visible[row.index()]];
                        let selected = state.selected_id == Some(flow.id);
                        row.set_selected(selected);
                        flow_row(&mut row, &columns, flow, selected, cmds);
                    });
                });
        });

    if visible.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(RichText::new("No traffic matches the current filters").weak().italics());
        });
    }

    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
        if let Some(selected) = state.selected_flow() {
            ui.ctx().copy_text(selected.url.clone());
        }
    }
}

fn flow_row(row: &mut TableRow, columns: &[ColumnId], flow: &HistoryEntry, selected: bool, cmds: &mut Commands) {
    let flags = FlowFlags::of(flow);
    let tint = if selected { None } else { row_tint(&flags) };
    let mut text_cells: Vec<egui::Response> = Vec::new();

    for &col in columns {
        row.col(|ui| {
            if let Some(color) = tint {
                let half_gap = 0.5 * ui.spacing().item_spacing;
                let painter = ui.painter().with_clip_rect(ui.clip_rect().expand2(half_gap));
                painter.rect_filled(ui.max_rect().expand2(half_gap), 0.0, color);
            }
            aligned(ui, col.align(), |ui| {
                if let Some(text) = cell(ui, col, flow, &flags) {
                    text_cells.push(text);
                }
            });
        });
    }

    // Selectable labels swallow row clicks; drag-to-select never counts as a click.
    let response = text_cells.into_iter().fold(row.response(), |acc, r| acc | r);

    if response.clicked() || response.double_clicked() {
        cmds.push(AppCommand::SelectFlow(flow.id));
    }
    response.context_menu(|ui| flow_menu(ui, flow, cmds));
}

fn row_tint(flags: &FlowFlags) -> Option<Color32> {
    if flags.pending_intercept {
        Some(tinted(AMBER, 38))
    } else if flags.pending_response {
        Some(tinted(ROSE, 26))
    } else if flags.failed {
        Some(tinted(ROSE, 14))
    } else if flags.intercepted {
        Some(tinted(AMBER, 14))
    } else if flags.rewritten {
        Some(tinted(SKY, 14))
    } else {
        None
    }
}

fn aligned(ui: &mut egui::Ui, align: egui::Align, add: impl FnOnce(&mut egui::Ui)) {
    let layout = match align {
        egui::Align::Min => egui::Layout::left_to_right(egui::Align::Center),
        egui::Align::Center => egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        egui::Align::Max => egui::Layout::right_to_left(egui::Align::Center),
    };
    ui.with_layout(layout, add);
}

fn cell(ui: &mut egui::Ui, col: ColumnId, flow: &HistoryEntry, flags: &FlowFlags) -> Option<egui::Response> {
    let weak = ui.visuals().weak_text_color();
    match col {
        ColumnId::Id => Some(ui.label(RichText::new(flow.id.to_string()).color(weak).size(10.0))),
        ColumnId::Method => {
            render_method_pill(ui, &flow.method);
            if flags.intercepted || flags.pending_intercept {
                badge(ui, "INT", AMBER);
            }
            if flags.rewritten {
                badge(ui, "RW", SKY);
            }
            None
        }
        ColumnId::Source => {
            if flow.listener_label.is_empty() || flow.listener_label == "Default" {
                ui.add(egui::Label::new(RichText::new("Default").monospace().size(10.0).color(weak)).selectable(false));
            } else {
                render_source_pill(ui, &flow.listener_label);
            }
            None
        }
        ColumnId::Host => Some(ui.add(egui::Label::new(RichText::new(&flow.host).strong()).truncate())),
        ColumnId::Path => Some(ui.add(egui::Label::new(RichText::new(&flow.path).color(weak)).truncate())),
        ColumnId::Status => {
            flow_status_badge(ui, flow.status_code, flags);
            None
        }
        ColumnId::ContentType => {
            let text = if flow.content_type.is_empty() { "-" } else { &flow.content_type };
            Some(ui.add(egui::Label::new(RichText::new(text).color(weak).size(11.0)).truncate()))
        }
        ColumnId::Size => {
            let text = if flags.is_pending() { "-".to_string() } else { format_bytes(flow.response_size) };
            Some(ui.label(RichText::new(text).color(weak).size(11.0)))
        }
        ColumnId::Duration => Some(if flags.is_pending() {
            pending_dots(ui, flags)
        } else {
            ui.label(RichText::new(format_duration(flow.duration_ms)).color(weak).size(11.0))
        }),
        ColumnId::RequestAt => Some(clock_cell(ui, flow.request_at.as_deref())),
        ColumnId::ResponseAt => Some(if flags.is_pending() {
            pending_dots(ui, flags)
        } else {
            clock_cell(ui, flow.response_at.as_deref())
        }),
    }
}

fn pending_dots(ui: &mut egui::Ui, flags: &FlowFlags) -> egui::Response {
    let color = if flags.pending_intercept { AMBER } else { ROSE };
    ui.label(RichText::new("...").monospace().strong().color(color))
}

fn clock_cell(ui: &mut egui::Ui, at: Option<&str>) -> egui::Response {
    let weak = ui.visuals().weak_text_color();
    match local_clock(at) {
        Some((short, full)) => ui.label(RichText::new(short).color(weak).size(11.0)).on_hover_text(full),
        None => ui.label(RichText::new("-").color(weak)),
    }
}

fn column_menu(ui: &mut egui::Ui, hidden: &mut Vec<ColumnId>) {
    ui.label(RichText::new("COLUMNS").weak().size(10.0).strong());
    let visible_count = ColumnId::ALL.len() - hidden.len();
    for col in ColumnId::ALL {
        let mut shown = !hidden.contains(&col);
        let is_last = shown && visible_count == 1;
        if ui.add_enabled(!is_last, egui::Checkbox::new(&mut shown, col.menu_label())).changed() {
            if shown {
                hidden.retain(|&c| c != col);
            } else {
                hidden.push(col);
            }
        }
    }
    ui.separator();
    if ui.button("Reset columns").clicked() {
        hidden.clear();
        ui.close();
    }
}

fn flow_menu(ui: &mut egui::Ui, flow: &HistoryEntry, cmds: &mut Commands) {
    let mut item = |ui: &mut egui::Ui, label: &str, cmd: AppCommand| {
        if ui.button(label).clicked() {
            cmds.push(cmd);
            ui.close();
        }
    };

    item(ui, "🔁 Send to Repeater", AppCommand::SendFlowToRepeater(flow.id));
    item(ui, "⚡ Send to Fuzzer", AppCommand::SendFlowToFuzzer(flow.id));
    item(ui, "🛡 Add to Intercept Rules", AppCommand::AddInterceptRuleForHost(flow.host.clone()));
    ui.menu_button("📋 Copy", |ui| {
        if ui.button("Copy URL").clicked() {
            ui.ctx().copy_text(flow.url.clone());
            ui.close();
        }
        item(ui, "Copy Curl", AppCommand::CopyFlow(flow.id, CopyFormat::Curl));
        item(ui, "Copy Curl and Res", AppCommand::CopyFlow(flow.id, CopyFormat::CurlAndResponse));
        item(ui, "Copy URL, Body, and Response", AppCommand::CopyFlow(flow.id, CopyFormat::UrlBodyAndResponse));
    });
    ui.separator();
    if ui.button(RichText::new("🗑 Delete Item").color(ROSE)).clicked() {
        cmds.push(AppCommand::DeleteFlow(flow.id));
        ui.close();
    }
}
