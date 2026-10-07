use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex};
use mitm_core::events::CoreEvent;
use mitm_core::MitmCore;

mod state;
mod views;

use state::UiState;
use views::collections::render_collections_view;
use views::fuzzer::render_fuzzer_view;
use views::inspector::render_inspector_view;
use views::intercept::render_intercept_view;
use views::repeater::render_repeater_view;
use views::settings::render_settings_view;
use views::traffic::render_traffic_view;
use views::websockets::render_websockets_view;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TabKind {
    Traffic,
    Inspector,
    Repeater,
    Fuzzer,
    Intercept,
    Collections,
    WebSockets,
    Settings,
}

#[derive(Debug, Clone)]
pub enum FuzzMsg {
    Result(crate::state::FuzzResultRow),
    Done,
}

struct TabViewerApp<'a> {
    state: &'a mut UiState,
    core: &'a Arc<MitmCore>,
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
            TabKind::Repeater => format!("Repeater ({})", self.state.repeater_tabs.len()).into(),
            TabKind::Fuzzer => "Fuzzer".into(),
            TabKind::Collections => "Collections".into(),
            TabKind::WebSockets => format!("WebSockets ({})", self.state.ws_connections.len()).into(),
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
                render_traffic_view(ui, self.state, self.core);
            }
            TabKind::Inspector => {
                render_inspector_view(ui, self.state);
            }
            TabKind::Repeater => {
                render_repeater_view(ui, self.state);
            }
            TabKind::Fuzzer => {
                render_fuzzer_view(ui, self.state);
            }
            TabKind::Collections => {
                render_collections_view(ui, self.state, &self.core.db_path);
            }
            TabKind::WebSockets => {
                render_websockets_view(ui, self.state, &self.core.db_path);
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
    repeater_result_tx: crossbeam_channel::Sender<(usize, Result<mitm_core::repeater::execute::SentRequest, String>)>,
    repeater_result_rx: crossbeam_channel::Receiver<(usize, Result<mitm_core::repeater::execute::SentRequest, String>)>,
    fuzzer_result_tx: crossbeam_channel::Sender<FuzzMsg>,
    fuzzer_result_rx: crossbeam_channel::Receiver<FuzzMsg>,
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
        ui_state.load_workspaces_and_collections(&core.db_path);
        ui_state.load_ws_connections(&core.db_path);

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

        let (repeater_result_tx, repeater_result_rx) = crossbeam_channel::unbounded();
        let (fuzzer_result_tx, fuzzer_result_rx) = crossbeam_channel::unbounded();

        // Default layout: Traffic on top/left, Inspector on right, bottom tabs for Repeater/Fuzzer/Settings
        let mut dock_state = DockState::new(vec![TabKind::Traffic, TabKind::Collections]);
        let [_traffic_node, inspector_node] =
            dock_state.main_surface_mut().split_right(NodeIndex::root(), 0.55, vec![TabKind::Inspector]);
        let [_inspector_node, _tools_node] =
            dock_state.main_surface_mut().split_below(inspector_node, 0.5, vec![
                TabKind::Repeater,
                TabKind::Fuzzer,
                TabKind::Intercept,
                TabKind::WebSockets,
                TabKind::Settings,
            ]);

        Self {
            dock_state,
            ui_state,
            core,
            rt,
            event_rx,
            proxy_stop_tx: None,
            repeater_result_tx,
            repeater_result_rx,
            fuzzer_result_tx,
            fuzzer_result_rx,
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
                    let flow_id = ev.entry.id;
                    let was_selected = self.ui_state.selected_flow_id == Some(flow_id);
                    self.ui_state.add_or_update_flow(ev.entry);
                    if was_selected {
                        self.ui_state.refresh_selected_flow_detail(&self.core.state.db_path);
                    }
                }
                CoreEvent::TrafficUpdated { id, status, size, duration_ms } => {
                    let was_selected = self.ui_state.selected_flow_id == Some(id);
                    self.ui_state.update_flow(id, status, size, duration_ms);
                    if was_selected {
                        self.ui_state.refresh_selected_flow_detail(&self.core.state.db_path);
                    }
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

        // Drain repeater results
        while let Ok((tab_idx, res)) = self.repeater_result_rx.try_recv() {
            if let Some(tab) = self.ui_state.repeater_tabs.get_mut(tab_idx) {
                tab.is_loading = false;
                match res {
                    Ok(r) => {
                        tab.last_result = Some(r);
                        tab.error = None;
                    }
                    Err(e) => {
                        tab.error = Some(e);
                        tab.last_result = None;
                    }
                }
            }
        }

        // Drain fuzzer results
        while let Ok(msg) = self.fuzzer_result_rx.try_recv() {
            match msg {
                FuzzMsg::Result(row) => {
                    self.ui_state.fuzzer_state.progress_completed += 1;
                    self.ui_state.fuzzer_state.results.push(row);
                }
                FuzzMsg::Done => {
                    self.ui_state.fuzzer_state.is_running = false;
                }
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
            core: &self.core,
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

        // 5. Drain queued repeater actions
        while let Some(cmd) = self.ui_state.repeater_action_queue.pop() {
            match cmd {
                crate::state::RepeaterActionCmd::Send { tab_index, tab } => {
                    let tx = self.repeater_result_tx.clone();
                    self.rt.spawn(async move {
                        let res = mitm_core::repeater::execute::send_tab_request(&tab).await;
                        let _ = tx.send((tab_index, res));
                    });
                }
            }
        }

        // 6. Drain queued fuzzer actions
        while let Some(cmd) = self.ui_state.fuzzer_action_queue.pop() {
            match cmd {
                crate::state::FuzzerActionCmd::Start { template, payloads, concurrency } => {
                    let tx = self.fuzzer_result_tx.clone();
                    let egui_ctx = ui.ctx().clone();
                    self.rt.spawn(async move {
                        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
                        let mut handles = Vec::new();

                        for (idx, payload) in payloads.into_iter().enumerate() {
                            let sem = Arc::clone(&semaphore);
                            let t = template.clone();
                            let tx_clone = tx.clone();
                            let egui_clone = egui_ctx.clone();

                            let handle = tokio::spawn(async move {
                                let _permit = sem.acquire().await.ok();
                                let re = mitm_core::fuzzer::marker_re();
                                let substituted_url = re.replace_all(&t.url, payload.as_str()).to_string();
                                let substituted_body = t.body.as_ref().map(|b| re.replace_all(b, payload.as_str()).to_string());
                                let substituted_headers = t.headers.iter().map(|h| {
                                    let sub_val = re.replace_all(&h.value, payload.as_str()).to_string();
                                    mitm_core::repeater::HeaderItem {
                                        id: h.id.clone(),
                                        key: h.key.clone(),
                                        value: sub_val,
                                        enabled: h.enabled,
                                    }
                                }).collect();

                                let tab = mitm_core::repeater::RepeaterTab {
                                    id: format!("fuzz-{}", idx),
                                    method: t.method.clone(),
                                    url: substituted_url,
                                    headers: substituted_headers,
                                    params: Vec::new(),
                                    body_type: t.body_type.clone(),
                                    body_content: substituted_body,
                                    extract_rules: Vec::new(),
                                    pre_requests: Vec::new(),
                                    post_requests: Vec::new(),
                                    order_index: idx as i32,
                                    created_at_ms: chrono::Utc::now().timestamp_millis(),
                                    updated_at_ms: chrono::Utc::now().timestamp_millis(),
                                    execution_count: 0,
                                    last_status_code: None,
                                    last_duration_ms: None,
                                };

                                let result = match mitm_core::repeater::execute::send_tab_request(&tab).await {
                                    Ok(res) => crate::state::FuzzResultRow {
                                        index: idx + 1,
                                        payload,
                                        status_code: res.status_code,
                                        size: res.response_size,
                                        duration_ms: res.duration_ms,
                                        error: None,
                                    },
                                    Err(e) => crate::state::FuzzResultRow {
                                        index: idx + 1,
                                        payload,
                                        status_code: 0,
                                        size: 0,
                                        duration_ms: 0,
                                        error: Some(e),
                                    },
                                };

                                let _ = tx_clone.send(FuzzMsg::Result(result));
                                egui_clone.request_repaint();
                            });
                            handles.push(handle);
                        }

                        for h in handles {
                            let _ = h.await;
                        }
                        let _ = tx.send(FuzzMsg::Done);
                        egui_ctx.request_repaint();
                    });
                }
                crate::state::FuzzerActionCmd::Stop => {}
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
