use eframe::egui;

/// A vertical scroll area for a list inside a modal, up to `max_height` tall and shrinking to fit fewer rows.
/// A modal only offers its contents last frame's height, so without the floor a list that shrank (a search
/// narrowed it) could never grow back.
pub fn vertical(max_height: f32) -> egui::ScrollArea {
    egui::ScrollArea::vertical().max_height(max_height).min_scrolled_height(max_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The height of a modal holding the list, after showing `rows` rows for a few frames.
    fn list_height(ctx: &egui::Context, rows: usize) -> f32 {
        let mut height = 0.0;
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 800.0))),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let modal = egui::Modal::new(egui::Id::new("list")).show(ui.ctx(), |ui| {
                    vertical(300.0).show(ui, |ui| (0..rows).for_each(|i| drop(ui.label(format!("row {i}")))));
                });
                height = modal.response.rect.height();
            });
            output.textures_delta.clear();
        }
        height
    }

    #[test]
    fn a_list_that_shrank_grows_back() {
        let ctx = egui::Context::default();
        let full = list_height(&ctx, 50);
        let short = list_height(&ctx, 2);
        assert!(short < full / 4.0, "2 rows are {short}px, 50 rows {full}px");
        assert_eq!(list_height(&ctx, 50), full);
    }
}
