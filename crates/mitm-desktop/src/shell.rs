use std::sync::Arc;
use eframe::egui;
use tokio::runtime::Runtime;
use crate::app::MitmApp;
use crate::recovery::{self, Recovery};

pub struct Shell {
    rt: Arc<Runtime>,
    screen: Screen,
}

enum Screen {
    Running(Box<MitmApp>),
    Recovery(Recovery),
}

impl Shell {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let rt = Arc::new(Runtime::new().expect("Failed to initialize Tokio runtime"));
        let screen = match recovery::start_core(&rt) {
            Ok(core) => Screen::Running(Box::new(MitmApp::new(&cc.egui_ctx, cc.storage, Arc::clone(&rt), core))),
            Err(error) => Screen::Recovery(Recovery::new(error)),
        };
        Self { rt, screen }
    }
}

impl eframe::App for Shell {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Screen::Running(app) = &mut self.screen {
            app.save(storage);
        }
    }

    fn on_exit(&mut self) {
        if let Screen::Running(app) = &mut self.screen {
            app.on_exit();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        match &mut self.screen {
            Screen::Running(app) => app.ui(ui, frame),
            Screen::Recovery(recovery) => {
                if let Some(core) = recovery.show(ui, &self.rt) {
                    let app = MitmApp::new(ui.ctx(), frame.storage(), Arc::clone(&self.rt), core);
                    self.screen = Screen::Running(Box::new(app));
                }
            }
        }
    }
}
