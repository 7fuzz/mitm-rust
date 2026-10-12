use eframe::egui::{self, Align2, Key, Modifiers, RichText};
use crate::app::command::{AppCommand, Commands};
use crate::ui::shortcuts;
use crate::app::state::AppState;

#[derive(Default)]
pub struct PaletteState {
    pub open: bool,
    query: String,
    selected: usize,
    /// Set when the keyboard moves the selection; scrolling every frame would undo mouse-wheel scrolling.
    scroll_to_selected: bool,
}

impl PaletteState {
    pub fn open(&mut self) {
        *self = Self { open: true, ..Default::default() };
    }
}

#[derive(Clone)]
pub struct PaletteItem {
    pub label: String,
    pub command: AppCommand,
    pub shortcut: Option<String>,
}

pub fn palette_items(state: &AppState, ctx: &egui::Context) -> Vec<PaletteItem> {
    let mut items = Vec::new();

    for action in shortcuts::palette_actions() {
        let shortcut = shortcuts::shortcut_for(action).map(|s| ctx.format_shortcut(&s));
        items.push(PaletteItem {
            label: action.label(),
            command: action.command(),
            shortcut,
        });
    }

    for entry in &state.credentials.entries {
        if entry.fields.iter().any(|f| f.kind == mitm_core::credentials::FieldKind::Totp) {
            items.push(PaletteItem {
                label: format!("Copy TOTP: {}", entry.name),
                command: crate::features::credentials::Cmd::CopyTotp(entry.id.clone()).into(),
                shortcut: None,
            });
        }
    }
    if state.credentials.has_password && !state.credentials.locked {
        items.push(PaletteItem { label: "Lock credentials".to_string(), command: crate::features::credentials::Cmd::Lock.into(), shortcut: None });
    }

    for ws in &state.environments.workspaces {
        let is_active = ws.id == state.environments.active_workspace_id;
        items.push(PaletteItem {
            label: format!("Switch workspace: {}{}", ws.name, if is_active { " (current)" } else { "" }),
            command: AppCommand::SelectWorkspace(ws.id.clone()),
            shortcut: None,
        });
    }

    let is_none_active = state.environments.active_environment_id.is_none();
    items.push(PaletteItem {
        label: format!("Switch environment: No environment{}", if is_none_active { " (current)" } else { "" }),
        command: crate::features::environments::Cmd::Select(None).into(),
        shortcut: None,
    });

    for env in &state.environments.environments {
        let is_active = Some(&env.id) == state.environments.active_environment_id.as_ref();
        items.push(PaletteItem {
            label: format!("Switch environment: {}{}", env.name, if is_active { " (current)" } else { "" }),
            command: crate::features::environments::Cmd::Select(Some(env.id.clone())).into(),
            shortcut: None,
        });
    }

    items
}

pub fn show(ctx: &egui::Context, state: &mut PaletteState, app_state: &AppState, cmds: &mut Commands) {
    if !state.open {
        return;
    }
    let all_items = palette_items(app_state, ctx);
    let matches = matching(&all_items, &state.query);
    state.selected = state.selected.min(matches.len().saturating_sub(1));

    let id = egui::Id::new("command_palette");
    let area = egui::Modal::default_area(id).anchor(Align2::CENTER_TOP, egui::vec2(0.0, 80.0));
    let mut chosen = None;
    let modal = egui::Modal::new(id).area(area).show(ctx, |ui| {
        ui.set_width(420.0);
        let search = crate::widgets::search::search_field(ui, &mut state.query, "Type a command");
        search.request_focus();
        if search.changed() {
            state.selected = 0;
            state.scroll_to_selected = true;
        }
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                state.selected = (state.selected + 1).min(matches.len().saturating_sub(1));
                state.scroll_to_selected = true;
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                state.selected = state.selected.saturating_sub(1);
                state.scroll_to_selected = true;
            }
            if i.consume_key(Modifiers::NONE, Key::Enter) {
                chosen = matches.get(state.selected).cloned();
            }
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (index, item) in matches.iter().enumerate() {
                let selected = index == state.selected;
                let response = egui::Sides::new().show(
                    ui,
                    |ui| ui.add(egui::Button::selectable(selected, &item.label).frame_when_inactive(false)),
                    |ui| {
                        if let Some(shortcut) = &item.shortcut {
                            ui.label(RichText::new(shortcut).weak().monospace());
                        }
                    },
                );
                if selected && std::mem::take(&mut state.scroll_to_selected) {
                    response.0.scroll_to_me(None);
                }
                if response.0.clicked() {
                    chosen = Some(item.clone());
                }
            }
        });
    });

    if let Some(item) = chosen {
        cmds.push(item.command);
        state.open = false;
    } else if modal.should_close() {
        state.open = false;
    }
}

fn matching(items: &[PaletteItem], query: &str) -> Vec<PaletteItem> {
    let query = query.trim().to_lowercase();
    let mut scored: Vec<(usize, PaletteItem)> = items
        .iter()
        .filter_map(|item| fuzzy_score(&item.label.to_lowercase(), &query).map(|score| (score, item.clone())))
        .collect();
    scored.sort_by_key(|(score, _)| *score);
    scored.into_iter().map(|(_, item)| item).collect()
}

// Lower is better: substring hits rank first, then subsequences by how spread out they are.
fn fuzzy_score(label: &str, query: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(0);
    }
    if let Some(pos) = label.find(query) {
        return Some(pos);
    }
    let mut chars = label.char_indices();
    let mut first = None;
    let mut last = 0;
    for q in query.chars() {
        let (i, _) = chars.by_ref().find(|(_, c)| *c == q)?;
        first.get_or_insert(i);
        last = i;
    }
    Some(1000 + last - first.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::fuzzy_score;

    #[test]
    fn ranks_substring_before_subsequence() {
        let substring = fuzzy_score("open repeater", "rep").unwrap();
        let subsequence = fuzzy_score("open repeater", "orp").unwrap();
        assert!(substring < subsequence);
        assert_eq!(fuzzy_score("open traffic", "xyz"), None);
    }

    #[test]
    fn palette_items_include_workspaces_and_environments() {
        let mut state = crate::app::state::AppState::default();
        state.environments.workspaces.push(mitm_core::workspace::Workspace {
            id: "ws-1".to_string(),
            name: "Backend WS".to_string(),
            description: None,
            active_environment_id: None,
            created_at_ms: 0,
            updated_at_ms: 0,
        });
        state.environments.environments.push(mitm_core::workspace::Environment {
            id: "env-1".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Staging".to_string(),
            is_active: false,
            variables: vec![],
            created_at_ms: 0,
            updated_at_ms: 0,
        });

        let ctx = egui::Context::default();
        let items = super::palette_items(&state, &ctx);
        assert!(items.iter().any(|i| i.label.contains("Backend WS")));
        assert!(items.iter().any(|i| i.label.contains("Staging")));
        assert!(items.iter().any(|i| i.label.contains("No environment")));
        assert!(items.iter().any(|i| i.label == "Import workspace…"));
        assert!(items.iter().any(|i| i.label == "Export workspace…"));
    }
}
