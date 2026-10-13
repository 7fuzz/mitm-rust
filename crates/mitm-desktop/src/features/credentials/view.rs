use eframe::egui::containers::menu::{MenuConfig, SubMenuButton};
use eframe::egui::{self, RichText};
use mitm_core::credentials::{Credential, CredentialSummary, FieldKind, Vault};
use mitm_core::workspace::Workspace;
use mitm_core::utils::totp;
use crate::app::command::Commands;
use crate::features::utilities::totp::group_code;
use crate::widgets::{confirm, split};
use super::lock::unlock_form;
use super::{CredentialsState, Rename, RenameTarget};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::list_row::{list_row, list_row_sensing};
use crate::widgets::pills::badge;
use crate::ui::theme::Tone;
use crate::widgets::search::search_field;
use crate::widgets::section::section_with;
use mitm_core::db::Db;

pub fn show(ui: &mut egui::Ui, state: &mut CredentialsState, workspace_id: &str, workspaces: &[Workspace], db: &Db, cmds: &mut Commands) {
    state.sync(db, workspace_id);
    let now = ui.input(|i| i.time);
    let [mut list_ui, mut detail_ui] = split::side_by_side(ui, "credentials_split", &mut state.split_fraction);
    vault_list(&mut list_ui, state, workspaces, db);
    entry_detail(&mut detail_ui, state, db, now, cmds);
    delete_vault_modal(ui.ctx(), state, db);
}

/// What a credential row carries while it's dragged to a new place in its vault.
struct EntryDrag {
    id: String,
    vault_id: String,
    name: String,
}

/// Tags shown on a list row before the rest collapse into `+N`.
const ROW_TAGS: usize = 3;

enum ListAction {
    Select(String),
    Reorder { vault_id: String, dragged: String, target: String, after: bool },
    AddEntry(String),
    ToggleCollapsed(String),
    Rename(RenameTarget, String),
    DeleteEntry(String),
    DeleteVault(String),
    Link { vault_id: String, workspace_id: String, linked: bool },
}

fn vault_list(ui: &mut egui::Ui, state: &mut CredentialsState, workspaces: &[Workspace], db: &Db) {
    section_with(ui, "Vaults", |ui| {
        ui.add_enabled_ui(!state.locked, |ui| {
            if icon_button(ui, icons::DOWNLOAD_SIMPLE, "Import from JSON…").clicked() {
                let vault = state.default_vault().map(|v| v.name.clone());
                state.json_import.start(vault);
            }
            if icon_button(ui, icons::PLUS, "New vault").clicked() {
                state.create_vault(db);
            }
        });
    });
    search_field(ui, &mut state.search, "Search names and tags");
    ui.add_space(4.0);

    let filter = state.search.trim().to_lowercase();
    let workspace_id = state.loaded_for.clone().unwrap_or_default();
    let (linked, other): (Vec<Vault>, Vec<Vault>) = state.vaults.iter().cloned().partition(|v| v.is_linked(&workspace_id));
    floating_drag_label(ui);
    let mut action = None;
    egui::ScrollArea::vertical().id_salt("credential_list").show(ui, |ui| {
        if state.vaults.is_empty() {
            ui.weak("No vaults yet");
        }
        for vault in &linked {
            vault_rows(ui, state, db, vault, &filter, workspaces, &workspace_id, &mut action);
        }
        if !other.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new("OTHER VAULTS").small().weak())
                .on_hover_text("Not linked to this workspace, so its requests and variables can't use them");
            for vault in &other {
                vault_rows(ui, state, db, vault, &filter, workspaces, &workspace_id, &mut action);
            }
        }
    });

    match action {
        Some(ListAction::Select(id)) => state.selected = Some(id),
        Some(ListAction::Reorder { vault_id, dragged, target, after }) => state.reorder(db, &vault_id, &dragged, &target, after),
        Some(ListAction::AddEntry(vault_id)) => state.create(db, &vault_id),
        Some(ListAction::ToggleCollapsed(vault_id)) => {
            if !state.collapsed.remove(&vault_id) {
                state.collapsed.insert(vault_id);
            }
        }
        Some(ListAction::Rename(target, text)) => state.rename = Some(Rename { target, text }),
        Some(ListAction::DeleteEntry(id)) => state.delete(db, &id),
        Some(ListAction::DeleteVault(id)) => state.delete_vault = Some(id),
        Some(ListAction::Link { vault_id, workspace_id, linked }) => state.set_linked(db, &vault_id, &workspace_id, linked),
        None => {}
    }
}

fn vault_rows(
    ui: &mut egui::Ui,
    state: &mut CredentialsState,
    db: &Db,
    vault: &Vault,
    filter: &str,
    workspaces: &[Workspace],
    workspace_id: &str,
    action: &mut Option<ListAction>,
) {
    let vault_matches = vault.name.to_lowercase().contains(filter);
    let entries: Vec<CredentialSummary> = state
        .summaries
        .iter()
        .filter(|s| s.vault_id == vault.id && (filter.is_empty() || vault_matches || entry_matches(s, filter)))
        .cloned()
        .collect();
    if !filter.is_empty() && entries.is_empty() && !vault_matches {
        return;
    }
    let collapsed = filter.is_empty() && state.collapsed.contains(&vault.id);

    let target = RenameTarget::Vault(vault.id.clone());
    let header = list_row(ui, false, |ui| {
        ui.label(RichText::new(if collapsed { icons::CARET_RIGHT } else { icons::CARET_DOWN }).weak());
        if !rename_edit(ui, state, db, &target) {
            ui.label(RichText::new(&vault.name).strong());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui.add_enabled_ui(!state.locked, |ui| icon_button(ui, icons::PLUS, "Add credential")).inner;
            if add.clicked() {
                *action = Some(ListAction::AddEntry(vault.id.clone()));
            }
            if collapsed {
                ui.label(RichText::new(entries.len().to_string()).small().weak());
            }
        });
    });
    if header.clicked() {
        *action = Some(ListAction::ToggleCollapsed(vault.id.clone()));
    }
    header.context_menu(|ui| {
        if ui.button("Rename").clicked() {
            *action = Some(ListAction::Rename(target.clone(), vault.name.clone()));
            ui.close();
        }
        let linked_here = vault.is_linked(workspace_id);
        let toggle = if linked_here { "Unlink from this workspace" } else { "Link to this workspace" };
        if ui.button(toggle).clicked() {
            *action = Some(ListAction::Link { vault_id: vault.id.clone(), workspace_id: workspace_id.to_string(), linked: !linked_here });
            ui.close();
        }
        // Several workspaces can be ticked in one go, so only a click outside closes it.
        let config = MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        SubMenuButton::new("Workspaces").config(config).ui(ui, |ui| {
            for workspace in workspaces {
                let mut linked = vault.is_linked(&workspace.id);
                if ui.checkbox(&mut linked, &workspace.name).changed() {
                    *action = Some(ListAction::Link { vault_id: vault.id.clone(), workspace_id: workspace.id.clone(), linked });
                }
            }
        });
        ui.separator();
        if ui.button(RichText::new("Delete vault…").color(ui.visuals().error_fg_color)).clicked() {
            *action = Some(ListAction::DeleteVault(vault.id.clone()));
            ui.close();
        }
    });
    if collapsed {
        return;
    }

    for entry in &entries {
        let selected = state.selected.as_deref() == Some(entry.id.as_str());
        let code = state.entries.iter().find(|e| e.id == entry.id).and_then(first_code);
        let row = ui
            .indent(("credential_entry", &entry.id), |ui| {
                list_row_sensing(ui, selected, egui::Sense::click_and_drag(), |ui| {
                    ui.add(egui::Label::new(&entry.name).selectable(false));
                    tag_pills(ui, &entry.tags, ROW_TAGS);
                    if let Some(code) = code {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(group_code(&code)).monospace().weak());
                        });
                    }
                })
            })
            .inner;
        if row.clicked() {
            *action = Some(ListAction::Select(entry.id.clone()));
        }
        if row.drag_started() {
            row.dnd_set_drag_payload(EntryDrag { id: entry.id.clone(), vault_id: vault.id.clone(), name: entry.name.clone() });
        }
        let same_vault = |drag: &EntryDrag| drag.vault_id == vault.id && drag.id != entry.id;
        if row.dnd_hover_payload::<EntryDrag>().is_some_and(|d| same_vault(&d)) {
            let after = row.ctx.pointer_interact_pos().is_some_and(|p| p.y > row.rect.center().y);
            let y = if after { row.rect.bottom() } else { row.rect.top() };
            let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
            ui.painter().hline(row.rect.x_range(), y, stroke);
        }
        if let Some(drag) = row.dnd_release_payload::<EntryDrag>().filter(|d| same_vault(d)) {
            let after = row.ctx.pointer_interact_pos().is_some_and(|p| p.y > row.rect.center().y);
            *action = Some(ListAction::Reorder { vault_id: vault.id.clone(), dragged: drag.id.clone(), target: entry.id.clone(), after });
        }
        row.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                *action = Some(ListAction::Rename(RenameTarget::Entry(entry.id.clone()), entry.name.clone()));
                ui.close();
            }
            if ui.button("Delete").clicked() {
                *action = Some(ListAction::DeleteEntry(entry.id.clone()));
                ui.close();
            }
        });
    }
    if entries.is_empty() && filter.is_empty() {
        ui.indent(("empty_vault", &vault.id), |ui| ui.weak("Empty"));
    }
}

fn delete_vault_modal(ctx: &egui::Context, state: &mut CredentialsState, db: &Db) {
    let Some(id) = state.delete_vault.clone() else { return };
    let Some(vault) = state.vaults.iter().find(|v| v.id == id).cloned() else {
        state.delete_vault = None;
        return;
    };
    let count = state.summaries.iter().filter(|s| s.vault_id == id).count();
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("delete_vault")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading(format!("Delete {}?", vault.name));
        ui.add_space(6.0);
        let entries = if count == 1 { "1 credential".to_string() } else { format!("{count} credentials") };
        ui.label(format!("Its {entries} are deleted too, and refs to it stop resolving in {} workspace(s).", vault.workspace_ids.len()));
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(crate::widgets::button::danger_filled(ui, "Delete")).clicked() {
                state.remove_vault(db, &id);
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        state.delete_vault = None;
    }
}

fn first_code(entry: &Credential) -> Option<String> {
    let field = entry.fields.iter().find(|f| f.kind == FieldKind::Totp)?;
    let params = totp::parse_secret(&field.value).ok()?;
    Some(totp::code_at(&params, totp::now_unix()))
}

fn entry_detail(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, now: f64, cmds: &mut Commands) {
    if state.locked {
        ui.add_space(8.0);
        unlock_form(ui, state, db);
        return;
    }
    if state.has_password {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if icon_button(ui, icons::LOCK, "Lock credentials").clicked() {
                state.lock(db);
            }
        });
    }
    let Some(id) = state.selected.clone() else { return };
    let Some(index) = state.entries.iter().position(|e| e.id == id) else { return };

    ui.horizontal(|ui| {
        let name = state.entries[index].name.clone();
        if rename_field(ui, state, db, RenameTarget::Entry(id.clone()), &name, RichText::new(&name).heading().strong()) {
            return;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let armed_id = ui.id().with(("delete_credential", &id));
            if confirm::confirm_button(ui, armed_id, RichText::new(icons::TRASH), "Delete?", "Delete credential") {
                state.delete(db, &id);
            }
        });
    });
    if let Some(error) = state.error.clone() {
        ui.horizontal(|ui| {
            ui.colored_label(ui.visuals().error_fg_color, error);
            if icon_button(ui, icons::X, "Dismiss").clicked() {
                state.error = None;
            }
        });
    }
    ui.add_space(2.0);
    tag_editor(ui, state, &id, now);
    ui.add_space(6.0);
    if state.selected.as_deref() != Some(id.as_str()) {
        return;
    }

    let mut changed = super::fields::field_rows(ui, state, db, &id, cmds);
    if ui.button("Add field").clicked() {
        state.add_field(db, &id);
    }
    ui.add_space(8.0);
    if let Some(entry) = state.entries.iter_mut().find(|e| e.id == id) {
        changed |= ui
            .add(egui::TextEdit::multiline(&mut entry.notes).hint_text("Notes").desired_rows(3).desired_width(f32::INFINITY))
            .changed();
    }
    if changed {
        state.mark_dirty(&id, now);
    }
}

fn entry_matches(entry: &CredentialSummary, filter: &str) -> bool {
    entry.name.to_lowercase().contains(filter) || entry.tags.iter().any(|t| t.to_lowercase().contains(filter))
}

/// A stable colour per tag, so the same tag looks the same everywhere.
fn tag_tone(tag: &str) -> Tone {
    let hash = tag.to_lowercase().bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    Tone::Tag((hash as usize % Tone::TAG_COUNT) as u8)
}

fn tag_pills(ui: &mut egui::Ui, tags: &[String], max: usize) {
    for tag in tags.iter().take(max) {
        badge(ui, tag, tag_tone(tag));
    }
    if tags.len() > max {
        ui.label(RichText::new(format!("+{}", tags.len() - max)).small().weak()).on_hover_text(tags[max..].join(", "));
    }
}

/// The entry's tags, each removable, and a field to add one (Enter) or pick one used elsewhere.
fn tag_editor(ui: &mut egui::Ui, state: &mut CredentialsState, id: &str, now: f64) {
    let Some(entry) = state.entries.iter().find(|e| e.id == id) else { return };
    let mut tags = entry.tags.clone();
    let mut known: Vec<String> = Vec::new();
    for tag in state.summaries.iter().flat_map(|s| &s.tags) {
        if !tags.iter().chain(&known).any(|t| t.eq_ignore_ascii_case(tag)) {
            known.push(tag.clone());
        }
    }
    known.sort_by_key(|t| t.to_lowercase());

    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        let mut remove = None;
        for (i, tag) in tags.iter().enumerate() {
            let pill = badge(ui, &format!("{tag} {}", icons::X), tag_tone(tag)).interact(egui::Sense::click());
            if pill.on_hover_text("Remove tag").clicked() {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            tags.remove(i);
            changed = true;
        }
        let edit_id = ui.make_persistent_id(("tag_draft", id));
        let suggestions = tag_suggestions(&known, &state.tag_draft);
        if ui.memory(|m| m.has_focus(edit_id))
            && !suggestions.is_empty()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab))
        {
            state.tag_draft = suggestions[0].clone();
            let mut edit_state = egui::text_edit::TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
            let end = egui::text::CCursor::new(state.tag_draft.chars().count());
            edit_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
            edit_state.store(ui.ctx(), edit_id);
        }
        // Holding Tab back from focus navigation while there's something to complete.
        let filter = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: !suggestions.is_empty(), ..Default::default() };
        let edit = ui.add(
            egui::TextEdit::singleline(&mut state.tag_draft).id(edit_id).hint_text("Add tag").desired_width(90.0).event_filter(filter),
        );
        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !state.tag_draft.trim().is_empty() {
            tags.push(std::mem::take(&mut state.tag_draft));
            changed = true;
            edit.request_focus();
        }
        if let Some(tag) = suggestion_popup(ui, &edit, &suggestions) {
            tags.push(tag);
            state.tag_draft.clear();
            changed = true;
            edit.request_focus();
        }
        if !known.is_empty() {
            ui.menu_button(icons::CARET_DOWN, |ui| {
                for tag in &known {
                    if ui.button(tag).clicked() {
                        tags.push(tag.clone());
                        changed = true;
                        ui.close();
                    }
                }
            });
        }
    });
    if changed {
        if let Some(entry) = state.entries.iter_mut().find(|e| e.id == id) {
            entry.tags = mitm_core::credentials::normalize_tags(&tags);
        }
        state.mark_dirty(id, now);
    }
}

/// Known tags containing `draft`, ignoring case, those starting with it first.
fn tag_suggestions(known: &[String], draft: &str) -> Vec<String> {
    const MAX: usize = 6;
    let draft = draft.trim().to_lowercase();
    if draft.is_empty() {
        return Vec::new();
    }
    let mut matches: Vec<&String> = known.iter().filter(|t| t.to_lowercase().contains(&draft)).collect();
    matches.sort_by_key(|t| !t.to_lowercase().starts_with(&draft));
    matches.into_iter().take(MAX).cloned().collect()
}

/// The suggestions under the tag edit while it has focus, the first marked as the one Tab completes to.
/// Returns a clicked suggestion.
fn suggestion_popup(ui: &egui::Ui, edit: &egui::Response, suggestions: &[String]) -> Option<String> {
    let open_id = edit.id.with("suggestions_open");
    let was_open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
    // A click on a suggestion takes focus from the edit before it lands; stay up until it does.
    let clicking = was_open && ui.input(|i| i.pointer.any_down() || i.pointer.any_released());
    let open = !suggestions.is_empty() && (edit.has_focus() || clicking);
    ui.data_mut(|d| d.insert_temp(open_id, open));
    if !open {
        return None;
    }
    let mut picked = None;
    egui::Area::new(edit.id.with("suggestions"))
        .order(egui::Order::Foreground)
        .fixed_pos(edit.rect.left_bottom() + egui::vec2(0.0, 2.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for (i, tag) in suggestions.iter().enumerate() {
                    if ui.add(egui::Button::selectable(i == 0, tag).frame_when_inactive(false)).clicked() {
                        picked = Some(tag.clone());
                    }
                }
            });
        });
    picked
}

fn floating_drag_label(ui: &egui::Ui) {
    let Some(drag) = egui::DragAndDrop::payload::<EntryDrag>(ui.ctx()) else { return };
    let Some(pos) = ui.ctx().pointer_interact_pos() else { return };
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    egui::Area::new(egui::Id::new("credential_drag_label"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos + egui::vec2(12.0, 8.0))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| ui.label(&drag.name));
        });
}

pub(super) const RENAME_WIDTH: f32 = 140.0;

/// A label that turns into an edit on click. Returns true while editing.
pub(super) fn rename_field(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, target: RenameTarget, current: &str, label: RichText) -> bool {
    if rename_edit(ui, state, db, &target) {
        return true;
    }
    let response = ui
        .add(egui::Label::new(label).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::Text);
    if response.clicked() {
        state.rename = Some(Rename { target, text: current.to_string() });
    }
    false
}

/// Rewrites a name being typed into a valid one, keeping the cursor after the same character.
fn sanitize_in_place(ui: &egui::Ui, text: &mut String, output: &mut egui::text_edit::TextEditOutput) {
    let clean = mitm_core::credentials::sanitize_name(text);
    if clean == *text {
        return;
    }
    let cursor = output.cursor_range.map_or(0, |r| r.primary.index.0);
    let before: String = text.chars().take(cursor).collect();
    let kept = mitm_core::credentials::sanitize_name(&before).chars().count().min(clean.chars().count());
    *text = clean;
    output.state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(kept))));
    output.state.clone().store(ui.ctx(), output.response.id);
}

/// The edit for a rename of `target` in progress; returns false when none is.
fn rename_edit(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, target: &RenameTarget) -> bool {
    let Some(rename) = state.rename.as_mut().filter(|r| &r.target == target) else { return false };
    let len = rename.text.chars().count();
    let mut output = egui::TextEdit::singleline(&mut rename.text).desired_width(RENAME_WIDTH).show(ui);
    if output.response.changed() {
        sanitize_in_place(ui, &mut rename.text, &mut output);
    }
    let edit = output.response;
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.rename = None;
    } else if edit.lost_focus() {
        state.commit_rename(db);
    } else if !edit.has_focus() {
        // Select the whole name so typing replaces it.
        edit.request_focus();
        let all = egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(len));
        output.state.cursor.set_char_range(Some(all));
        output.state.store(ui.ctx(), edit.id);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_names_and_tags_and_tags_keep_their_colour() {
        let entry = CredentialSummary {
            id: "c".into(),
            vault_id: "v".into(),
            vault: "DG".into(),
            name: "dgAdmin".into(),
            tags: vec!["Staging".into()],
            fields: Vec::new(),
        };
        assert!(entry_matches(&entry, "admin") && entry_matches(&entry, "stag"));
        assert!(!entry_matches(&entry, "prod"));
        assert_eq!(tag_tone("staging"), tag_tone("Staging"));
    }

    #[test]
    fn tag_suggestions_match_anywhere_and_put_prefix_matches_first() {
        let known: Vec<String> = ["backstage", "prod", "staging", "Stable"].map(String::from).to_vec();
        assert_eq!(tag_suggestions(&known, "sta"), ["staging", "Stable", "backstage"]);
        assert_eq!(tag_suggestions(&known, "PROD"), ["prod"]);
        assert!(tag_suggestions(&known, " ").is_empty());
        assert!(tag_suggestions(&known, "qa").is_empty());
    }
}
