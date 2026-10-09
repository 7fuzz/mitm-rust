use std::collections::HashMap;
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder, TableRow};
use mitm_core::collections::CollectionTreeItem;
use mitm_core::state::HistoryEntry;
use crate::app::command::{AppCommand, Commands};
use crate::features::collections::SaveTarget;
use crate::util::format::{format_bytes, format_duration, local_clock, CopyFormat};
use crate::ui::theme::{self, Tone};
use crate::widgets::copy_custom;
use crate::widgets::pills::{badge, render_method_pill, render_source_pill};
use super::{flow_status_badge, ColumnId, ColumnLayout, FlowFlags, TrafficState};

const ROW_HEIGHT: f32 = 26.0;
const LOAD_AHEAD_ROWS: usize = 50;

pub fn show(ui: &mut egui::Ui, state: &mut TrafficState, visible: &[usize], folders: &[CollectionTreeItem], cmds: &mut Commands) {
    let columns = state.prefs.columns.visible();

    let visuals = ui.visuals().clone();
    let ctx = ui.ctx().clone();
    let mut near_end = false;
    let held_offset = offset_holding_rows_in_place(state, visible, ROW_HEIGHT + ui.spacing().item_spacing.y);
    egui::ScrollArea::horizontal()
        .id_salt("traffic_table_hscroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut builder = TableBuilder::new(ui)
                // egui keeps resized widths per column index, so key them by the visible column set.
                .id_salt(("traffic_table", &columns, state.width_epoch))
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .auto_shrink([false, false])
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
            for col in &columns {
                let width = state.prefs.widths.get(col).copied().unwrap_or_else(|| col.default_width());
                builder = builder.column(Column::initial(width).at_least(40.0).clip(true));
            }
            if let Some(offset) = held_offset {
                builder = builder.vertical_scroll_offset(offset);
            }

            let output = builder
                .header(24.0, |mut header| {
                    for &col in &columns {
                        // The returned rect only covers the label; the response spans the whole column.
                        let (_, cell) = header.col(|ui| header_cell(ui, col, &mut state.prefs.columns));
                        let rect = cell.rect;
                        remember_width(&mut state.prefs.widths, col, rect.width());
                        if edge_double_clicked(&ctx, rect) {
                            state.prefs.widths.remove(&col);
                            state.width_epoch += 1;
                        }
                    }
                    header.response().context_menu(|ui| column_menu(ui, &mut state.prefs.columns, None));
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, visible.len(), |mut row| {
                        near_end |= row.index() + LOAD_AHEAD_ROWS >= visible.len();
                        let flow = &state.flows[visible[row.index()]];
                        let selected = state.selected_id == Some(flow.id);
                        row.set_selected(selected);
                        flow_row(&mut row, &visuals, &columns, flow, selected, folders, cmds);
                    });
                });
            state.scroll_anchor = visible.first().map(|&i| (state.flows[i].id, output.state.offset.y));
        });

    if state.has_older && (near_end || visible.is_empty()) {
        cmds.push(AppCommand::LoadOlderFlows);
    }

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

// New flows are pushed on top; once scrolled away from the top, keep the rows being read where they are.
fn offset_holding_rows_in_place(state: &TrafficState, visible: &[usize], row_step: f32) -> Option<f32> {
    let (top_id, offset) = state.scroll_anchor?;
    if offset < 1.0 {
        return None;
    }
    let added = visible.iter().take_while(|&&i| state.flows[i].id > top_id).count();
    (added > 0).then_some(offset + added as f32 * row_step)
}

fn flow_row(
    row: &mut TableRow,
    visuals: &egui::Visuals,
    columns: &[ColumnId],
    flow: &HistoryEntry,
    selected: bool,
    folders: &[CollectionTreeItem],
    cmds: &mut Commands,
) {
    let flags = FlowFlags::of(flow);
    let tint = if selected { None } else { row_tint(visuals, &flags) };
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
    response.context_menu(|ui| flow_menu(ui, flow, folders, cmds));
}

fn row_tint(visuals: &egui::Visuals, flags: &FlowFlags) -> Option<Color32> {
    let (tone, alpha) = if flags.pending_intercept {
        (Tone::Warn, 38)
    } else if flags.pending_response {
        (Tone::Danger, 26)
    } else if flags.failed {
        (Tone::Danger, 14)
    } else if flags.intercepted {
        (Tone::Warn, 14)
    } else if flags.rewritten {
        (Tone::Info, 14)
    } else {
        return None;
    };
    Some(tone.tint(visuals, alpha))
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
                badge(ui, "INT", Tone::Warn);
            }
            if flags.rewritten {
                badge(ui, "RW", Tone::Info);
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
    let color = theme::color(ui, if flags.pending_intercept { Tone::Warn } else { Tone::Danger });
    ui.label(RichText::new("...").monospace().strong().color(color))
}

fn clock_cell(ui: &mut egui::Ui, at: Option<&str>) -> egui::Response {
    let weak = ui.visuals().weak_text_color();
    match local_clock(at) {
        Some((short, full)) => ui.label(RichText::new(short).color(weak).size(11.0)).on_hover_text(full),
        None => ui.label(RichText::new("-").color(weak)),
    }
}

// The resize handle owns the edge, so read the double-click from raw input.
fn edge_double_clicked(ctx: &egui::Context, cell: egui::Rect) -> bool {
    ctx.input(|i| {
        i.pointer.button_double_clicked(egui::PointerButton::Primary)
            && i.pointer.interact_pos().is_some_and(|p| (p.x - cell.right()).abs() <= 4.0 && cell.y_range().contains(p.y))
    })
}

fn remember_width(widths: &mut HashMap<ColumnId, f32>, col: ColumnId, width: f32) {
    if widths.get(&col).is_none_or(|w| (w - width).abs() > 0.5) {
        widths.insert(col, width);
    }
}

// The whole cell is the drag handle and drop zone; dropping on its right half places the column after it.
fn header_cell(ui: &mut egui::Ui, col: ColumnId, layout: &mut ColumnLayout) {
    aligned(ui, col.align(), |ui| {
        ui.add(egui::Label::new(RichText::new(col.label()).strong().size(11.0)).selectable(false));
    });
    let rect = ui.max_rect();
    let response = ui.interact(rect, ui.id().with(("traffic_header", col)), egui::Sense::click_and_drag());
    response.dnd_set_drag_payload(col);
    let after = response.hover_pos().is_some_and(|p| p.x > rect.center().x);
    if response.dnd_hover_payload::<ColumnId>().is_some_and(|dragged| *dragged != col) {
        let x = if after { rect.right() } else { rect.left() };
        let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
        ui.painter().with_clip_rect(rect.expand(2.0)).vline(x, rect.y_range(), stroke);
    }
    if let Some(dragged) = response.dnd_release_payload::<ColumnId>() {
        layout.move_next_to(*dragged, col, after);
    }
    response.context_menu(|ui| column_menu(ui, layout, Some(col)));
}

fn column_menu(ui: &mut egui::Ui, layout: &mut ColumnLayout, clicked: Option<ColumnId>) {
    if let Some(col) = clicked {
        let visible = layout.visible();
        let index = visible.iter().position(|&c| c == col).unwrap_or(0);
        if ui.add_enabled(index > 0, egui::Button::new("Move left")).clicked() {
            layout.shift(col, false);
            ui.close();
        }
        if ui.add_enabled(index + 1 < visible.len(), egui::Button::new("Move right")).clicked() {
            layout.shift(col, true);
            ui.close();
        }
        ui.separator();
    }
    ui.label(RichText::new("COLUMNS").weak().size(10.0).strong());
    let visible_count = layout.visible().len();
    for col in ColumnId::ALL {
        let mut shown = layout.is_shown(col);
        let is_last = shown && visible_count == 1;
        if ui.add_enabled(!is_last, egui::Checkbox::new(&mut shown, col.menu_label())).changed() {
            layout.set_shown(col, shown);
        }
    }
    ui.separator();
    if ui.button("Reset columns").clicked() {
        *layout = ColumnLayout::default();
        ui.close();
    }
}

fn flow_menu(ui: &mut egui::Ui, flow: &HistoryEntry, folders: &[CollectionTreeItem], cmds: &mut Commands) {
    menu_item(ui, "🔁 Send to Repeater", AppCommand::SendFlowToRepeater(flow.id), cmds);
    menu_item(ui, "⚡ Send to Fuzzer", AppCommand::SendFlowToFuzzer(flow.id), cmds);
    menu_item(ui, "🛡 Add to Intercept Rules", AppCommand::AddInterceptRuleForHost(flow.host.clone()), cmds);
    if flow.is_rewritten {
        menu_item(ui, "✏ Show rewrite", AppCommand::ShowRewriteFor(flow.id), cmds);
    }
    ui.menu_button("📁 Save to Collection", |ui| {
        for folder in folders {
            folder_entry(ui, flow.id, folder, cmds);
        }
        if !folders.is_empty() {
            ui.separator();
        }
        new_folder_entry(ui, flow.id, cmds);
    });
    ui.menu_button("📋 Copy", |ui| {
        if ui.button("Copy URL").clicked() {
            ui.ctx().copy_text(flow.url.clone());
            ui.close();
        }
        menu_item(ui, "Copy Curl", AppCommand::CopyFlow(flow.id, CopyFormat::Curl), cmds);
        menu_item(ui, "Copy Curl and Res", AppCommand::CopyFlow(flow.id, CopyFormat::CurlAndResponse), cmds);
        menu_item(ui, "Copy URL, Body, and Response", AppCommand::CopyFlow(flow.id, CopyFormat::UrlBodyAndResponse), cmds);
        if let Some(options) = copy_custom::submenu(ui) {
            cmds.push(AppCommand::CopyFlowCustom(flow.id, options));
        }
    });
    ui.separator();
    if ui.button(RichText::new("🗑 Delete Item").color(theme::color(ui, Tone::Danger))).clicked() {
        cmds.push(AppCommand::DeleteFlow(flow.id));
        ui.close();
    }
}

fn menu_item(ui: &mut egui::Ui, label: &str, cmd: AppCommand, cmds: &mut Commands) {
    if ui.button(label).clicked() {
        cmds.push(cmd);
        ui.close();
    }
}

fn save_into(ui: &mut egui::Ui, label: &str, flow_id: i64, target: SaveTarget, cmds: &mut Commands) {
    if ui.button(label).clicked() {
        cmds.push(AppCommand::SaveFlowToCollection { flow_id, target });
        ui.close();
    }
}

fn folder_entry(ui: &mut egui::Ui, flow_id: i64, folder: &CollectionTreeItem, cmds: &mut Commands) {
    if folder.children.is_empty() {
        save_into(ui, &folder.name, flow_id, SaveTarget::Folder(folder.id.clone()), cmds);
        return;
    }
    ui.menu_button(&folder.name, |ui| {
        save_into(ui, "Save here", flow_id, SaveTarget::Folder(folder.id.clone()), cmds);
        ui.separator();
        for child in &folder.children {
            folder_entry(ui, flow_id, child, cmds);
        }
    });
}

fn new_folder_entry(ui: &mut egui::Ui, flow_id: i64, cmds: &mut Commands) {
    let id = egui::Id::new("traffic_new_folder_name");
    let mut name: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    let edit = ui.add(egui::TextEdit::singleline(&mut name).hint_text("New folder…").desired_width(160.0));
    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !name.trim().is_empty() {
        cmds.push(AppCommand::SaveFlowToCollection { flow_id, target: SaveTarget::NewFolder(name.trim().to_string()) });
        name.clear();
        ui.close();
    }
    ui.data_mut(|d| d.insert_temp(id, name));
}
