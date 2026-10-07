use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TabKind {
    Traffic,
    Inspector,
    Repeater,
    Intercept,
    Settings,
}

pub struct AppContext {
    pub proxy_running: bool,
    pub proxy_port: u16,
}

struct TabViewerApp<'a> {
    ctx: &'a mut AppContext,
}

impl<'a> egui_dock::TabViewer for TabViewerApp<'a> {
    type Tab = TabKind;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(&*tab)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            TabKind::Traffic => "Traffic".into(),
            TabKind::Inspector => "Inspector".into(),
            TabKind::Repeater => "Repeater".into(),
            TabKind::Intercept => "Intercept".into(),
            TabKind::Settings => "Settings".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            TabKind::Traffic => {
                ui.heading("Traffic Flows");
                ui.separator();
                ui.label("Captured HTTP/HTTPS requests will appear here.");
            }
            TabKind::Inspector => {
                ui.heading("Request / Response Inspector");
                ui.separator();
                ui.label("Select a flow from the traffic table to inspect headers and body.");
            }
            TabKind::Repeater => {
                ui.heading("Repeater");
                ui.separator();
                ui.label("Send custom HTTP requests and view raw responses.");
            }
            TabKind::Intercept => {
                ui.heading("Interception Rules");
                ui.separator();
                ui.label("Configure breakpoints for requests and responses.");
            }
            TabKind::Settings => {
                ui.heading("Proxy Settings & CA");
                ui.separator();
                ui.label(format!("Active proxy port: {}", self.ctx.proxy_port));
            }
        }
    }
}

pub struct NativeMitmApp {
    dock_state: DockState<TabKind>,
    context: AppContext,
}

impl NativeMitmApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        mitm_core::init();

        // Default layout: Traffic on top/left, Inspector on right, bottom tabs for Repeater/Settings
        let mut dock_state = DockState::new(vec![TabKind::Traffic]);
        let [_traffic_node, inspector_node] =
            dock_state.main_surface_mut().split_right(NodeIndex::root(), 0.55, vec![TabKind::Inspector]);
        let [_inspector_node, _tools_node] =
            dock_state.main_surface_mut().split_below(inspector_node, 0.5, vec![TabKind::Repeater, TabKind::Intercept, TabKind::Settings]);

        Self {
            dock_state,
            context: AppContext {
                proxy_running: false,
                proxy_port: 8080,
            },
        }
    }
}

impl eframe::App for NativeMitmApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Top status/control bar
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("MITM Native Inspector");
                ui.separator();

                let status_text = if self.context.proxy_running {
                    egui::RichText::new("● Running").color(egui::Color32::GREEN)
                } else {
                    egui::RichText::new("○ Stopped").color(egui::Color32::GRAY)
                };
                ui.label(status_text);

                if ui.button(if self.context.proxy_running { "Stop Proxy" } else { "Start Proxy" }).clicked() {
                    self.context.proxy_running = !self.context.proxy_running;
                }

                ui.separator();
                ui.label(format!("Port: {}", self.context.proxy_port));
            });
        });

        // Docking area fills remaining space
        let mut tab_viewer = TabViewerApp { ctx: &mut self.context };
        DockArea::new(&mut self.dock_state).show_inside(ui, &mut tab_viewer);
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
