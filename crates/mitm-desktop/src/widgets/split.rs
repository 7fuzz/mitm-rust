use eframe::egui::{self, AsIdSalt, CursorIcon, Rect, Sense, Stroke, UiBuilder};

const HANDLE_WIDTH: f32 = 6.0;
const MIN_PANE_WIDTH: f32 = 120.0;

pub fn horizontal(
    ui: &mut egui::Ui,
    id_salt: impl AsIdSalt,
    fraction: &mut f32,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    let rect = ui.available_rect_before_wrap();
    let usable = (rect.width() - HANDLE_WIDTH).max(0.0);
    let min = MIN_PANE_WIDTH.min(usable / 2.0);
    let left_width = (usable * *fraction).clamp(min, usable - min);

    let left_rect = Rect::from_min_max(rect.min, egui::pos2(rect.left() + left_width, rect.bottom()));
    let handle_rect = Rect::from_min_max(
        egui::pos2(left_rect.right(), rect.top()),
        egui::pos2(left_rect.right() + HANDLE_WIDTH, rect.bottom()),
    );
    let right_rect = Rect::from_min_max(egui::pos2(handle_rect.right(), rect.top()), rect.max);

    let handle = ui.interact(handle_rect, ui.id().with(&id_salt), Sense::click_and_drag());
    if let (true, Some(pointer)) = (handle.dragged(), handle.interact_pointer_pos()) {
        *fraction = ((pointer.x - rect.left() - HANDLE_WIDTH / 2.0) / usable).clamp(0.1, 0.9);
    }
    if handle.double_clicked() {
        *fraction = 0.5;
    }

    let active = handle.hovered() || handle.dragged();
    if active {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let stroke = if active {
        Stroke::new(2.0, ui.visuals().selection.bg_fill)
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    ui.painter().vline(handle_rect.center().x, rect.y_range(), stroke);

    pane(ui, left_rect, (&id_salt, "left"), left);
    pane(ui, right_rect, (&id_salt, "right"), right);
    ui.advance_cursor_after_rect(rect);
}

fn pane(ui: &mut egui::Ui, rect: Rect, id_salt: impl AsIdSalt, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(UiBuilder::new().id_salt(id_salt).max_rect(rect));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    add(&mut child);
}
