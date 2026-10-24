use eframe::egui;
use mitm_core::proxy::listener::ListenerConfig;
use mitm_core::proxy::rules::SourceScope;

/// "All" or a pick of listeners and containers; deselecting the last pick falls back to All.
pub fn chips(ui: &mut egui::Ui, scope: &mut SourceScope, listeners: &[ListenerConfig], containers: &[String]) -> bool {
    let mut changed = false;
    if ui.selectable_label(scope.all, "All sources").clicked() && !scope.all {
        *scope = SourceScope::default();
        changed = true;
    }
    for listener in listeners {
        let picked = !scope.all && scope.ids.contains(&listener.id);
        if ui.selectable_label(picked, &listener.label).on_hover_text(&listener.address).clicked() {
            toggle(scope, |s| &mut s.ids, listener.id);
            changed = true;
        }
    }
    let unseen = scope.containers.iter().filter(|c| !containers.contains(c)).cloned();
    for label in containers.iter().cloned().chain(unseen.collect::<Vec<_>>()) {
        let picked = !scope.all && scope.containers.contains(&label);
        if ui.selectable_label(picked, &label).on_hover_text("Container").clicked() {
            toggle(scope, |s| &mut s.containers, label);
            changed = true;
        }
    }
    changed
}

fn toggle<T: PartialEq>(scope: &mut SourceScope, list: fn(&mut SourceScope) -> &mut Vec<T>, item: T) {
    if scope.all {
        *scope = SourceScope { all: false, ids: Vec::new(), containers: Vec::new() };
    }
    let items = list(scope);
    if let Some(index) = items.iter().position(|i| *i == item) {
        items.remove(index);
    } else {
        items.push(item);
    }
    scope.all = scope.ids.is_empty() && scope.containers.is_empty();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picking_narrows_and_unpicking_the_last_returns_to_all() {
        let mut scope = SourceScope::default();
        toggle(&mut scope, |s| &mut s.ids, 1);
        assert!(!scope.all && scope.ids == [1]);
        toggle(&mut scope, |s| &mut s.ids, 2);
        assert_eq!(scope.ids, [1, 2]);
        toggle(&mut scope, |s| &mut s.ids, 1);
        toggle(&mut scope, |s| &mut s.ids, 2);
        assert!(scope.all && scope.ids.is_empty());
    }

    #[test]
    fn containers_and_listeners_combine_and_the_last_unpick_returns_to_all() {
        let mut scope = SourceScope::default();
        toggle(&mut scope, |s| &mut s.containers, "App A".to_string());
        toggle(&mut scope, |s| &mut s.ids, 1);
        assert!(!scope.all && scope.containers == ["App A"] && scope.ids == [1]);
        toggle(&mut scope, |s| &mut s.ids, 1);
        assert!(!scope.all);
        toggle(&mut scope, |s| &mut s.containers, "App A".to_string());
        assert!(scope.all);
    }
}
