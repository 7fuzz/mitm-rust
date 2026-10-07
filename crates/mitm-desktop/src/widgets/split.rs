use eframe::egui::{self, pos2, AsIdSalt, CursorIcon, Rect, Sense, Stroke, UiBuilder};

const HANDLE_SIZE: f32 = 6.0;
const MIN_PANE_SIZE: f32 = 100.0;

pub fn side_by_side(ui: &mut egui::Ui, id_salt: impl AsIdSalt, fraction: &mut f32) -> [egui::Ui; 2] {
    regions(ui, id_salt, fraction, false)
}

pub fn stacked(ui: &mut egui::Ui, id_salt: impl AsIdSalt, fraction: &mut f32) -> [egui::Ui; 2] {
    regions(ui, id_salt, fraction, true)
}

fn regions(ui: &mut egui::Ui, id_salt: impl AsIdSalt, fraction: &mut f32, stacked: bool) -> [egui::Ui; 2] {
    let rect = ui.available_rect_before_wrap();
    let (start, length) = if stacked { (rect.top(), rect.height()) } else { (rect.left(), rect.width()) };
    let usable = (length - HANDLE_SIZE).max(0.0);
    let min = MIN_PANE_SIZE.min(usable / 2.0);
    let at = start + (usable * *fraction).clamp(min, usable - min);

    let (first, handle_rect, second) = if stacked {
        (
            Rect::from_min_max(rect.min, pos2(rect.right(), at)),
            Rect::from_min_max(pos2(rect.left(), at), pos2(rect.right(), at + HANDLE_SIZE)),
            Rect::from_min_max(pos2(rect.left(), at + HANDLE_SIZE), rect.max),
        )
    } else {
        (
            Rect::from_min_max(rect.min, pos2(at, rect.bottom())),
            Rect::from_min_max(pos2(at, rect.top()), pos2(at + HANDLE_SIZE, rect.bottom())),
            Rect::from_min_max(pos2(at + HANDLE_SIZE, rect.top()), rect.max),
        )
    };

    let handle = ui.interact(handle_rect, ui.id().with(&id_salt), Sense::click_and_drag());
    if let (true, Some(pointer)) = (handle.dragged(), handle.interact_pointer_pos()) {
        let along = if stacked { pointer.y } else { pointer.x };
        *fraction = ((along - start - HANDLE_SIZE / 2.0) / usable).clamp(0.1, 0.9);
    }
    if handle.double_clicked() {
        *fraction = 0.5;
    }

    let active = handle.hovered() || handle.dragged();
    if active {
        ui.ctx().set_cursor_icon(if stacked { CursorIcon::ResizeVertical } else { CursorIcon::ResizeHorizontal });
    }
    let stroke = if active {
        Stroke::new(2.0, ui.visuals().selection.bg_fill)
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let center = handle_rect.center();
    if stacked {
        ui.painter().hline(rect.x_range(), center.y, stroke);
    } else {
        ui.painter().vline(center.x, rect.y_range(), stroke);
    }

    let first = pane(ui, first, (&id_salt, "first"));
    let second = pane(ui, second, (&id_salt, "second"));
    ui.advance_cursor_after_rect(rect);
    [first, second]
}

fn pane(ui: &mut egui::Ui, rect: Rect, id_salt: impl AsIdSalt) -> egui::Ui {
    let mut child = ui.new_child(UiBuilder::new().id_salt(id_salt).max_rect(rect));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child
}
