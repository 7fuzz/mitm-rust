use eframe::egui;
use mitm_core::state::{ListenerConfig, SourceScope};

/// "All" or a pick of listeners; deselecting the last listener falls back to All.
pub fn chips(ui: &mut egui::Ui, scope: &mut SourceScope, listeners: &[ListenerConfig]) -> bool {
    let mut changed = false;
    if ui.selectable_label(scope.all, "All sources").clicked() && !scope.all {
        *scope = SourceScope::default();
        changed = true;
    }
    for listener in listeners {
        let picked = !scope.all && scope.ids.contains(&listener.id);
        if ui.selectable_label(picked, &listener.label).on_hover_text(&listener.address).clicked() {
            toggle(scope, listener.id);
            changed = true;
        }
    }
    changed
}

fn toggle(scope: &mut SourceScope, id: u32) {
    if scope.all {
        *scope = SourceScope { all: false, ids: vec![id] };
    } else if let Some(index) = scope.ids.iter().position(|&i| i == id) {
        scope.ids.remove(index);
        scope.all = scope.ids.is_empty();
    } else {
        scope.ids.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picking_narrows_and_unpicking_the_last_returns_to_all() {
        let mut scope = SourceScope::default();
        toggle(&mut scope, 1);
        assert!(!scope.all && scope.ids == [1]);
        toggle(&mut scope, 2);
        assert_eq!(scope.ids, [1, 2]);
        toggle(&mut scope, 1);
        toggle(&mut scope, 2);
        assert!(scope.all && scope.ids.is_empty());
    }
}
