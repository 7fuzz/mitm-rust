use eframe::egui::{self, Color32, RichText};
use crate::state::{InspectorPhase, InspectorViewMode, UiState};
use crate::views::pills::{format_bytes, format_duration, render_method_pill, render_status_pill};

pub fn render_inspector_view(ui: &mut egui::Ui, state: &mut UiState) {
    let flow = match state.selected_flow() {
        Some(f) => f.clone(),
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("No Request Selected");
                ui.label("Select a request from the Traffic table to inspect headers, payload, and responses.");
            });
            return;
        }
    };

    let detail = state.selected_flow_detail.as_ref();

    // 1. Top bar: Phase selection, Status, Method, Actions
    ui.horizontal(|ui| {
        // Request / Response toggle buttons
        let req_selected = state.inspector_phase == InspectorPhase::Request;
        let res_selected = state.inspector_phase == InspectorPhase::Response;

        if ui.selectable_label(req_selected, "Request").clicked() {
            state.inspector_phase = InspectorPhase::Request;
        }
        if ui.selectable_label(res_selected, "Response").clicked() {
            state.inspector_phase = InspectorPhase::Response;
        }

        ui.separator();

        render_method_pill(ui, &flow.method);
        render_status_pill(ui, flow.status_code);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📋 Copy cURL").on_hover_text("Copy as bash cURL command").clicked() {
                let curl_cmd = build_curl_command(&flow, detail);
                ui.ctx().copy_text(curl_cmd);
            }
            if ui.button("📋 Copy URL").on_hover_text("Copy URL to clipboard").clicked() {
                ui.ctx().copy_text(flow.url.clone());
            }
        });
    });

    // 2. Dedicated URL Address Bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("URL:").weak().size(11.0));
        let mut display_url = flow.url.clone();
        ui.add(
            egui::TextEdit::singleline(&mut display_url)
                .desired_width(ui.available_width())
                .font(egui::TextStyle::Monospace)
        );
    });

    ui.separator();

    // 2. Metadata info line
    ui.horizontal(|ui| {
        ui.label(RichText::new("Duration:").weak());
        ui.label(format_duration(flow.duration_ms));
        ui.separator();

        ui.label(RichText::new("Size:").weak());
        ui.label(format_bytes(flow.response_size));
        ui.separator();

        ui.label(RichText::new("Type:").weak());
        ui.label(&flow.content_type);
        ui.separator();

        if let Some(req_at) = &flow.request_at {
            ui.label(RichText::new("Time:").weak());
            ui.label(req_at);
        }
    });

    ui.separator();

    // 3. Sub-tabs bar: Headers, Formatted, Raw, Hex
    let (headers, body_str) = match state.inspector_phase {
        InspectorPhase::Request => {
            let h = detail.map(|d| &d.request_headers).unwrap_or(&flow.request_headers);
            let b = detail.map(|d| d.request_body.as_str()).unwrap_or(flow.request_body.as_str());
            (h, b)
        }
        InspectorPhase::Response => {
            let h = detail.map(|d| &d.response_headers).unwrap_or(&flow.response_headers);
            let b = detail.map(|d| d.response_body.as_str()).unwrap_or(flow.response_body.as_str());
            (h, b)
        }
    };

    ui.horizontal(|ui| {
        let headers_label = format!("Headers ({})", headers.len());
        if ui.selectable_label(state.inspector_mode == InspectorViewMode::Headers, headers_label).clicked() {
            state.inspector_mode = InspectorViewMode::Headers;
        }

        if ui.selectable_label(state.inspector_mode == InspectorViewMode::Formatted, "Formatted").clicked() {
            state.inspector_mode = InspectorViewMode::Formatted;
        }

        if ui.selectable_label(state.inspector_mode == InspectorViewMode::Raw, "Raw").clicked() {
            state.inspector_mode = InspectorViewMode::Raw;
        }

        if ui.selectable_label(state.inspector_mode == InspectorViewMode::Hex, "Hex Dump").clicked() {
            state.inspector_mode = InspectorViewMode::Hex;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !body_str.is_empty() && ui.button("📋 Copy Body").clicked() {
                ui.ctx().copy_text(body_str.to_string());
            }
        });
    });

    ui.separator();

    // 4. Content Area
    match state.inspector_mode {
        InspectorViewMode::Headers => {
            render_headers_view(ui, &mut state.header_search_query, headers);
        }
        InspectorViewMode::Formatted => {
            render_formatted_view(ui, body_str);
        }
        InspectorViewMode::Raw => {
            render_raw_view(ui, body_str);
        }
        InspectorViewMode::Hex => {
            render_hex_view(ui, body_str);
        }
    }
}

fn render_headers_view(ui: &mut egui::Ui, search_query: &mut String, headers: &[(String, String)]) {
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Search headers...")
                .desired_width(180.0),
        );

        if !search_query.is_empty() && ui.small_button("✖").clicked() {
            search_query.clear();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📋 Copy All").clicked() {
                let all_headers = headers
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");
                ui.ctx().copy_text(all_headers);
            }
        });
    });

    ui.separator();

    let query = search_query.trim().to_lowercase();
    let filtered_headers: Vec<&(String, String)> = headers
        .iter()
        .filter(|(k, v)| {
            if query.is_empty() {
                true
            } else {
                k.to_lowercase().contains(&query) || v.to_lowercase().contains(&query)
            }
        })
        .collect();

    if filtered_headers.is_empty() {
        ui.label(RichText::new("No headers match the filter.").weak());
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("headers_inspector_grid")
            .striped(true)
            .num_columns(3)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Name");
                ui.strong("Value");
                ui.label("");
                ui.end_row();

                for (name, val) in filtered_headers {
                    ui.label(RichText::new(name).color(Color32::from_rgb(147, 197, 253)).strong());
                    ui.label(val);

                    let copy_text = format!("{}: {}", name, val);
                    if ui.small_button("📋").on_hover_text("Copy header line").clicked() {
                        ui.ctx().copy_text(copy_text);
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_formatted_view(ui: &mut egui::Ui, body: &str) {
    if body.is_empty() {
        ui.label(RichText::new("(Empty body)").weak());
        return;
    }

    // Attempt to pretty print JSON
    let formatted_json = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok());

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(json) = formatted_json {
            ui.add(
                egui::TextEdit::multiline(&mut json.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        } else {
            ui.add(
                egui::TextEdit::multiline(&mut body.as_ref())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        }
    });
}

fn render_raw_view(ui: &mut egui::Ui, body: &str) {
    if body.is_empty() {
        ui.label(RichText::new("(Empty body)").weak());
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut body.as_ref())
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
    });
}

fn render_hex_view(ui: &mut egui::Ui, body: &str) {
    if body.is_empty() {
        ui.label(RichText::new("(Empty body)").weak());
        return;
    }

    let hex_dump = generate_hex_dump(body.as_bytes());

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut hex_dump.as_str())
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
    });
}

pub fn generate_hex_dump(bytes: &[u8]) -> String {
    let mut output = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        output.push_str(&format!("{:08x}:  ", i * 16));

        for j in 0..16 {
            if j == 8 {
                output.push(' ');
            }
            if j < chunk.len() {
                output.push_str(&format!("{:02x} ", chunk[j]));
            } else {
                output.push_str("   ");
            }
        }

        output.push_str(" |");
        for &b in chunk {
            if b.is_ascii_graphic() || b == b' ' {
                output.push(b as char);
            } else {
                output.push('.');
            }
        }
        output.push_str("|\n");
    }
    output
}

pub fn build_curl_command(flow: &mitm_core::state::HistoryEntry, detail: Option<&mitm_core::state::HistoryDetail>) -> String {
    let mut curl = format!("curl -X {} '{}'", flow.method, flow.url);
    if let Some(detail) = detail {
        for (k, v) in &detail.request_headers {
            if !k.eq_ignore_ascii_case("content-length") {
                curl.push_str(&format!(" \\\n  -H '{}: {}'", k, v.replace('\'', "'\\''")));
            }
        }
        if !detail.request_body.is_empty() {
            curl.push_str(&format!(" \\\n  --data-raw '{}'", detail.request_body.replace('\'', "'\\''")));
        }
    }
    curl
}
