use eframe::egui::{self, RichText};
use mitm_core::repeater::RepeaterTab;
use crate::app::command::Commands;
use crate::ui::theme::{self, Tone};
use crate::widgets::pills::{method_text, status_text};
use crate::widgets::search::search_field;
use super::RepeaterState;
use crate::ui::icons;
use crate::widgets::section::section_with;
use crate::widgets::button::icon_button;

pub fn show(ui: &mut egui::Ui, state: &mut RepeaterState, cmds: &mut Commands) {
    section_with(ui, &format!("Requests · {}", state.tabs.len()), |ui| {
        if icon_button(ui, icons::PLUS, "New request").clicked() {
            cmds.push(super::Cmd::New);
        }
    });
    search_field(ui, &mut state.search, "Filter");
    ui.add_space(4.0);

    let term = state.search.trim().to_lowercase();
    let visible: Vec<&RepeaterTab> = state
        .tabs
        .iter()
        .filter(|t| term.is_empty() || format!("{} {}", t.method, t.url).to_lowercase().contains(&term))
        .collect();

    if visible.is_empty() {
        ui.weak(if term.is_empty() { "No requests yet" } else { "No matches" });
        return;
    }

    egui::ScrollArea::vertical().id_salt("repeater_sidebar_list").auto_shrink([false, false]).show(ui, |ui| {
        for tab in visible {
            let active = state.active_id.as_deref() == Some(tab.id.as_str());
            if let Some(cmd) = row(ui, tab, active) {
                cmds.push(cmd);
            }
        }
    });
}

fn row(ui: &mut egui::Ui, tab: &RepeaterTab, active: bool) -> Option<super::Cmd> {
    let style = ui.style().clone();
    let mut title = egui::text::LayoutJob::default();
    method_text(ui.visuals(), &tab.method).append_to(&mut title, &style, egui::FontSelection::Default, egui::Align::Center);
    RichText::new(format!("  {}", url_path(&tab.url)))
        .monospace()
        .append_to(&mut title, &style, egui::FontSelection::Default, egui::Align::Center);
    let host = RichText::new(url_host(&tab.url)).small().weak();
    let status = match tab.last_status_code {
        Some(code) => status_text(ui.visuals(), code),
        None => RichText::new("·").weak(),
    };

    let response = ui
        .add(egui::Button::selectable(active, "").min_size(egui::vec2(ui.available_width(), 36.0)))
        .on_hover_ui(|ui| {
            ui.monospace(&tab.url);
            match (tab.last_status_code, tab.last_duration_ms) {
                (Some(code), Some(ms)) => ui.weak(format!("Last: {} in {}ms · sent {}×", code, ms, tab.execution_count)),
                _ => ui.weak("Not sent yet"),
            };
        });
    paint_row_text(ui, &response, active, title, host, status);

    let mut cmd = None;
    response.context_menu(|ui| {
        if ui.button(RichText::new("Delete").color(theme::color(ui, Tone::Danger))).clicked() {
            cmd = Some(super::Cmd::Delete(tab.id.clone()));
            ui.close();
        }
    });
    if response.clicked() {
        cmd = Some(super::Cmd::Select(tab.id.clone()));
    }
    cmd
}

// Button truncation clamps text to one row, so the two lines are painted over an empty button.
fn paint_row_text(ui: &egui::Ui, response: &egui::Response, active: bool, title: egui::text::LayoutJob, host: RichText, status: RichText) {
    let rect = response.rect.shrink2(ui.spacing().button_padding);
    let color = ui.style().interact_selectable(response, active).text_color();
    let truncated = |text: egui::WidgetText, width: f32| {
        text.into_galley(ui, Some(egui::TextWrapMode::Truncate), width, egui::FontSelection::Default)
    };

    let status = truncated(status.into(), rect.width());
    let gap = ui.spacing().item_spacing.x;
    let title = truncated(title.into(), (rect.width() - status.size().x - gap).max(0.0));
    let host = truncated(host.into(), rect.width());

    let top = rect.top() + (rect.height() - title.size().y - host.size().y) / 2.0;
    let painter = ui.painter();
    painter.galley(egui::pos2(rect.right() - status.size().x, top + (title.size().y - status.size().y) / 2.0), status, color);
    painter.galley(egui::pos2(rect.left(), top + title.size().y), host, color);
    painter.galley(egui::pos2(rect.left(), top), title, color);
}

fn url_host(url: &str) -> &str {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, rest)| rest);
    let authority = rest.find(['/', '?', '#']).map_or(rest, |i| &rest[..i]);
    authority.rsplit_once('@').map_or(authority, |(_, host)| host)
}

fn url_path(url: &str) -> &str {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, rest)| rest);
    rest.find(['/', '?']).map_or(rest, |i| &rest[i..])
}
