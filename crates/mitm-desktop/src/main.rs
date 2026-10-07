use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex};
use mitm_core::events::CoreEvent;
use mitm_core::MitmCore;

mod state;
mod views;

use state::UiState;
use views::inspector::render_inspector_view;
use views::intercept::render_intercept_view;
use views::settings::render_settings_view;
use views::traffic::render_traffic_view;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TabKind {
    Traffic,
    Inspector,
    Repeater,
    Intercept,
    Settings,
}

struct TabViewerApp<'a> {
    state: &'a mut UiState,
    db_path: &'a std::path::PathBuf,
}

impl<'a> egui_dock::TabViewer for TabViewerApp<'a> {
    type Tab = TabKind;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(&*tab)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            TabKind::Traffic => format!("Traffic ({})", self.state.flows.len()).into(),
            TabKind::Inspector => "Inspector".into(),
            TabKind::Repeater => "Repeater".into(),
            TabKind::Intercept => {
                let count = self.state.pending_intercepts.len();
                if count > 0 {
                    format!("Intercept ({})", count).into()
                } else {
                    "Intercept".into()
                }
            }
            TabKind::Settings => "Settings".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            TabKind::Traffic => {
                render_traffic_view(ui, self.state, self.db_path);
            }
            TabKind::Inspector => {
                render_inspector_view(ui, self.state);
            }
            TabKind::Repeater => {
                ui.heading("Repeater");
                ui.separator();
                ui.label("Send custom HTTP requests and view raw responses.");
            }
            TabKind::Intercept => {
                render_intercept_view(ui, self.state);
            }
            TabKind::Settings => {
                render_settings_view(ui, self.state);
            }
        }
    }
}

pub struct NativeMitmApp {
    dock_state: DockState<TabKind>,
    ui_state: UiState,
    core: Arc<MitmCore>,
    rt: Arc<tokio::runtime::Runtime>,
    event_rx: Receiver<CoreEvent>,
    proxy_stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl NativeMitmApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to initialize Tokio runtime"));
        let _guard = rt.enter();

        let core = Arc::new(MitmCore::init().expect("Failed to initialize mitm-core"));

        let mut ui_state = UiState::new();
        ui_state.load_history_from_db(&core.db_path);
        if let Ok(rules) = mitm_core::db::load_intercept_rules(&core.db_path) {
            ui_state.intercept_rules = rules;
        }

        // Bridge Tokio broadcast channel to sync crossbeam channel
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<CoreEvent>();
        let mut core_rx = core.event_bus.subscribe();
        let egui_ctx = cc.egui_ctx.clone();

        rt.spawn(async move {
            while let Ok(event) = core_rx.recv().await {
                let _ = event_tx.send(event);
                egui_ctx.request_repaint();
            }
        });

        // Default layout: Traffic on top/left, Inspector on right, bottom tabs for Repeater/Settings
        let mut dock_state = DockState::new(vec![TabKind::Traffic]);
        let [_traffic_node, inspector_node] =
            dock_state.main_surface_mut().split_right(NodeIndex::root(), 0.55, vec![TabKind::Inspector]);
        let [_inspector_node, _tools_node] =
            dock_state.main_surface_mut().split_below(inspector_node, 0.5, vec![TabKind::Repeater, TabKind::Intercept, TabKind::Settings]);

        Self {
            dock_state,
            ui_state,
            core,
            rt,
            event_rx,
            proxy_stop_tx: None,
        }
    }

    fn toggle_proxy(&mut self) {
        if self.ui_state.proxy_running {
            if let Some(stop_tx) = self.proxy_stop_tx.take() {
                let _ = stop_tx.send(());
            }
            self.ui_state.proxy_running = false;
        } else {
            let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
            self.proxy_stop_tx = Some(stop_tx);
            let core = Arc::clone(&self.core);
            let port = self.ui_state.proxy_port;
            let label = self.ui_state.proxy_label.clone();
            self.ui_state.proxy_running = true;
            self.ui_state.proxy_error = None;

            self.rt.spawn(async move {
                if let Err(e) = core.start_proxy(port, label, stop_rx).await {
                    eprintln!("Proxy server stopped with error: {}", e);
                }
            });
        }
    }
}

impl eframe::App for NativeMitmApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 1. Drain incoming events from background Tokio thread
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                CoreEvent::TrafficCaptured(ev) => {
                    self.ui_state.add_flow(ev.entry);
                }
                CoreEvent::TrafficUpdated { id, status, size, duration_ms } => {
                    self.ui_state.update_flow(id, status, size, duration_ms);
                }
                CoreEvent::TrafficCleared => {
                    self.ui_state.clear_flows();
                }
                CoreEvent::InterceptPrompt(payload) => {
                    self.ui_state.add_pending_intercept(payload);
                }
                CoreEvent::InterceptResolved { flow_id } => {
                    self.ui_state.remove_pending_intercept(&flow_id);
                }
                _ => {}
            }
        }

        // 2. Top status/control bar
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("MITM Native Inspector");
                ui.separator();

                let status_text = if self.ui_state.proxy_running {
                    egui::RichText::new("● Running").color(egui::Color32::GREEN).strong()
                } else {
                    egui::RichText::new("○ Stopped").color(egui::Color32::GRAY)
                };
                ui.label(status_text);

                let button_label = if self.ui_state.proxy_running {
                    "⏹ Stop Proxy"
                } else {
                    "▶ Start Proxy"
                };

                if ui.button(button_label).clicked() {
                    self.toggle_proxy();
                }

                ui.separator();
                ui.label(format!("Port: {}", self.ui_state.proxy_port));

                if !self.ui_state.pending_intercepts.is_empty() {
                    ui.separator();
                    let badge = egui::RichText::new(format!("⚠️ {} Paused Flow(s)", self.ui_state.pending_intercepts.len()))
                        .color(egui::Color32::from_rgb(245, 158, 11))
                        .strong();
                    ui.label(badge);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{} flows in memory", self.ui_state.flows.len())).weak());
                });
            });
        });

        // 3. Docking area fills remaining space
        let mut tab_viewer = TabViewerApp {
            state: &mut self.ui_state,
            db_path: &self.core.db_path,
        };
        DockArea::new(&mut self.dock_state).show_inside(ui, &mut tab_viewer);

        // 4. Drain queued intercept actions
        while let Some(cmd) = self.ui_state.intercept_action_queue.pop() {
            match cmd {
                crate::state::InterceptActionCmd::Forward { flow_id, url, method, headers, body } => {
                    let _ = self.core.forward_intercepted_flow(&flow_id, url, method, headers, body);
                    self.ui_state.remove_pending_intercept(&flow_id);
                }
                crate::state::InterceptActionCmd::Drop { flow_id } => {
                    let _ = self.core.drop_intercepted_flow(&flow_id);
                    self.ui_state.remove_pending_intercept(&flow_id);
                }
                crate::state::InterceptActionCmd::ToggleMaster { enabled, mode } => {
                    let core = Arc::clone(&self.core);
                    self.rt.spawn(async move {
                        let _ = core.toggle_interceptor(enabled, mode).await;
                    });
                }
                crate::state::InterceptActionCmd::SaveRules { rules } => {
                    let core = Arc::clone(&self.core);
                    self.rt.spawn(async move {
                        let _ = core.update_intercept_rules(rules).await;
                    });
                }
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("MITM Native Inspector"),
        ..Default::default()
    };

    eframe::run_native(
        "MITM Native Inspector",
        native_options,
        Box::new(|cc| Ok(Box::new(NativeMitmApp::new(cc)))),
    )
}
