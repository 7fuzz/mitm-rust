use eframe::egui::{self, Align2, Key, Modifiers, RichText};
use crate::command::Commands;
use crate::shortcuts::{self, Action};

#[derive(Default)]
pub struct PaletteState {
    pub open: bool,
    query: String,
    selected: usize,
}

impl PaletteState {
    pub fn open(&mut self) {
        *self = Self { open: true, ..Default::default() };
    }
}

pub fn show(ctx: &egui::Context, state: &mut PaletteState, cmds: &mut Commands) {
    if !state.open {
        return;
    }
    let matches = matching(&state.query);
    state.selected = state.selected.min(matches.len().saturating_sub(1));

    let id = egui::Id::new("command_palette");
    let area = egui::Modal::default_area(id).anchor(Align2::CENTER_TOP, egui::vec2(0.0, 80.0));
    let mut chosen = None;
    let modal = egui::Modal::new(id).area(area).show(ctx, |ui| {
        ui.set_width(420.0);
        let search = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .hint_text("Type a command")
                .desired_width(f32::INFINITY),
        );
        search.request_focus();
        if search.changed() {
            state.selected = 0;
        }
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                state.selected = (state.selected + 1).min(matches.len().saturating_sub(1));
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                state.selected = state.selected.saturating_sub(1);
            }
            if i.consume_key(Modifiers::NONE, Key::Enter) {
                chosen = matches.get(state.selected).copied();
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (index, action) in matches.iter().enumerate() {
                let selected = index == state.selected;
                let response = egui::Sides::new().show(
                    ui,
                    |ui| ui.add(egui::Button::selectable(selected, action.label()).frame_when_inactive(false)),
                    |ui| {
                        if let Some(shortcut) = shortcuts::shortcut_for(*action) {
                            ui.label(RichText::new(ui.ctx().format_shortcut(&shortcut)).weak().monospace());
                        }
                    },
                );
                if selected {
                    response.0.scroll_to_me(None);
                }
                if response.0.clicked() {
                    chosen = Some(*action);
                }
            }
        });
    });

    if let Some(action) = chosen {
        cmds.push(action.command());
        state.open = false;
    } else if modal.should_close() {
        state.open = false;
    }
}

fn matching(query: &str) -> Vec<Action> {
    let query = query.trim().to_lowercase();
    let mut scored: Vec<(usize, Action)> = shortcuts::palette_actions()
        .into_iter()
        .filter_map(|action| fuzzy_score(&action.label().to_lowercase(), &query).map(|score| (score, action)))
        .collect();
    scored.sort_by_key(|(score, _)| *score);
    scored.into_iter().map(|(_, action)| action).collect()
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
}
