use eframe::egui::{self, Align, Layout, RichText};
use mitm_core::credentials::{Credential, FieldKind};
use mitm_core::db::Db;
use mitm_core::utils::totp;
use crate::app::command::{Commands, QrTarget};
use crate::features::utilities::totp::{code_label, copy_code, countdown, qr_buttons};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::grow_edit;
use super::view::{rename_field, RENAME_WIDTH};
use super::{CredentialsState, RenameTarget};

const VALUE_LINES: f32 = 3.0;
const MIN_VALUE_WIDTH: f32 = 220.0;
/// `grow_edit`'s border margin plus room for the cursor, so a value doesn't wrap just before the box widens.
const VALUE_PADDING: f32 = 20.0;
const SLOT_ICONS: [&str; 4] = [icons::EYE, icons::COPY, icons::X, icons::PENCIL_SIMPLE];

/// What a field row carries while it's dragged to a new place in its entry.
struct FieldDrag {
    entry_id: String,
    index: usize,
    key: String,
}

/// Widths every field row of one entry shares, so the kinds, values and buttons line up.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Columns {
    handle: f32,
    name: f32,
    kind: f32,
    /// One square button: the eye before the value, copy and remove after it.
    slot: f32,
    /// The widest value as shown; rows cap it at the room left in the panel.
    value: f32,
}

impl Columns {
    fn measure(ui: &egui::Ui, entry: &Credential, renaming: bool) -> Self {
        let text_width = |text: &str, style: egui::TextStyle| {
            let font = style.resolve(ui.style());
            ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font, egui::Color32::PLACEHOLDER)).size().x
        };
        let widest = |texts: &mut dyn Iterator<Item = String>, style: egui::TextStyle| {
            texts.map(|t| text_width(&t, style.clone())).fold(0.0, f32::max)
        };
        let spacing = ui.spacing();
        let name = widest(&mut entry.fields.iter().map(|f| f.key.clone()), egui::TextStyle::Body);
        let kind = widest(&mut FieldKind::ALL.iter().map(|k| k.label().to_string()), egui::TextStyle::Button)
            + spacing.icon_width
            + spacing.icon_spacing
            + 2.0 * spacing.button_padding.x;
        let mut shown = entry.fields.iter().filter(|f| f.kind != FieldKind::Totp).flat_map(|f| match f.kind {
            FieldKind::Secret => vec!["•".repeat(f.value.chars().count())],
            _ => f.value.lines().map(str::to_string).collect(),
        });
        Columns {
            handle: text_width(icons::DOTS_SIX_VERTICAL, egui::TextStyle::Body),
            name: if renaming { name.max(RENAME_WIDTH) } else { name },
            kind,
            slot: (widest(&mut SLOT_ICONS.iter().map(|i| i.to_string()), egui::TextStyle::Button) + 2.0 * spacing.button_padding.x)
                .max(spacing.interact_size.y),
            value: (widest(&mut shown, egui::TextStyle::Body) + VALUE_PADDING).max(MIN_VALUE_WIDTH),
        }
    }
}

struct RowOutcome {
    changed: bool,
    remove: bool,
}

/// The entry's fields, one row each: drag handle, name, kind, value and its buttons. Returns true when an
/// edit changed the entry.
pub(super) fn field_rows(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, id: &str, cmds: &mut Commands) -> bool {
    let Some(entry) = state.entries.iter().find(|e| e.id == id) else { return false };
    let renaming = matches!(&state.rename, Some(r) if matches!(&r.target, RenameTarget::Field(e, _) if e == id));
    let columns = Columns::measure(ui, entry, renaming);
    let keys: Vec<String> = entry.fields.iter().map(|f| f.key.clone()).collect();

    let mut changed = false;
    let mut remove = None;
    let mut rows = Vec::new();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for (i, key) in keys.iter().enumerate() {
            let row = ui.with_layout(Layout::left_to_right(Align::Min), |ui| field_row(ui, state, db, cmds, &columns, id, i, key));
            changed |= row.inner.changed;
            if row.inner.remove {
                remove = Some(i);
            }
            rows.push(row.response.rect);
        }
    });

    let Some(entry) = state.entries.iter_mut().find(|e| e.id == id) else { return changed };
    if let Some(i) = remove {
        entry.fields.remove(i);
        changed = true;
    }
    let x = rows.iter().fold(egui::Rect::NOTHING, |all, r| all.union(*r)).x_range();
    let ys: Vec<egui::Rangef> = rows.iter().map(|r| r.y_range()).collect();
    changed | drop_field(ui, entry, &ys, x)
}

#[allow(clippy::too_many_arguments)]
fn field_row(
    ui: &mut egui::Ui,
    state: &mut CredentialsState,
    db: &Db,
    cmds: &mut Commands,
    columns: &Columns,
    id: &str,
    i: usize,
    key: &str,
) -> RowOutcome {
    let mut changed = false;
    let handle = cell(ui, columns.handle, |ui| {
        ui.add(egui::Label::new(RichText::new(icons::DOTS_SIX_VERTICAL).weak()).selectable(false).sense(egui::Sense::drag()))
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text("Drag to reorder")
    });
    if handle.drag_started() {
        handle.dnd_set_drag_payload(FieldDrag { entry_id: id.to_string(), index: i, key: key.to_string() });
    }
    cell(ui, columns.name, |ui| rename_field(ui, state, db, RenameTarget::Field(id.to_string(), key.to_string()), key, RichText::new(key).strong()));

    let Some(field) = state.entries.iter_mut().find(|e| e.id == id).and_then(|e| e.fields.get_mut(i)) else {
        return RowOutcome { changed, remove: false };
    };
    cell(ui, columns.kind, |ui| {
        egui::ComboBox::from_id_salt(("credential_kind", id, i)).selected_text(field.kind.label()).width(columns.kind).show_ui(ui, |ui| {
            for kind in FieldKind::ALL {
                changed |= ui.selectable_value(&mut field.kind, kind, kind.label()).changed();
            }
        });
    });
    let slot = columns.slot;
    let trailing = 2.0 * (slot + ui.spacing().item_spacing.x);
    let value_width = |ui: &egui::Ui| columns.value.min(ui.available_width() - trailing).max(slot * 4.0);

    match field.kind {
        FieldKind::Text => {
            cell(ui, slot, |_| {});
            changed |= value_edit(ui, (id, key), &mut field.value, false, value_width(ui)).changed();
            if cell(ui, slot, |ui| slot_button(ui, slot, icons::COPY, "Copy")).clicked() {
                ui.ctx().copy_text(field.value.clone());
            }
        }
        FieldKind::Secret => {
            let eye = cell(ui, slot, |ui| eye_button(ui, slot));
            changed |= value_edit(ui, (id, key), &mut field.value, !eye.is_pointer_button_down_on(), value_width(ui)).changed();
            if cell(ui, slot, |ui| slot_button(ui, slot, icons::COPY, "Copy")).clicked() {
                crate::util::clipboard::copy_secret(ui.ctx(), &field.value);
            }
        }
        FieldKind::Totp => {
            let draft = state.totp_draft.as_ref().filter(|(d_id, d_key, _)| d_id == id && d_key == key).map(|d| d.2.clone());
            match (draft, totp::parse_secret(&field.value).ok()) {
                (None, Some(params)) => {
                    cell(ui, slot, |_| {});
                    let width = value_width(ui);
                    let edit = cell_with(ui, width, Layout::top_down(Align::Min), |ui| {
                        let edit = ui.horizontal(|ui| {
                            code_label(ui, &params, 16.0);
                            icon_button(ui, icons::PENCIL_SIMPLE, "Edit secret")
                        });
                        countdown(ui, &params);
                        edit.inner
                    });
                    if edit.clicked() {
                        state.totp_draft = Some((id.to_string(), key.to_string(), field.value.clone()));
                    }
                    if cell(ui, slot, |ui| slot_button(ui, slot, icons::COPY, "Copy code")).clicked() {
                        copy_code(ui, &params);
                    }
                }
                (draft, _) => {
                    let mut text = draft.unwrap_or_default();
                    let eye = cell(ui, slot, |ui| eye_button(ui, slot));
                    let width = value_width(ui);
                    let (edit, scanned) = cell(ui, width, |ui| {
                        let qr_width = 2.0 * (slot + ui.spacing().item_spacing.x);
                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut text)
                                .password(!eye.is_pointer_button_down_on())
                                .hint_text("secret or otpauth://…")
                                .desired_width(width - qr_width),
                        );
                        let target = QrTarget::CredentialField { id: id.to_string(), key: key.to_string() };
                        (edit, qr_buttons(ui, cmds, target))
                    });
                    cell(ui, slot, |_| {});
                    let was_scanned = matches!(scanned, Some(Ok(_)));
                    match scanned {
                        Some(Ok(scanned)) => text = scanned,
                        Some(Err(e)) => state.error = Some(e),
                        None => {}
                    }
                    let has_saved = totp::parse_secret(&field.value).is_ok();
                    let escaped = edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let unchanged = text.trim().is_empty() || text.trim() == field.value;
                    if has_saved && (escaped || (edit.lost_focus() && unchanged)) {
                        state.totp_draft = None;
                    } else if (edit.lost_focus() || was_scanned) && !text.trim().is_empty() {
                        match totp::parse_secret(&text) {
                            Ok(_) => {
                                field.value = text.trim().to_string();
                                changed = true;
                                state.totp_draft = None;
                            }
                            Err(e) => {
                                state.error = Some(e);
                                state.totp_draft = Some((id.to_string(), key.to_string(), text));
                            }
                        }
                    } else if edit.changed() || state.totp_draft.is_some() {
                        state.totp_draft = Some((id.to_string(), key.to_string(), text));
                    }
                }
            }
        }
    }
    let remove = cell(ui, slot, |ui| slot_button(ui, slot, icons::X, "Remove field")).clicked();
    RowOutcome { changed, remove }
}

/// `icon_button` sized to fill a slot, so every slot's button is the same width.
fn slot_button(ui: &mut egui::Ui, side: f32, icon: &str, hover: &str) -> egui::Response {
    ui.add(egui::Button::new(icon).frame_when_inactive(false).min_size(egui::vec2(side, ui.spacing().interact_size.y)))
        .on_hover_text(hover)
}

fn eye_button(ui: &mut egui::Ui, side: f32) -> egui::Response {
    let size = egui::vec2(side, ui.spacing().interact_size.y);
    ui.add(egui::Button::new(icons::EYE).min_size(size).sense(egui::Sense::drag())).on_hover_text("Hold to reveal")
}

/// A value box that widens with its text up to `width`, then wraps, growing to three lines before it scrolls.
/// Line breaks are kept, so keys and certificates paste intact.
fn value_edit(ui: &mut egui::Ui, id_salt: impl egui::AsIdSalt, text: &mut String, masked: bool, width: f32) -> egui::Response {
    grow_edit::show(ui, id_salt, egui::TextStyle::Body, width, VALUE_LINES, |ui, inner_width| {
        ui.add(
            egui::TextEdit::multiline(text)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .desired_rows(1)
                .password(masked)
                .desired_width(inner_width),
        )
    })
}

/// A slot exactly `width` wide and one control tall, its contents centred vertically.
fn cell<R>(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    cell_with(ui, width, Layout::left_to_right(Align::Center), add)
}

fn cell_with<R>(ui: &mut egui::Ui, width: f32, layout: Layout, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let size = egui::vec2(width, ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        ui.set_min_size(size);
        add(ui)
    })
    .inner
}

/// While a field of `entry` is dragged, marks where it would land; on release moves it there.
fn drop_field(ui: &egui::Ui, entry: &mut Credential, rows: &[egui::Rangef], x: egui::Rangef) -> bool {
    let Some(drag) = egui::DragAndDrop::payload::<FieldDrag>(ui.ctx()).filter(|d| d.entry_id == entry.id) else { return false };
    let Some(pointer) = ui.ctx().pointer_interact_pos() else { return false };
    let slot = rows.iter().filter(|row| row.center() < pointer.y).count();
    let y = match slot {
        0 => rows.first().map_or(0.0, |r| r.min - 3.0),
        n if n == rows.len() => rows[n - 1].max + 3.0,
        n => (rows[n - 1].max + rows[n].min) / 2.0,
    };
    ui.painter().hline(x, y, egui::Stroke::new(2.0, ui.visuals().selection.stroke.color));
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if !ui.input(|i| i.pointer.any_released()) || drag.index >= entry.fields.len() || entry.fields[drag.index].key != drag.key {
        return false;
    }
    egui::DragAndDrop::clear_payload(ui.ctx());
    let field = entry.fields.remove(drag.index);
    entry.fields.insert(if slot > drag.index { slot - 1 } else { slot }, field);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitm_core::credentials::Field;

    fn entry(fields: Vec<Field>) -> Credential {
        Credential {
            id: "c".into(),
            vault_id: "v".into(),
            vault: "DG".into(),
            name: "admin".into(),
            tags: Vec::new(),
            fields,
            notes: String::new(),
            order_index: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    fn field(key: &str, kind: FieldKind, value: &str) -> Field {
        Field { key: key.into(), kind, extracted: false, value: value.into() }
    }

    fn texts(shape: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => out.push((text.galley.text().to_string(), egui::Rect::from_min_size(text.pos, text.galley.size()))),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| texts(s, out)),
            _ => {}
        }
    }

    /// Every text drawn by the rows of an entry with these fields, with where it landed.
    fn drawn(fields: Vec<Field>) -> Vec<(String, egui::Rect)> {
        let ctx = egui::Context::default();
        let mut state = CredentialsState { entries: vec![entry(fields)], ..Default::default() };
        let db = Db::new(std::env::temp_dir().join("mitm-fields-layout-unused.db"));
        let mut cmds = Commands::default();
        let mut found = Vec::new();
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 400.0))),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                field_rows(ui, &mut state, &db, "c", &mut cmds);
            });
            output.textures_delta.clear();
            found.clear();
            output.shapes.iter().for_each(|s| texts(&s.shape, &mut found));
        }
        found
    }

    fn lefts(found: &[(String, egui::Rect)], text: &str) -> Vec<f32> {
        found.iter().filter(|(t, _)| t == text).map(|(_, r)| r.left().round()).collect()
    }

    fn left_of(found: &[(String, egui::Rect)], text: &str) -> f32 {
        lefts(found, text)[0]
    }

    #[test]
    fn kinds_values_and_buttons_line_up_across_rows() {
        let found = drawn(vec![
            field("username", FieldKind::Text, &format!("admin@{}", "a".repeat(60))),
            field("password", FieldKind::Secret, "hunter2"),
            field("otp", FieldKind::Totp, "MZXW6"),
        ]);
        assert_eq!(left_of(&found, "text"), left_of(&found, "secret"));
        assert_eq!(left_of(&found, "text"), left_of(&found, "totp"));
        let copies = lefts(&found, icons::COPY);
        assert_eq!(copies.len(), 3, "{found:?}");
        assert!(copies.iter().all(|x| *x == copies[0]), "copy buttons at {copies:?}");
        let removes = lefts(&found, icons::X);
        assert!(removes.len() == 3 && removes.iter().all(|x| *x == removes[0]), "remove buttons at {removes:?}");
        assert_eq!(left_of(&found, &format!("admin@{}", "a".repeat(60))), left_of(&found, "•••••••"));
    }

    #[test]
    fn the_handle_sits_next_to_the_name_and_values_widen_with_their_text() {
        let found = drawn(vec![field("username", FieldKind::Text, "short")]);
        let handle = found.iter().find(|(t, _)| t == icons::DOTS_SIX_VERTICAL).unwrap().1;
        assert!(left_of(&found, "username") - handle.right() < 12.0);
        let narrow = left_of(&found, icons::COPY);
        let found = drawn(vec![field("username", FieldKind::Text, &"a".repeat(80))]);
        assert!(left_of(&found, icons::COPY) > narrow + 100.0);
    }
}
