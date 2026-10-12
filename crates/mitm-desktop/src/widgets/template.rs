use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::sync::Arc;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::text::CCursor;
use eframe::egui::text_edit::{TextEditOutput, TextEditState};
use eframe::egui::text_selection::CCursorRange;
use eframe::egui::{self, Color32, Galley, Id, Key, Modifiers, Response, RichText, TextBuffer, TextEdit, TextStyle, Ui};
use mitm_core::workspace::Environment;
use crate::ui::theme::Tone;

const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;

thread_local! {
    static ACTIVE_ENV: RefCell<Option<Environment>> = const { RefCell::new(None) };
}

pub fn set_active_env(env: Option<&Environment>) {
    ACTIVE_ENV.with(|cell| {
        *cell.borrow_mut() = env.cloned();
    });
}

pub fn resolve_var(name: &str) -> Option<String> {
    ACTIVE_ENV.with(|cell| {
        let env_ref = cell.borrow();
        mitm_core::workspace::variable_value_in_env(env_ref.as_ref(), name)
    })
}

fn is_secret(name: &str) -> bool {
    let name = name.trim();
    ACTIVE_ENV.with(|cell| {
        cell.borrow().as_ref().is_some_and(|env| {
            env.variables
                .iter()
                .any(|v| v.enabled && v.r#type == "secret" && v.key.trim().eq_ignore_ascii_case(name))
        })
    })
}

fn totp_params(name: &str) -> Option<mitm_core::utils::totp::TotpParams> {
    ACTIVE_ENV.with(|cell| mitm_core::workspace::totp_params_in_env(cell.borrow().as_ref(), name))
}

fn hover_value(name: &str) -> String {
    if let Some(params) = totp_params(name) {
        use mitm_core::utils::totp;
        let now = totp::now_unix();
        let code = crate::features::utilities::totp::group_code(&totp::code_at(&params, now));
        return format!("{code} · {}s", totp::remaining(params.period, now));
    }
    match resolve_var(name) {
        Some(_) if is_secret(name) => "••••••".to_string(),
        Some(value) => value,
        None => "undefined".to_string(),
    }
}

thread_local! {
    static MARK_PENDING: Cell<bool> = const { Cell::new(false) };
    static DECODE_REQUEST: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Text a template field's **Decode in Encoding tool** asked to open; taken once per frame by the app.
pub fn take_decode_request() -> Option<String> {
    DECODE_REQUEST.with(|c| c.borrow_mut().take())
}

/// The field's current selection, or its whole text when nothing is selected.
fn selected_or_all(ctx: &egui::Context, owner: Id, text: &str) -> String {
    if let Some(range) = TextEditState::load(ctx, owner).and_then(|s| s.cursor.char_range()) {
        let sorted = range.as_sorted_char_range();
        let (start, end) = (sorted.start.0, sorted.end.0);
        if start != end {
            let byte_at = |index: usize| text.char_indices().nth(index).map_or(text.len(), |(b, _)| b);
            return text[byte_at(start)..byte_at(end)].to_string();
        }
    }
    text.to_string()
}

fn decode_menu(response: &Response, owner: Id, text: &str) {
    if text.is_empty() {
        return;
    }
    response.context_menu(|ui| {
        if ui.button("Decode in Encoding tool").clicked() {
            let chosen = selected_or_all(ui.ctx(), owner, text);
            DECODE_REQUEST.with(|c| *c.borrow_mut() = Some(chosen));
            ui.close();
        }
    });
}

/// Ask the focused template field to wrap its selection in `{{ }}` this frame.
pub fn request_mark() {
    MARK_PENDING.with(|c| c.set(true));
}

/// Drop an unconsumed mark request so it never fires on a later frame.
pub fn clear_mark_request() {
    MARK_PENDING.with(|c| c.set(false));
}

pub fn show<'t>(ui: &mut Ui, text: &'t mut String, edit: impl FnOnce(&'t mut String) -> TextEdit<'t>) -> Response {
    // The edit has no id salt, so it takes the next auto id; keys must be consumed before it reads them.
    let owner = ui.next_auto_id();
    let inserted = handle_picker_keys(ui, owner, text);
    let marked = apply_mark(ui, owner, text);
    let output = edit(text).show(ui);
    let mut response = output.response.response.clone();
    if inserted || marked {
        response.mark_changed();
    }
    update_picker(ui, &output);
    let galley_text = output.galley.text();
    let response = hover_tooltip(response, galley_text);
    decode_menu(&response, owner, galley_text);
    response
}

/// Wraps the focused field's selection in `{{ }}`, turning the selected text into a fuzz variable name.
fn apply_mark(ui: &mut Ui, owner: Id, text: &mut String) -> bool {
    if !ui.memory(|m| m.has_focus(owner)) {
        return false;
    }
    let requested = MARK_PENDING.with(|c| c.get()) || ui.input(|i| i.modifiers.command && i.key_pressed(Key::M));
    if !requested {
        return false;
    }
    MARK_PENDING.with(|c| c.set(false));
    let Some(mut state) = TextEditState::load(ui.ctx(), owner) else { return false };
    let Some(range) = state.cursor.char_range() else { return false };
    let sorted = range.as_sorted_char_range();
    let (start, end) = (sorted.start.0, sorted.end.0);
    if start == end {
        return false;
    }
    let byte_at = |index: usize| text.char_indices().nth(index).map_or(text.len(), |(b, _)| b);
    let wrapped = format!("{{{{{}}}}}", &text[byte_at(start)..byte_at(end)]);
    text.replace_range(byte_at(start)..byte_at(end), &wrapped);
    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(end + 4))));
    state.store(ui.ctx(), owner);
    true
}

#[derive(Clone, Debug, PartialEq)]
struct Trigger {
    start: usize,
    end: usize,
    prefix: String,
}

#[derive(Clone)]
struct Picker {
    owner: Id,
    trigger: Trigger,
    matches: Vec<String>,
    selected: usize,
    accept: Option<usize>,
}

/// The `{{` the user pressed Esc on, so the picker stays closed until the cursor leaves it.
#[derive(Clone, Copy, PartialEq)]
struct Dismissed {
    owner: Id,
    start: usize,
}

fn picker_id() -> Id {
    Id::new("template_var_picker")
}

fn handle_picker_keys(ui: &mut Ui, owner: Id, text: &mut String) -> bool {
    let Some(mut picker) = ui.data(|d| d.get_temp::<Picker>(picker_id())) else { return false };
    if picker.owner != owner || (picker.accept.is_none() && !ui.memory(|m| m.has_focus(owner))) {
        return false;
    }
    let (down, up, enter, escape) = ui.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::ArrowUp),
            i.consume_key(Modifiers::NONE, Key::Enter) | i.consume_key(Modifiers::NONE, Key::Tab),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    if escape {
        ui.data_mut(|d| {
            d.remove::<Picker>(picker_id());
            d.insert_temp(picker_id(), Dismissed { owner, start: picker.trigger.start });
        });
        return false;
    }
    let count = picker.matches.len();
    if down {
        picker.selected = (picker.selected + 1) % count;
    }
    if up {
        picker.selected = (picker.selected + count - 1) % count;
    }
    let accept = picker.accept.or(enter.then_some(picker.selected));
    let Some(key) = accept.and_then(|i| picker.matches.get(i)) else {
        ui.data_mut(|d| d.insert_temp(picker_id(), picker));
        return false;
    };
    let cursor = insert_variable(text, &picker.trigger, key);
    let mut state = TextEditState::load(ui.ctx(), owner).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    state.store(ui.ctx(), owner);
    ui.memory_mut(|m| m.request_focus(owner));
    ui.data_mut(|d| d.remove::<Picker>(picker_id()));
    true
}

fn update_picker(ui: &mut Ui, output: &TextEditOutput) {
    let response = &output.response.response;
    let owner = response.id;
    let text = output.galley.text();
    let previous = ui.data(|d| d.get_temp::<Picker>(picker_id())).filter(|p| p.owner == owner);
    // A click on the picker takes focus from the edit this frame; keep the picker up so the click lands.
    let active = response.has_focus() || (previous.is_some() && ui.input(|i| i.pointer.any_released()));
    let trigger = output
        .cursor_range
        .filter(|range| active && range.is_empty())
        .and_then(|range| trigger_at(text, range.primary.index.0));
    let dismissed = ui.data(|d| d.get_temp::<Dismissed>(picker_id()));

    let Some(trigger) = trigger else {
        ui.data_mut(|d| {
            if previous.is_some_and(|p| p.accept.is_none()) {
                d.remove::<Picker>(picker_id());
            }
            if dismissed.is_some_and(|x| x.owner == owner) {
                d.remove::<Dismissed>(picker_id());
            }
        });
        return;
    };
    if dismissed == Some(Dismissed { owner, start: trigger.start }) {
        return;
    }
    let matches = matching_keys(&trigger.prefix);
    if matches.is_empty() {
        if previous.is_some() {
            ui.data_mut(|d| d.remove::<Picker>(picker_id()));
        }
        return;
    }
    let selected = previous
        .as_ref()
        .filter(|p| p.trigger.start == trigger.start)
        .map_or(0, |p| p.selected.min(matches.len() - 1));
    let anchor = output.galley.pos_from_cursor(CCursor::new(trigger.start));
    let mut picker = Picker { owner, trigger, matches, selected, accept: None };

    egui::Area::new(owner.with("var_picker"))
        .order(egui::Order::Foreground)
        .fixed_pos(output.galley_pos + anchor.left_bottom().to_vec2())
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for (index, key) in picker.matches.iter().enumerate() {
                        let selected = index == picker.selected;
                        let item = ui.add(egui::Button::selectable(selected, RichText::new(key).monospace()).frame_when_inactive(false));
                        if selected {
                            item.scroll_to_me(None);
                        }
                        if item.clicked() {
                            picker.accept = Some(index);
                        }
                    }
                });
            });
        });
    if picker.accept.is_some() {
        ui.memory_mut(|m| m.request_focus(owner));
    }
    ui.data_mut(|d| d.insert_temp(picker_id(), picker));
}

fn matching_keys(prefix: &str) -> Vec<String> {
    let prefix = prefix.to_lowercase();
    let mut keys: Vec<String> = ACTIVE_ENV.with(|cell| {
        cell.borrow().as_ref().map_or_else(Vec::new, |env| {
            env.variables
                .iter()
                .filter(|v| v.enabled)
                .map(|v| v.key.trim())
                .filter(|k| !k.is_empty() && k.to_lowercase().contains(&prefix))
                .map(str::to_string)
                .collect()
        })
    });
    keys.dedup();
    keys.sort_by_key(|k| !k.to_lowercase().starts_with(&prefix));
    keys
}

/// The `{{name` the cursor sits in, as char indices; `end` also covers an existing `}}` after the cursor.
fn trigger_at(text: &str, cursor: usize) -> Option<Trigger> {
    let byte = text.char_indices().nth(cursor).map_or(text.len(), |(b, _)| b);
    let before = &text[..byte];
    let open = before.rfind("{{")?;
    let prefix = before[open + 2..].trim_start();
    if prefix.contains(['{', '}']) || prefix.contains(char::is_whitespace) {
        return None;
    }
    let after = &text[byte..];
    let rest = after.find(|c: char| c == '{' || c == '}' || c.is_whitespace()).unwrap_or(after.len());
    let tail = if after[rest..].starts_with("}}") { after[..rest].chars().count() + 2 } else { 0 };
    Some(Trigger {
        start: before[..open].chars().count(),
        end: cursor + tail,
        prefix: prefix.to_string(),
    })
}

/// Replaces the trigger with `{{key}}` and returns the char index just after it.
fn insert_variable(text: &mut String, trigger: &Trigger, key: &str) -> usize {
    let byte_at = |index: usize| text.char_indices().nth(index).map_or(text.len(), |(b, _)| b);
    let range = byte_at(trigger.start)..byte_at(trigger.end);
    text.replace_range(range, &format!("{{{{{key}}}}}"));
    trigger.start + key.chars().count() + 4
}

fn hover_tooltip(resp: Response, text: &str) -> Response {
    if !resp.hovered() || !text.contains("{{") {
        return resp;
    }
    let vars: Vec<String> = spans_with_names(text)
        .into_iter()
        .filter_map(|(_, opt)| opt)
        .collect();
    if vars.is_empty() {
        return resp;
    }

    let mut lines = Vec::new();
    for v in &vars {
        lines.push(format!("{{{{{}}}}}: {}", v, hover_value(v)));
    }
    if vars.iter().any(|v| totp_params(v).is_some()) {
        resp.ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
    resp.on_hover_text(lines.join("\n"))
}

pub fn layouter(style: TextStyle, color: Option<Color32>) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> {
    move |ui, buf, wrap_width| {
        let mut job = layout_job(ui, buf.as_str(), &style, color);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

/// For single-line edits: they scroll sideways, so wrapping to a narrow field would stack the text into a column.
pub fn singleline_layouter(style: TextStyle, color: Option<Color32>) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> {
    move |ui, buf, _wrap_width| ui.fonts_mut(|f| f.layout_job(layout_job(ui, buf.as_str(), &style, color)))
}

fn layout_job(ui: &Ui, text: &str, style: &TextStyle, color: Option<Color32>) -> LayoutJob {
    let visuals = ui.visuals();
    let font = style.resolve(ui.style());
    let plain = TextFormat::simple(
        font.clone(),
        color.or(visuals.override_text_color).unwrap_or_else(|| visuals.widgets.inactive.text_color()),
    );
    let mut job = LayoutJob::default();
    if text.len() > MAX_HIGHLIGHT_BYTES {
        job.append(text, 0.0, plain);
        return job;
    }
    for (range, var_name) in spans_with_names(text) {
        if let Some(name) = var_name {
            let is_known = resolve_var(&name).is_some();
            let tone = if is_known { Tone::Info } else { Tone::Danger };
            let variable = TextFormat {
                font_id: font.clone(),
                color: tone.color(visuals),
                background: tone.tint(visuals, 30),
                ..Default::default()
            };
            job.append(&text[range], 0.0, variable);
        } else {
            job.append(&text[range], 0.0, plain.clone());
        }
    }
    job
}

pub fn spans_with_names(text: &str) -> Vec<(Range<usize>, Option<String>)> {
    let mut spans = Vec::new();
    let mut plain_start = 0;
    let mut cursor = 0;
    while let Some(open) = text[cursor..].find("{{").map(|i| cursor + i) {
        let Some(close) = text[open + 2..].find("}}").map(|i| open + 2 + i) else { break };
        let name = &text[open + 2..close];
        if name.trim().is_empty() || name.contains(['{', '\n']) {
            cursor = open + 1;
            continue;
        }
        if plain_start < open {
            spans.push((plain_start..open, None));
        }
        let trimmed_name = name.trim().to_string();
        spans.push((open..close + 2, Some(trimmed_name)));
        plain_start = close + 2;
        cursor = plain_start;
    }
    if plain_start < text.len() || spans.is_empty() {
        spans.push((plain_start..text.len(), None));
    }
    spans
}

#[cfg(test)]
fn spans(text: &str) -> Vec<(Range<usize>, bool)> {
    spans_with_names(text)
        .into_iter()
        .map(|(r, opt)| (r, opt.is_some()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variables(text: &str) -> Vec<&str> {
        spans(text).into_iter().filter(|(_, v)| *v).map(|(r, _)| &text[r]).collect()
    }

    #[test]
    fn finds_variables_and_keeps_every_byte() {
        let text = "{{BASE_URL}}/users/{{ id }}?q=é";
        let all: String = spans(text).into_iter().map(|(r, _)| &text[r]).collect();
        assert_eq!(all, text);
        assert_eq!(variables(text), ["{{BASE_URL}}", "{{ id }}"]);
    }

    #[test]
    fn ignores_empty_unclosed_and_nested_braces() {
        assert!(variables("{{}} and {{  }} and {{open").is_empty());
        assert_eq!(variables("{{{{x}}"), ["{{x}}"]);
        assert!(variables("{{a\nb}}").is_empty());
    }

    #[test]
    fn test_known_and_unknown_variables() {
        let env = Environment {
            id: "e1".to_string(),
            workspace_id: "w1".to_string(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![
                mitm_core::workspace::EnvironmentVariable {
                    key: "BASE_URL".to_string(),
                    value: "https://example.com".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        set_active_env(Some(&env));
        assert_eq!(resolve_var("BASE_URL").as_deref(), Some("https://example.com"));
        assert_eq!(resolve_var("MISSING"), None);
        assert!(resolve_var("$guid").is_some());
    }

    #[test]
    fn trigger_finds_the_open_variable_before_the_cursor() {
        let text = "https://{{BA/x";
        assert_eq!(trigger_at(text, 12), Some(Trigger { start: 8, end: 12, prefix: "BA".to_string() }));
        assert_eq!(trigger_at("a {{", 4), Some(Trigger { start: 2, end: 4, prefix: String::new() }));
        assert_eq!(trigger_at("{{a}} b", 7), None);
        assert_eq!(trigger_at("{{a b", 5), None);
        assert_eq!(trigger_at("no braces", 3), None);
    }

    #[test]
    fn trigger_covers_an_existing_close_after_the_cursor() {
        let text = "é{{BA_URL}}/x";
        let trigger = trigger_at(text, 5).unwrap();
        assert_eq!(trigger, Trigger { start: 1, end: 11, prefix: "BA".to_string() });
    }

    #[test]
    fn inserting_replaces_the_trigger_and_returns_the_cursor_after_it() {
        let mut text = "é{{BA_URL}}/x".to_string();
        let trigger = trigger_at(&text, 5).unwrap();
        let cursor = insert_variable(&mut text, &trigger, "BASE_URL");
        assert_eq!(text, "é{{BASE_URL}}/x");
        assert_eq!(cursor, 13);

        let mut text = "get {{ho".to_string();
        let trigger = trigger_at(&text, 8).unwrap();
        assert_eq!(insert_variable(&mut text, &trigger, "host"), 12);
        assert_eq!(text, "get {{host}}");
    }

    #[test]
    fn secret_values_are_masked_in_hovers() {
        let mut env = Environment {
            id: "e1".to_string(),
            workspace_id: "w1".to_string(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![mitm_core::workspace::EnvironmentVariable {
                key: "TOKEN".to_string(),
                value: "hunter2".to_string(),
                enabled: true,
                r#type: "secret".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        set_active_env(Some(&env));
        assert_eq!(hover_value("token"), "••••••");
        env.variables[0].r#type = "default".to_string();
        set_active_env(Some(&env));
        assert_eq!(hover_value("TOKEN"), "hunter2");
        assert_eq!(hover_value("MISSING"), "undefined");
    }

    #[test]
    fn totp_refs_hover_as_a_live_code() {
        let env = Environment {
            id: "e1".to_string(),
            workspace_id: "w1".to_string(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![mitm_core::workspace::EnvironmentVariable {
                key: "adminTotpSecret".to_string(),
                value: "MZXW6".to_string(),
                enabled: true,
                r#type: "secret".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        set_active_env(Some(&env));
        let hover = hover_value("$totp:adminTotpSecret");
        assert!(hover.ends_with('s') && hover.contains(" · "), "{hover}");
        assert_eq!(hover.split(" · ").next().unwrap().len(), 7);
        assert_eq!(hover_value("adminTotpSecret"), "••••••");
        assert_eq!(hover_value("$totp:missing"), "undefined");
    }
}
