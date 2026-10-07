use std::path::PathBuf;
use eframe::egui::{self, Color32, RichText};
use mitm_core::collections::{CollectionTreeItem, RequestItem};
use crate::state::{RepeaterTabState, UiState};
use crate::views::pills::render_method_pill;

pub fn render_collections_view(ui: &mut egui::Ui, state: &mut UiState, db_path: &PathBuf) {
    ui.heading("Workspaces & Collections");
    ui.add_space(4.0);

    // 1. Workspace switcher
    let mut ws_changed = false;
    let mut reload_clicked = false;

    ui.horizontal(|ui| {
        ui.strong("Workspace:");

        let current_name = state
            .workspaces
            .iter()
            .find(|w| w.id == state.active_workspace_id)
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "Default Workspace".to_string());

        egui::ComboBox::from_id_salt("workspace_selector_combo")
            .selected_text(&current_name)
            .show_ui(ui, |ui| {
                for ws in &state.workspaces {
                    if ui.selectable_value(&mut state.active_workspace_id, ws.id.clone(), &ws.name).clicked() {
                        ws_changed = true;
                    }
                }
            });

        if ui.button("🔄 Reload").clicked() {
            reload_clicked = true;
        }
    });

    if ws_changed || reload_clicked {
        state.load_workspaces_and_collections(db_path);
    }

    ui.separator();

    // 2. Tree rendering
    if state.collection_trees.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("No collections found in this workspace.").weak());
        });
        return;
    }

    let mut send_request_to_repeater: Option<RequestItem> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for collection in &state.collection_trees {
            render_collection_node(ui, collection, &mut send_request_to_repeater);
        }
    });

    if let Some(req) = send_request_to_repeater {
        let headers_str = req
            .headers
            .iter()
            .map(|h| format!("{}: {}", h.key, h.value))
            .collect::<Vec<_>>()
            .join("\n");

        let body_str = req
            .body_raw
            .or(req.body_json)
            .or(req.body_form_data)
            .unwrap_or_default();

        let tab = RepeaterTabState {
            id: uuid::Uuid::new_v4().to_string(),
            name: req.name.clone(),
            method: req.method.clone(),
            url: req.url.clone(),
            headers: headers_str,
            body: body_str,
            is_loading: false,
            last_result: None,
            error: None,
        };
        state.repeater_tabs.push(tab);
        state.active_repeater_index = state.repeater_tabs.len() - 1;
    }
}

fn render_collection_node(
    ui: &mut egui::Ui,
    item: &CollectionTreeItem,
    send_to_repeater: &mut Option<RequestItem>,
) {
    let header_text = format!("📁 {} ({} reqs)", item.name, item.requests.len());

    ui.collapsing(RichText::new(header_text).strong(), |ui| {
        // Render nested sub-collections
        for sub in &item.children {
            render_collection_node(ui, sub, send_to_repeater);
        }

        // Render requests in this folder
        for req in &item.requests {
            ui.horizontal(|ui| {
                render_method_pill(ui, &req.method);
                ui.label(RichText::new(&req.name).color(Color32::from_rgb(229, 231, 235)));
                ui.add(egui::Label::new(RichText::new(&req.url).weak()).truncate());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🔁 Repeater").clicked() {
                        *send_to_repeater = Some(req.clone());
                    }
                });
            });
        }
    });
}
