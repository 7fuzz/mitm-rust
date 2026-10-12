use eframe::egui::{self, CornerRadius, RichText};
use mitm_core::proxy::listener::ProxyMode;
use crate::ui::theme::{self, Tone};
use egui_dock::DockState;
use crate::app::command::{AppCommand, Commands};
use crate::ui::dock::{self, Tab};
use crate::app::state::AppState;
use crate::widgets::toast;
use crate::ui::icons;
use crate::features::settings;
use crate::features::environments::EnvironmentsState;
use crate::util::format::format_count;
use crate::widgets::pills::badge;

/// Reports a background result as a toast.
pub fn flash(ui: &egui::Ui, result: Result<String, String>) {
    toast::push(ui.ctx(), result);
}

const GROUP_GAP: f32 = 16.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, dock: &DockState<Tab>, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        proxy_controls(ui, state, cmds);
        ui.add_space(GROUP_GAP);
        breadcrumb(ui, state, cmds);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            theme_toggle(ui);
            add_panel_menu(ui, dock, cmds);
            let paused = state.intercept.pending.len();
            if paused > 0 {
                ui.add_space(GROUP_GAP);
                let pill = badge(ui, &format!("{} paused", format_count(paused)), Tone::Warn).interact(egui::Sense::click());
                if pill.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Show paused flows").clicked() {
                    cmds.push(AppCommand::OpenTab(Tab::Intercept));
                }
            }
        });
    });

    new_workspace_modal(ui.ctx(), state, cmds);
}

/// Flat text with a caret; the frame only shows on hover.
fn flat_menu(ui: &mut egui::Ui, text: &str, content: impl FnOnce(&mut egui::Ui)) {
    let button = egui::Button::new((text, RichText::new(icons::CARET_DOWN).weak())).frame_when_inactive(false);
    egui::containers::menu::MenuButton::from_button(button).ui(ui, content);
}

fn breadcrumb(ui: &mut egui::Ui, state: &mut AppState, cmds: &mut Commands) {
    let ws_state = &mut state.environments;
    let ws_name = ws_state.active_workspace().map_or("No workspace", |w| w.name.as_str()).to_string();
    let env_name = ws_state.active_environment().map_or("No environment", |e| e.name.as_str()).to_string();
    flat_menu(ui, &ws_name, |ui| workspace_menu(ui, ws_state, cmds));
    ui.weak("/");
    flat_menu(ui, &env_name, |ui| environment_menu(ui, ws_state, cmds));
}

fn workspace_menu(ui: &mut egui::Ui, ws_state: &mut EnvironmentsState, cmds: &mut Commands) {
    if ws_state.workspaces.len() > 8 {
        ui.add(egui::TextEdit::singleline(&mut ws_state.workspace_search).hint_text("Search…"));
        ui.separator();
    }
    let search_lower = ws_state.workspace_search.trim().to_lowercase();
    for ws in &ws_state.workspaces {
        if !search_lower.is_empty() && !ws.name.to_lowercase().contains(&search_lower) {
            continue;
        }
        let is_active = ws.id == ws_state.active_workspace_id;
        if ui.selectable_label(is_active, &ws.name).clicked() && !is_active {
            cmds.push(AppCommand::SelectWorkspace(ws.id.clone()));
            ui.close();
        }
    }
    ui.separator();
    if ui.button("New…").clicked() {
        ws_state.new_workspace_modal = true;
        ws_state.new_workspace_name.clear();
        ui.close();
    }
    if ui.button("Import…").clicked() {
        cmds.push(AppCommand::OpenImport);
        ui.close();
    }
    if ui.button("Export…").clicked() {
        cmds.push(AppCommand::ExportWorkspace);
        ui.close();
    }
    if ui.button("Manage").clicked() {
        cmds.push(AppCommand::OpenTab(Tab::Environments));
        ui.close();
    }
}

fn environment_menu(ui: &mut egui::Ui, ws_state: &EnvironmentsState, cmds: &mut Commands) {
    let is_none_active = ws_state.active_environment_id.is_none();
    if ui.selectable_label(is_none_active, "No environment").clicked() {
        cmds.push(crate::features::environments::Cmd::Select(None));
        ui.close();
    }
    if !ws_state.environments.is_empty() {
        ui.separator();
    }
    for env in &ws_state.environments {
        let is_active = Some(&env.id) == ws_state.active_environment_id.as_ref();
        if ui.selectable_label(is_active, &env.name).clicked() && !is_active {
            cmds.push(crate::features::environments::Cmd::Select(Some(env.id.clone())));
            ui.close();
        }
    }
}

fn new_workspace_modal(ctx: &egui::Context, state: &mut AppState, cmds: &mut Commands) {
    if !state.environments.new_workspace_modal {
        return;
    }
    egui::Modal::new(egui::Id::new("new_workspace_modal")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.heading("New workspace");
        ui.add_space(6.0);
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.environments.new_workspace_name)
                .hint_text("Workspace name"),
        );
        let name = state.environments.new_workspace_name.trim().to_string();
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter && !name.is_empty() {
            cmds.push(AppCommand::CreateWorkspace(name.clone()));
            state.environments.new_workspace_name.clear();
            state.environments.new_workspace_modal = false;
        }
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_create = !name.is_empty();
            if ui.add_enabled(can_create, crate::widgets::button::primary(ui, "Create")).clicked() {
                cmds.push(AppCommand::CreateWorkspace(name));
                state.environments.new_workspace_name.clear();
                state.environments.new_workspace_modal = false;
            }
            if ui.button("Cancel").clicked() {
                state.environments.new_workspace_name.clear();
                state.environments.new_workspace_modal = false;
            }
        });
    });
}

fn proxy_controls(ui: &mut egui::Ui, state: &AppState, cmds: &mut Commands) {
    let proxy = &state.proxy;
    let toggle_hint = if proxy.mode.is_enabled() { "Turn the proxy off" } else { "Turn the proxy on" };
    ui.add_enabled_ui(!proxy.busy(), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let left = CornerRadius { nw: 4, sw: 4, ne: 0, se: 0 };
        let right = CornerRadius { nw: 0, sw: 0, ne: 4, se: 4 };
        let dot = RichText::new("●").color(mode_tone(proxy.mode).color(ui.visuals()));
        if ui.add(egui::Button::new((dot, mode_label(proxy.mode))).corner_radius(left)).on_hover_text(toggle_hint).clicked() {
            cmds.push(settings::Cmd::ToggleProxy);
        }
        let caret = egui::Button::new(icons::CARET_DOWN).corner_radius(right);
        egui::containers::menu::MenuButton::from_button(caret).ui(ui, |ui| {
            for mode in ProxyMode::ALL {
                if ui.selectable_label(proxy.mode == mode, mode_label(mode)).clicked() {
                    cmds.push(settings::Cmd::SetProxyMode(mode));
                    ui.close();
                }
            }
        });
    });
    let problems = proxy.problems();
    if !problems.is_empty() {
        let warning = egui::Label::new(RichText::new(icons::WARNING).color(theme::color(ui, Tone::Danger))).sense(egui::Sense::click());
        if ui.add(warning).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(problems.join("\n")).clicked() {
            cmds.push(AppCommand::OpenSettings(crate::features::settings::SettingsSection::Proxy));
        }
    }
}

pub fn mode_label(mode: ProxyMode) -> &'static str {
    match mode {
        ProxyMode::On => "On",
        ProxyMode::Off => "Off",
        ProxyMode::BlockClient => "Block client",
        ProxyMode::BlockAll => "Block all",
    }
}

fn mode_tone(mode: ProxyMode) -> Tone {
    match mode {
        ProxyMode::On => Tone::Success,
        ProxyMode::Off => Tone::Neutral,
        ProxyMode::BlockClient => Tone::Caution,
        ProxyMode::BlockAll => Tone::Danger,
    }
}

fn theme_toggle(ui: &mut egui::Ui) {
    let (icon, hint, next) = if ui.visuals().dark_mode {
        (icons::SUN, "Light theme", egui::ThemePreference::Light)
    } else {
        (icons::MOON, "Dark theme", egui::ThemePreference::Dark)
    };
    if crate::widgets::button::icon_button(ui, icon, hint).clicked() {
        ui.ctx().set_theme(next);
    }
}

fn add_panel_menu(ui: &mut egui::Ui, dock: &DockState<Tab>, cmds: &mut Commands) {
    let button = egui::Button::new(icons::PLUS).frame_when_inactive(false);
    let (response, _) = egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
        if let Some(tab) = dock::panel_menu(ui, |tab| dock.find_tab(&tab).is_some()) {
            cmds.push(AppCommand::OpenTab(tab));
        }
    });
    response.on_hover_text("Add panel");
}
