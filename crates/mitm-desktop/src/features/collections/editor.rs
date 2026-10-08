use eframe::egui::{self, RichText};
use mitm_core::collections::{CollectionTreeItem, RequestItem};
use crate::command::{AppCommand, Commands};
use crate::features::request_editor::body::{convert, is_multipart, is_urlencoded, COLLECTION_BODY_TYPES};
use crate::features::request_editor::kv_table::{self, KvLabels};
use crate::features::request_editor::method::method_combo;
use crate::features::request_editor::steps::{self, StepOption};
use crate::features::request_editor::{body_editor, extract_rules};
use crate::theme::{self, Tone};
use crate::widgets::template;
use super::{docs, CollectionsCmd, CollectionsState, DocsMode, EditorTab, PreviewMode};


pub fn show(ui: &mut egui::Ui, state: &mut CollectionsState, cmds: &mut Commands, now: f64) {
    let Some(index) = state.active_id.as_ref().and_then(|id| state.open.iter().position(|r| &r.id == id)) else {
        return;
    };
    let id = state.open[index].id.clone();
    let step_options = step_options(&state.trees, &id, "");
    let executing = state.executing.contains(&id);
    let request = &mut state.open[index];
    let mut changed = false;
    let mut send = false;

    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        changed |= method_combo(ui, "collection_method", &mut request.method);
        let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
        let width = ui.available_width() - 130.0;
        let url = template::show(ui, &mut request.url, |text| {
            egui::TextEdit::singleline(text)
                .hint_text("https://api.example.com/v1/resource or {{BASE_URL}}/endpoint")
                .layouter(&mut layouter)
                .font(egui::TextStyle::Monospace)
                .desired_width(width)
        });
        changed |= url.changed();
        send |= url.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let label = if executing { "Sending..." } else { "▶ Send" };
        let button = egui::Button::new(RichText::new(label).strong()).fill(ui.visuals().selection.bg_fill);
        send |= ui.add_enabled(!executing, button).on_hover_text("Ctrl+Enter").clicked();
        let history_hint = if state.prefs.history_open { "Hide run history" } else { "Show run history" };
        if ui.selectable_label(state.prefs.history_open, "🕘").on_hover_text(history_hint).clicked() {
            state.prefs.history_open = !state.prefs.history_open;
        }
    });
    let hotkey = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
    send |= hotkey && ui.ui_contains_pointer();
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        let docs_label = if request.description.as_deref().is_some_and(|d| !d.is_empty()) { "Docs •" } else { "Docs" };
        let tabs = [
            (EditorTab::Docs, docs_label.to_string()),
            (EditorTab::Params, format!("Params ({})", request.params.len())),
            (EditorTab::Headers, format!("Headers ({})", request.headers.len())),
            (EditorTab::Body, "Body".to_string()),
            (EditorTab::Extract, format!("Extract Rules ({})", request.extract_rules.len())),
            (EditorTab::Steps, format!("Pre/Post ({})", request.pre_requests.len() + request.post_requests.len())),
            (EditorTab::Interpolation, "Interpolation".to_string()),
        ];
        for (tab, label) in tabs {
            if ui.selectable_label(state.editor_tab == tab, label).clicked() {
                state.editor_tab = tab;
                if tab == EditorTab::Interpolation {
                    cmds.push(AppCommand::Collections(CollectionsCmd::RefreshPreview(id.clone())));
                }
            }
        }
    });
    ui.separator();

    match state.editor_tab {
        EditorTab::Docs => changed |= docs_tab(ui, request, &mut state.docs_mode),
        EditorTab::Body => changed |= body_tab(ui, request),
        EditorTab::Interpolation => interpolation_tab(ui, state, &id, cmds),
        other => {
            egui::ScrollArea::vertical().id_salt("collection_editor").auto_shrink([false, false]).show(ui, |ui| match other {
                EditorTab::Params => {
                    let labels = KvLabels { key: "URL Param Key", value: "Value" };
                    changed |= kv_table::show(ui, "collection_params", &mut request.params, labels, &[]);
                }
                EditorTab::Headers => {
                    let labels = KvLabels { key: "Header Name (e.g. Authorization)", value: "Header Value (e.g. Bearer {{TOKEN}})" };
                    changed |= kv_table::show(ui, "collection_headers", &mut request.headers, labels, &[]);
                }
                EditorTab::Extract => changed |= extract_rules::show(ui, "collection_rules", &mut request.extract_rules),
                EditorTab::Steps => {
                    changed |= steps::show(ui, &mut request.pre_requests, &mut request.post_requests, &step_options)
                }
                _ => {}
            });
        }
    }

    if changed {
        state.mark_dirty(&id, now);
    }
    if send && !executing {
        cmds.push(AppCommand::Collections(CollectionsCmd::Execute(id)));
    }
}

fn step_options(trees: &[CollectionTreeItem], exclude: &str, prefix: &str) -> Vec<StepOption> {
    trees
        .iter()
        .flat_map(|folder| {
            let path = format!("{}{} / ", prefix, folder.name);
            let own = folder.requests.iter().filter(|r| r.id != exclude).map(|r| StepOption {
                id: r.id.clone(),
                label: format!("{}{} ({})", path, r.name, r.method),
            });
            own.chain(step_options(&folder.children, exclude, &path)).collect::<Vec<_>>()
        })
        .collect()
}

fn docs_tab(ui: &mut egui::Ui, request: &mut RequestItem, mode: &mut DocsMode) -> bool {
    let description = request.description.get_or_insert_with(String::new);
    let mut changed = false;

    ui.horizontal(|ui| {
        for (value, label) in [(DocsMode::Preview, "PREVIEW"), (DocsMode::Edit, "EDIT"), (DocsMode::Split, "SPLIT")] {
            ui.selectable_value(mode, value, label);
        }
        if !description.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("📋 Copy MD").clicked() {
                    ui.ctx().copy_text(description.clone());
                }
            });
        }
    });
    ui.separator();

    let preview = |ui: &mut egui::Ui, text: &str, mode: &mut DocsMode| {
        egui::ScrollArea::vertical().id_salt("docs_preview").auto_shrink([false, false]).show(ui, |ui| {
            if text.trim().is_empty() {
                ui.label(RichText::new("No documentation yet").weak().italics());
                if ui.link("Write some").clicked() {
                    *mode = DocsMode::Edit;
                }
            } else {
                docs::markdown(ui, text);
            }
        });
    };

    match *mode {
        DocsMode::Preview => preview(ui, description, mode),
        DocsMode::Edit => changed |= body_editor::code_editor(ui, "docs_edit", description),
        DocsMode::Split => {
            ui.columns(2, |columns| {
                changed |= body_editor::code_editor(&mut columns[0], "docs_split", description);
                preview(&mut columns[1], description, mode);
            });
        }
    }
    changed
}

fn body_buffer(request: &mut RequestItem) -> Option<&mut String> {
    let (slot, default) = match request.body_type.as_str() {
        "json" => (&mut request.body_json, "{\n  \n}"),
        "raw" => (&mut request.body_raw, ""),
        t if is_multipart(t) => (&mut request.body_form_data, "[]"),
        t if is_urlencoded(t) => (&mut request.body_urlencoded, "[]"),
        _ => return None,
    };
    Some(slot.get_or_insert_with(|| default.to_string()))
}

fn body_tab(ui: &mut egui::Ui, request: &mut RequestItem) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (value, label) in COLLECTION_BODY_TYPES {
            let selected = match value {
                "form-data" => is_multipart(&request.body_type),
                "urlencoded" => is_urlencoded(&request.body_type),
                other => request.body_type == other,
            };
            if ui.radio(selected, label).clicked() && !selected {
                request.body_type = value.to_string();
                changed = true;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed |= convert_menu(ui, request);
        });
    });
    ui.separator();

    let body_type = request.body_type.clone();
    match body_buffer(request) {
        Some(content) => changed |= body_editor::show(ui, "collection_body", &body_type, content),
        None => {
            ui.label(RichText::new("This request has no body").weak().italics());
        }
    }
    changed
}

fn convert_menu(ui: &mut egui::Ui, request: &mut RequestItem) -> bool {
    let from = match request.body_type.as_str() {
        t if is_multipart(t) => "form-data".to_string(),
        t if is_urlencoded(t) => "urlencoded".to_string(),
        other => other.to_string(),
    };
    let targets: &[(&str, &str)] = match from.as_str() {
        "json" => &[("form-data", "JSON → form-data"), ("urlencoded", "JSON → x-www-form-urlencoded")],
        "raw" => &[("json", "Raw → JSON")],
        "form-data" => &[("json", "form-data → JSON"), ("urlencoded", "form-data → x-www-form-urlencoded")],
        "urlencoded" => &[("json", "x-www-form-urlencoded → JSON"), ("form-data", "x-www-form-urlencoded → form-data")],
        _ => &[],
    };
    if targets.is_empty() {
        return false;
    }

    let mut picked = None;
    ui.menu_button("Convert ⏷", |ui| {
        for (to, label) in targets {
            if ui.button(*label).clicked() {
                picked = Some(*to);
                ui.close();
            }
        }
    });
    let Some(to) = picked else { return false };
    let source = body_buffer(request).cloned().unwrap_or_default();
    let converted = convert(&from, to, &source);
    request.body_type = to.to_string();
    if let Some(target) = body_buffer(request) {
        *target = converted;
    }
    true
}

fn interpolation_tab(ui: &mut egui::Ui, state: &mut CollectionsState, id: &str, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        for (mode, label) in [(PreviewMode::Request, "request"), (PreviewMode::FullUrl, "custom"), (PreviewMode::Curl, "cURL")] {
            ui.selectable_value(&mut state.preview_mode, mode, label);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("⟳ Refresh").clicked() {
                cmds.push(AppCommand::Collections(CollectionsCmd::RefreshPreview(id.to_string())));
            }
            if let Some(Ok(preview)) = state.previews.get(id) {
                let text = match state.preview_mode {
                    PreviewMode::Request => &preview.full_request,
                    PreviewMode::FullUrl => &preview.full_url_request,
                    PreviewMode::Curl => &preview.curl_command,
                };
                if ui.small_button("📋 Copy").clicked() {
                    ui.ctx().copy_text(text.clone());
                }
            }
        });
    });
    ui.separator();

    match state.previews.get(id) {
        Some(Ok(preview)) => {
            let text = match state.preview_mode {
                PreviewMode::Request => &preview.full_request,
                PreviewMode::FullUrl => &preview.full_url_request,
                PreviewMode::Curl => &preview.curl_command,
            };
            egui::ScrollArea::both().id_salt("collection_preview").auto_shrink([false, false]).show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(text.replace("\r\n", "\n")).monospace()).extend());
            });
        }
        Some(Err(error)) => {
            ui.colored_label(theme::color(ui, Tone::Danger), error);
        }
        None => {
            ui.label(RichText::new("Resolving variables...").weak().italics());
        }
    }
}
