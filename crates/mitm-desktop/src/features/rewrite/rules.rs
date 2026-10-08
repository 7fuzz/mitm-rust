use eframe::egui::{self, RichText};
use mitm_core::state::{ListenerConfig, RewriteRule};
use crate::command::{AppCommand, Commands};
use crate::theme;
use crate::widgets::{source_scope, split};
use super::{action_label, action_tone, editor, RewriteState};

pub fn show(ui: &mut egui::Ui, state: &mut RewriteState, listeners: &[ListenerConfig], cmds: &mut Commands) {
    let now = ui.input(|i| i.time);
    toolbar(ui, state, listeners, cmds, now);
    ui.separator();
    let Some(index) = state.selected_index() else {
        list(ui, state, now);
        return;
    };
    let mut fraction = state.prefs.list_fraction;
    let [mut list_ui, mut editor_ui] = split::side_by_side(ui, "rewrite_split", &mut fraction);
    state.prefs.list_fraction = fraction;
    list(&mut list_ui, state, now);
    egui::ScrollArea::vertical().id_salt("rewrite_editor").auto_shrink([false, false]).show(&mut editor_ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong("Rule");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✖").on_hover_text("Close editor").clicked() {
                    state.selected = None;
                }
            });
        });
        ui.separator();
        let Some(rule) = state.rules.get_mut(index) else { return };
        if state.mock_headers.as_ref().is_none_or(|(id, _)| *id != rule.id) {
            state.mock_headers = Some((rule.id.clone(), editor::mock_header_rows(rule)));
        }
        let Some((_, mock_headers)) = state.mock_headers.as_mut() else { return };
        let changed = editor::show(ui, rule, mock_headers);
        editor::preview(ui, rule, &mut state.sample);
        if changed {
            state.mark_dirty(now);
        }
    });
}

fn list(ui: &mut egui::Ui, state: &mut RewriteState, now: f64) {
    egui::ScrollArea::vertical().id_salt("rewrite_rules").auto_shrink([false, false]).show(ui, |ui| {
        rules_table(ui, state, now);
    });
}

fn toolbar(ui: &mut egui::Ui, state: &mut RewriteState, listeners: &[ListenerConfig], cmds: &mut Commands, now: f64) {
    ui.horizontal(|ui| {
        let label = if state.enabled { "● Rewrite ON" } else { "○ Rewrite OFF" };
        if ui.toggle_value(&mut state.enabled, label).changed() {
            cmds.push(AppCommand::SetRewriteEnabled(state.enabled));
        }
        ui.separator();
        if source_scope::chips(ui, &mut state.scope, listeners) {
            cmds.push(AppCommand::SetRewriteScope(state.scope.clone()));
        }
        ui.separator();
        ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("🔍 Search rules").desired_width(180.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕").on_hover_text("Add rule").clicked() {
                state.add_rule(now);
            }
        });
    });
}

fn rules_table(ui: &mut egui::Ui, state: &mut RewriteState, now: f64) {
    let search = state.search.trim().to_lowercase();
    let count = state.rules.len();
    let mut action = None;

    egui::Grid::new("rewrite_rules_grid").striped(true).num_columns(5).spacing([10.0, 6.0]).show(ui, |ui| {
        for (index, rule) in state.rules.iter_mut().enumerate() {
            if !search.is_empty() && !rule_text(rule).contains(&search) {
                continue;
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new("⏶").small()).clicked() {
                    action = Some(RowAction::Swap(index, index - 1));
                }
                if ui.add_enabled(index + 1 < count, egui::Button::new("⏷").small()).clicked() {
                    action = Some(RowAction::Swap(index, index + 1));
                }
            });
            ui.push_id(&rule.id, |ui| {
                if ui.checkbox(&mut rule.enabled, "").changed() {
                    action = Some(RowAction::Edited);
                }
            });
            let selected = state.selected.as_deref() == Some(&rule.id);
            let name = ui.selectable_label(selected, RichText::new(&rule.name).strong());
            if name.clicked() {
                state.selected = Some(rule.id.clone());
            }
            name.context_menu(|ui| {
                if ui.button("Duplicate").clicked() {
                    action = Some(RowAction::Duplicate(index));
                    ui.close();
                }
                if ui.button(RichText::new("Delete").color(theme::color(ui, theme::Tone::Danger))).clicked() {
                    action = Some(RowAction::Delete(index));
                    ui.close();
                }
            });
            ui.add(egui::Label::new(RichText::new(match_text(rule)).monospace().weak()).truncate());
            let tone = action_tone(&rule.action_type);
            ui.label(RichText::new(action_label(&rule.action_type)).color(theme::color(ui, tone)).strong().size(11.0));
            ui.end_row();
        }
    });

    match action {
        Some(RowAction::Swap(a, b)) => {
            state.rules.swap(a, b);
            state.mark_dirty(now);
        }
        Some(RowAction::Duplicate(index)) => state.duplicate(index, now),
        Some(RowAction::Delete(index)) => state.delete(index, now),
        Some(RowAction::Edited) => state.mark_dirty(now),
        None => {}
    }
}

enum RowAction {
    Swap(usize, usize),
    Duplicate(usize),
    Delete(usize),
    Edited,
}

fn match_text(rule: &RewriteRule) -> String {
    if rule.match_field == "all" {
        return "all traffic".to_string();
    }
    format!("{} {} {}", rule.match_field, rule.match_operator, rule.match_value)
}

fn rule_text(rule: &RewriteRule) -> String {
    format!("{} {} {}", rule.name, match_text(rule), action_label(&rule.action_type)).to_lowercase()
}
