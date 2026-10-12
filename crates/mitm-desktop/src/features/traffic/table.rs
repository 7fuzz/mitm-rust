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
use crate::widgets::pills::{badge, method_text, render_source_pill, status_text};
use super::{flow_status_badge, ColumnId, ColumnLayout, FlowFlags, TrafficState};

const ROW_HEIGHT: f32 = 26.0;
const LOAD_AHEAD_ROWS: usize = 50;

pub fn show(ui: &mut egui::Ui, state: &mut TrafficState, visible: &[usize], folders: &[CollectionTreeItem], cmds: &mut Commands) {
    let columns = state.prefs.columns.visible();

    let visuals = ui.visuals().clone();
    let ctx = ui.ctx().clone();
    let mut near_end = false;
    // Rows pack edge to edge; ROW_HEIGHT holds the air, and hover/selection fills meet without gaps.
    ui.spacing_mut().item_spacing.y = 0.0;
    let accent = visuals.selection.stroke.color;
    let visuals_mut = ui.visuals_mut();
    visuals_mut.selection.bg_fill = accent.gamma_multiply(0.2);
    visuals_mut.selection.stroke.color = theme::Palette::of(&visuals).text_strong;
    let held_offset = offset_holding_rows_in_place(state, visible, ROW_HEIGHT + ui.spacing().item_spacing.y);
    egui::ScrollArea::horizontal()
        .id_salt("traffic_table_hscroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut builder = TableBuilder::new(ui)
                // egui keeps resized widths per column index, so key them by the visible column set.
                .id_salt(("traffic_table", &columns, state.width_epoch))
                .striped(false)
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
                    let mut reset = false;
                    for &col in &columns {
                        let mut rect = egui::Rect::NOTHING;
                        header.col(|ui| (rect, reset) = header_cell(ui, col, &mut state.prefs.columns, reset));
                        remember_width(&mut state.prefs.widths, col, rect.width());
                        if edge_double_clicked(&ctx, rect) {
                            state.prefs.widths.remove(&col);
                            state.width_epoch += 1;
                        }
                    }
                    header.response().context_menu(|ui| reset |= column_menu(ui, &mut state.prefs.columns, None));
                    if reset {
                        state.reset_columns();
                    }
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
            ui.label(RichText::new("No traffic matches the current filters").weak());
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

    for (index, &col) in columns.iter().enumerate() {
        row.col(|ui| {
            if selected && index == 0 {
                let rect = ui.max_rect();
                ui.painter().vline(rect.left() + 1.0, rect.y_range(), egui::Stroke::new(2.0, visuals.selection.stroke.color));
            }
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
        ColumnId::Id => Some(ui.label(RichText::new(flow.id.to_string()).color(weak).small())),
        ColumnId::Method => {
            ui.add(egui::Label::new(method_text(ui.visuals(), &flow.method)).selectable(false));
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
                ui.add(egui::Label::new(RichText::new("Default").small().family(egui::FontFamily::Monospace).color(weak)).selectable(false));
            } else {
                render_source_pill(ui, &flow.listener_label);
            }
            None
        }
        ColumnId::Host => Some(ui.add(egui::Label::new(RichText::new(&flow.host).color(weak)).truncate())),
        ColumnId::Path => Some(ui.add(egui::Label::new(RichText::new(&flow.path)).truncate())),
        ColumnId::Status => {
            status_cell(ui, flow.status_code, flags);
            None
        }
        ColumnId::ContentType => {
            let text = if flow.content_type.is_empty() { "-" } else { &flow.content_type };
            Some(ui.add(egui::Label::new(RichText::new(text).color(weak).small()).truncate()))
        }
        ColumnId::Size => {
            let text = if flags.is_pending() { "-".to_string() } else { format_bytes(flow.response_size) };
            Some(ui.label(mono_small(text).color(weak)))
        }
        ColumnId::Duration => Some(if flags.is_pending() {
            pending_dots(ui, flags)
        } else {
            ui.label(mono_small(format_duration(flow.duration_ms)).color(weak))
        }),
        ColumnId::RequestAt => Some(clock_cell(ui, flow.request_at.as_deref())),
        ColumnId::ResponseAt => Some(if flags.is_pending() {
            pending_dots(ui, flags)
        } else {
            clock_cell(ui, flow.response_at.as_deref())
        }),
    }
}

fn mono_small(text: impl Into<String>) -> RichText {
    RichText::new(text).small().family(egui::FontFamily::Monospace)
}

/// Coloured code in the table; waiting states keep their badge, since they need to stand out.
fn status_cell(ui: &mut egui::Ui, code: u16, flags: &FlowFlags) {
    if flags.is_pending() {
        flow_status_badge(ui, code, flags);
        return;
    }
    let text = if flags.failed && code != 0 {
        mono_small(code.to_string()).strong().color(theme::color(ui, Tone::Danger))
    } else {
        status_text(ui.visuals(), code)
    };
    ui.add(egui::Label::new(text).selectable(false));
}

fn pending_dots(ui: &mut egui::Ui, flags: &FlowFlags) -> egui::Response {
    let color = theme::color(ui, if flags.pending_intercept { Tone::Warn } else { Tone::Danger });
    ui.label(RichText::new("...").monospace().strong().color(color))
}

fn clock_cell(ui: &mut egui::Ui, at: Option<&str>) -> egui::Response {
    let weak = ui.visuals().weak_text_color();
    match local_clock(at) {
        Some((short, full)) => ui.label(mono_small(short).color(weak)).on_hover_text(full),
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

/// The whole cell is the drag handle and drop zone; dropping on its right half places the column after it.
/// Returns the cell's rect (the column's real width, unlike the cell response, which grows with its content)
/// and whether **Reset columns** was picked, carried along in `reset`.
fn header_cell(ui: &mut egui::Ui, col: ColumnId, layout: &mut ColumnLayout, mut reset: bool) -> (egui::Rect, bool) {
    aligned(ui, col.align(), |ui| {
        ui.add(egui::Label::new(RichText::new(col.label()).small().weak()).selectable(false));
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
    response.context_menu(|ui| reset |= column_menu(ui, layout, Some(col)));
    (rect, reset)
}

/// Returns true when **Reset columns** was picked; the caller resets order, visibility and widths.
fn column_menu(ui: &mut egui::Ui, layout: &mut ColumnLayout, clicked: Option<ColumnId>) -> bool {
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
    ui.label(RichText::new("COLUMNS").weak().small().strong());
    let visible_count = layout.visible().len();
    for col in ColumnId::ALL {
        let mut shown = layout.is_shown(col);
        let is_last = shown && visible_count == 1;
        if ui.add_enabled(!is_last, egui::Checkbox::new(&mut shown, col.menu_label())).changed() {
            layout.set_shown(col, shown);
        }
    }
    ui.separator();
    let reset = ui.button("Reset columns").clicked();
    if reset {
        ui.close();
    }
    reset
}

fn flow_menu(ui: &mut egui::Ui, flow: &HistoryEntry, folders: &[CollectionTreeItem], cmds: &mut Commands) {
    menu_item(ui, "Send to Repeater", AppCommand::SendFlowToRepeater(flow.id), cmds);
    menu_item(ui, "Send to Fuzzer", AppCommand::SendFlowToFuzzer(flow.id), cmds);
    menu_item(ui, "Add to Intercept Rules", AppCommand::AddInterceptRuleForHost(flow.host.clone()), cmds);
    if flow.is_rewritten {
        menu_item(ui, "Show rewrite", AppCommand::ShowRewriteFor(flow.id), cmds);
    }
    ui.menu_button("Save to Collection", |ui| {
        for folder in folders {
            folder_entry(ui, flow.id, folder, cmds);
        }
        if !folders.is_empty() {
            ui.separator();
        }
        new_folder_entry(ui, flow.id, cmds);
    });
    ui.menu_button("Copy", |ui| {
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
    if ui.button(RichText::new("Delete Item").color(theme::color(ui, Tone::Danger))).clicked() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_widths_survive_restarts_unchanged() {
        let mut state = TrafficState::default();
        let restored = state.prefs.columns.visible().iter().map(|c| (*c, c.default_width() + 7.0)).collect();
        state.prefs.widths = restored;
        let before = state.prefs.widths.clone();
        for _restart in 0..3 {
            let ctx = egui::Context::default();
            let mut cmds = Commands::default();
            for _frame in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 600.0))),
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |ui| show(ui, &mut state, &[], &[], &mut cmds));
                output.textures_delta.clear();
            }
        }
        for (col, width) in &before {
            assert!((state.prefs.widths[col] - width).abs() < 0.5, "{col:?}: {width} became {}", state.prefs.widths[col]);
        }
    }
}
