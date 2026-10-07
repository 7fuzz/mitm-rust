use std::sync::Arc;
use eframe::egui;
use tokio::runtime::Runtime;
use crate::app::MitmApp;
use crate::recovery::Recovery;
use crate::startup::Starting;

pub struct Shell {
    rt: Arc<Runtime>,
    screen: Screen,
}

enum Screen {
    Starting(Starting),
    Running(Box<MitmApp>),
    Recovery(Recovery),
}

impl Shell {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let rt = Arc::new(Runtime::new().expect("Failed to initialize Tokio runtime"));
        let screen = Screen::Starting(Starting::spawn(&rt, &cc.egui_ctx));
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
        let next = match &mut self.screen {
            Screen::Running(app) => {
                app.ui(ui, frame);
                None
            }
            Screen::Starting(starting) => starting.show(ui).map(|result| match result {
                Ok(core) => Screen::Running(Box::new(MitmApp::new(ui.ctx(), frame.storage(), Arc::clone(&self.rt), core))),
                Err(error) => Screen::Recovery(Recovery::new(error)),
            }),
            Screen::Recovery(recovery) => {
                recovery.show(ui, &self.rt).then(|| Screen::Starting(Starting::spawn(&self.rt, ui.ctx())))
            }
        };
        if let Some(next) = next {
            self.screen = next;
            ui.ctx().request_repaint();
        }
    }
}
