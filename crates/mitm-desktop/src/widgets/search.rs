use eframe::egui;

fn armed_id() -> egui::Id {
    egui::Id::new("search_focus_armed")
}

/// Ctrl+F: the dock arms this only while drawing the focused tab, so that tab's first search field takes focus.
pub fn arm(ctx: &egui::Context, armed: bool) {
    ctx.data_mut(|d| d.insert_temp(armed_id(), armed));
}

pub fn claim_focus(response: &egui::Response) {
    let armed = response.ctx.data_mut(|d| std::mem::take(d.get_temp_mut_or_default::<bool>(armed_id())));
    if armed {
        response.request_focus();
    }
}

pub fn search_field(ui: &mut egui::Ui, query: &mut String, hint: &str) {
    let has_text = !query.is_empty();
    let (_, clear) = egui::Sides::new().shrink_left().show(
        ui,
        |ui| claim_focus(&ui.add(egui::TextEdit::singleline(query).hint_text(format!("🔍 {}", hint)).desired_width(f32::INFINITY))),
        |ui| has_text && ui.small_button("✖").on_hover_text("Clear").clicked(),
    );
    if clear {
        query.clear();
    }
}
