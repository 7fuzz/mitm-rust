use eframe::egui::{self, Align2, Key, Modifiers, RichText};
use mitm_core::workspace::EnvironmentVariable;
use super::EnvironmentsState;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Scope {
    #[default]
    Workspace,
    Repeater,
}

#[derive(Default)]
pub struct QuickVars {
    pub open: bool,
    scope: Scope,
    query: String,
    selected: usize,
    editing: Option<usize>,
    edit_text: String,
}

impl QuickVars {
    pub fn toggle(&mut self, scope: Scope) {
        *self = Self { open: !self.open, scope, ..Self::default() };
    }
}

#[derive(Default)]
struct Keys {
    down: bool,
    up: bool,
    left: bool,
    right: bool,
    enter: bool,
    escape: bool,
    add: bool,
}

pub fn show(ctx: &egui::Context, state: &mut EnvironmentsState, now: f64) {
    if !state.quick.open {
        return;
    }
    let EnvironmentsState { quick, environments, active_environment_id, repeater_env, .. } = state;
    let env = match quick.scope {
        Scope::Workspace => environments.iter_mut().find(|e| Some(&e.id) == active_environment_id.as_ref()),
        Scope::Repeater => repeater_env.as_mut(),
    };
    let Some(env) = env else {
        quick.open = false;
        return;
    };
    let env_id = env.id.clone();
    let variables = &mut env.variables;
    let hint = match quick.scope {
        Scope::Workspace => "Search variables",
        Scope::Repeater => "Search repeater variables",
    };
    let mut changed = false;

    let id = egui::Id::new("quick_variables");
    let area = egui::Modal::default_area(id).anchor(Align2::CENTER_TOP, egui::vec2(0.0, 80.0));
    let modal = egui::Modal::new(id).area(area).show(ctx, |ui| {
        ui.set_width(460.0);
        let keys = consume_keys(ui, quick.editing.is_some());
        let matches = matching(variables, &quick.query);
        quick.selected = quick.selected.min(matches.len().saturating_sub(1));

        if quick.editing.is_none() {
            if keys.down {
                quick.selected = (quick.selected + 1).min(matches.len().saturating_sub(1));
            }
            if keys.up {
                quick.selected = quick.selected.saturating_sub(1);
            }
            if let Some(&var_idx) = matches.get(quick.selected) {
                if keys.left || keys.right {
                    changed |= cycle_variant(&mut variables[var_idx], keys.right);
                }
                if keys.enter {
                    start_edit(quick, var_idx, &variables[var_idx]);
                }
            }
            if keys.add {
                let key = match quick.query.trim() {
                    "" => "new_variable".to_string(),
                    query => query.to_string(),
                };
                variables.push(new_variable(key));
                changed = true;
                quick.query.clear();
                quick.selected = variables.len() - 1;
                start_edit(quick, variables.len() - 1, &variables[variables.len() - 1]);
            }
        } else if keys.escape {
            quick.editing = None;
        }

        let search = ui.add(
            egui::TextEdit::singleline(&mut quick.query)
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        );
        if quick.editing.is_none() {
            search.request_focus();
        }
        if search.changed() {
            quick.selected = 0;
        }
        ui.separator();

        let matches = matching(variables, &quick.query);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (row, &var_idx) in matches.iter().enumerate() {
                let var = &mut variables[var_idx];
                if quick.editing == Some(var_idx) {
                    changed |= edit_row(ui, quick, var);
                    continue;
                }
                let selected = row == quick.selected;
                let (key, _) = egui::Sides::new().show(
                    ui,
                    |ui| ui.add(egui::Button::selectable(selected, RichText::new(&var.key).monospace()).frame_when_inactive(false)),
                    |ui| value_label(ui, var),
                );
                if selected {
                    key.scroll_to_me(None);
                }
                if key.clicked() {
                    quick.selected = row;
                }
                if key.double_clicked() {
                    start_edit(quick, var_idx, var);
                }
            }
        });
    });

    if changed {
        state.mark_dirty(&env_id, now);
    }
    if state.quick.editing.is_none() && modal.should_close() {
        state.quick.open = false;
    }
}

// Taken before the search field draws, so arrows cycle variants instead of moving its cursor.
fn consume_keys(ui: &mut egui::Ui, editing: bool) -> Keys {
    ui.input_mut(|i| {
        if editing {
            return Keys { escape: i.consume_key(Modifiers::NONE, Key::Escape), ..Keys::default() };
        }
        Keys {
            down: i.consume_key(Modifiers::NONE, Key::ArrowDown),
            up: i.consume_key(Modifiers::NONE, Key::ArrowUp),
            left: i.consume_key(Modifiers::NONE, Key::ArrowLeft),
            right: i.consume_key(Modifiers::NONE, Key::ArrowRight),
            enter: i.consume_key(Modifiers::NONE, Key::Enter),
            escape: false,
            add: i.consume_key(Modifiers::COMMAND, Key::N),
        }
    })
}

fn edit_row(ui: &mut egui::Ui, quick: &mut QuickVars, var: &mut EnvironmentVariable) -> bool {
    let (_, edit) = egui::Sides::new().show(
        ui,
        |ui| ui.label(RichText::new(&var.key).monospace().strong()),
        |ui| ui.add(egui::TextEdit::singleline(&mut quick.edit_text).desired_width(260.0)),
    );
    if !edit.lost_focus() {
        edit.request_focus();
        return false;
    }
    set_value(var, std::mem::take(&mut quick.edit_text));
    quick.editing = None;
    true
}

fn value_label(ui: &mut egui::Ui, var: &EnvironmentVariable) {
    let value = if var.r#type == "secret" { "••••••" } else { var.current_value() };
    ui.add(egui::Label::new(RichText::new(value).monospace().weak()).truncate());
    if var.variants.len() > 1 {
        let name = &var.variants[var.active_index.min(var.variants.len() - 1)].name;
        ui.label(RichText::new(format!("‹ {name} ›")).small());
    }
}

fn start_edit(quick: &mut QuickVars, var_idx: usize, var: &EnvironmentVariable) {
    quick.editing = Some(var_idx);
    quick.edit_text = var.current_value().to_string();
}

fn matching(variables: &[EnvironmentVariable], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    (0..variables.len())
        .filter(|&i| query.is_empty() || variables[i].key.to_lowercase().contains(&query))
        .collect()
}

fn new_variable(key: String) -> EnvironmentVariable {
    EnvironmentVariable {
        key,
        value: String::new(),
        enabled: true,
        r#type: "default".to_string(),
        active_index: 0,
        variants: Vec::new(),
    }
}

fn cycle_variant(var: &mut EnvironmentVariable, forward: bool) -> bool {
    let count = var.variants.len();
    if count < 2 {
        return false;
    }
    let current = var.active_index.min(count - 1);
    var.active_index = if forward { (current + 1) % count } else { (current + count - 1) % count };
    var.value = var.variants[var.active_index].value.clone();
    true
}

fn set_value(var: &mut EnvironmentVariable, value: String) {
    if let Some(variant) = var.variants.get_mut(var.active_index) {
        variant.value = value.clone();
    }
    var.value = value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitm_core::workspace::VariableVariant;

    fn with_variants() -> EnvironmentVariable {
        EnvironmentVariable {
            variants: vec![
                VariableVariant { name: "local".to_string(), value: "http://localhost".to_string() },
                VariableVariant { name: "prod".to_string(), value: "https://api.example.com".to_string() },
            ],
            ..new_variable("host".to_string())
        }
    }

    #[test]
    fn cycling_wraps_both_ways_and_skips_single_values() {
        let mut var = with_variants();
        assert!(cycle_variant(&mut var, false));
        assert_eq!(var.current_value(), "https://api.example.com");
        assert!(cycle_variant(&mut var, true));
        assert_eq!(var.value, "http://localhost");
        assert!(!cycle_variant(&mut new_variable("plain".to_string()), true));
    }

    #[test]
    fn setting_a_value_writes_the_active_variant() {
        let mut var = with_variants();
        var.active_index = 1;
        set_value(&mut var, "https://staging.example.com".to_string());
        assert_eq!(var.variants[1].value, "https://staging.example.com");
        assert_eq!(var.current_value(), "https://staging.example.com");
        assert_eq!(var.variants[0].value, "http://localhost");
    }

    #[test]
    fn matching_filters_keys_case_insensitively() {
        let vars = vec![new_variable("BASE_URL".to_string()), new_variable("token".to_string())];
        assert_eq!(matching(&vars, "url"), vec![0]);
        assert_eq!(matching(&vars, ""), vec![0, 1]);
    }
}
