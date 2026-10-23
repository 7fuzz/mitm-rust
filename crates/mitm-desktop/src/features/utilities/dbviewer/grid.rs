use dbviewer_core::{QueryResult, Sort};

use super::fk::{FkLabel, FkState};
use egui::RichText;
use egui_extras::{Column, TableBuilder};

pub enum EditAccess {
    Hidden,
    Disabled(&'static str),
    Enabled,
}

#[derive(Default)]
pub struct GridAction {
    pub sort_by: Option<String>,
    pub edit_row: Option<usize>,
    pub pick_row: Option<usize>,
    /// Foreign key cells hovered whose referenced row hasn't been looked up: (column, value).
    pub lookups: Vec<(String, String)>,
}

pub struct GridOptions<'a> {
    pub sort: Option<&'a Sort>,
    pub sortable: bool,
    pub edit: &'a EditAccess,
    pub fk: Option<&'a FkState>,
    /// A click on a row picks it instead of the double-click edit.
    pub pick: bool,
}

pub fn show(ui: &mut egui::Ui, result: &QueryResult, opts: &GridOptions) -> GridAction {
    let GridOptions { sort, sortable, .. } = *opts;
    let mut action = GridAction::default();
    if result.columns.is_empty() {
        let message = match result.affected {
            Some(n) => format!("OK, {n} row(s) affected"),
            None => "No results".to_string(),
        };
        ui.label(RichText::new(message).weak());
        return action;
    }

    egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .auto_shrink([false, false])
            .column(Column::exact(48.0));
        for _ in &result.columns {
            table = table.column(Column::initial(150.0).at_least(60.0).clip(true));
        }
        table
            .header(24.0, |mut header| {
                header.col(|_| {});
                for name in &result.columns {
                    header.col(|ui| {
                        let direction = sort.filter(|s| s.column == *name).map(|s| s.descending);
                        let sense = if sortable { egui::Sense::click() } else { egui::Sense::hover() };
                        let response = ui
                            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let arrow_response = match direction {
                                    Some(descending) => sort_arrow(ui, descending, sense),
                                    None => ui.allocate_response(egui::Vec2::ZERO, sense),
                                };
                                let name_response = ui.add(egui::Label::new(RichText::new(name).strong()).truncate().sense(sense));
                                arrow_response.union(name_response)
                            })
                            .inner;
                        if sortable && response.on_hover_text("Click to sort").clicked() {
                            action.sort_by = Some(name.clone());
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, result.rows.len(), |mut row| {
                    let index = row.index();
                    row.col(|ui| {
                        ui.label(RichText::new((index + 1).to_string()).weak().small());
                    });
                    for (cell, column) in result.rows[index].iter().zip(&result.columns) {
                        row.col(|ui| cell_ui(ui, index, column, cell.as_deref(), opts, &mut action));
                    }
                });
            });
    });
    action
}

fn cell_ui(ui: &mut egui::Ui, row: usize, column: &str, value: Option<&str>, opts: &GridOptions, action: &mut GridAction) {
    let text = match value {
        Some(v) => RichText::new(v.lines().next().unwrap_or("")),
        None => RichText::new("NULL").weak().italics(),
    };
    let response = ui.add(egui::Label::new(text).truncate().show_tooltip_when_elided(false).sense(egui::Sense::click()));
    if let Some(v) = value {
        response.clone().on_hover_ui(|ui| {
            ui.label(v.chars().take(500).collect::<String>());
            let Some(fk) = opts.fk.and_then(|state| state.key_for(column).map(|fk| (state, fk))) else { return };
            let (state, fk) = fk;
            match state.label(column, v) {
                Some(FkLabel::Found(label)) => ui.label(RichText::new(format!("-> {}: {label}", fk.target.name)).strong()),
                Some(FkLabel::Missing) => ui.label(RichText::new(format!("-> {}: not found", fk.target.name)).weak()),
                Some(FkLabel::Loading) => ui.label(RichText::new("looking up...").weak()),
                None => {
                    action.lookups.push((column.to_string(), v.to_string()));
                    ui.label(RichText::new("looking up...").weak())
                }
            };
        });
    }
    if opts.pick && response.clicked() {
        action.pick_row = Some(row);
    }
    if response.double_clicked() && !opts.pick && matches!(opts.edit, EditAccess::Enabled) {
        action.edit_row = Some(row);
    }
    response.context_menu(|ui| {
        if ui.button("Copy value").clicked() {
            ui.ctx().copy_text(value.unwrap_or("NULL").to_string());
            ui.close();
        }
        match opts.edit {
            EditAccess::Hidden => {}
            EditAccess::Disabled(reason) => {
                ui.add_enabled(false, egui::Button::new("Edit row...")).on_disabled_hover_text(*reason);
            }
            EditAccess::Enabled => {
                if ui.button("Edit row...").clicked() {
                    action.edit_row = Some(row);
                    ui.close();
                }
            }
        }
    });
}

fn sort_arrow(ui: &mut egui::Ui, descending: bool, sense: egui::Sense) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), sense);
    let c = rect.center();
    let (tip, base) = if descending { (c.y + 4.0, c.y - 3.0) } else { (c.y - 4.0, c.y + 3.0) };
    let points = vec![egui::pos2(c.x, tip), egui::pos2(c.x - 4.0, base), egui::pos2(c.x + 4.0, base)];
    let color = ui.visuals().strong_text_color();
    ui.painter().add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
    response
}
