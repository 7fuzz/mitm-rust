use eframe::egui::{self, RichText};
use crate::util::format::format_bytes;
use crate::ui::theme::{self, Tone};
use super::message_pane::SaveBody;

pub struct FormField {
    pub key: String,
    pub value: FormValue,
}

pub enum FormValue {
    Text(String),
    File { name: String, content_type: String, bytes: Vec<u8> },
}

/// Fields of a form body as rows; `a[b][c]` keys nest as collapsing groups. Returns a file part to save.
pub fn show(ui: &mut egui::Ui, id_salt: &str, fields: &[FormField]) -> Option<SaveBody> {
    let mut nodes = Vec::new();
    for field in fields {
        insert(&mut nodes, &key_path(&field.key), field);
    }
    let mut save = None;
    egui::ScrollArea::both().id_salt((id_salt, "form")).auto_shrink([false, false]).show(ui, |ui| {
        show_nodes(ui, &nodes, &mut save);
    });
    save
}

struct Node<'a> {
    name: &'a str,
    field: Option<&'a FormField>,
    children: Vec<Node<'a>>,
}

fn key_path(key: &str) -> Vec<&str> {
    match key.find('[') {
        Some(open) if open > 0 && key.ends_with(']') => {
            let mut path = vec![&key[..open]];
            path.extend(key[open + 1..key.len() - 1].split("]["));
            path
        }
        _ => vec![key],
    }
}

fn insert<'a>(nodes: &mut Vec<Node<'a>>, path: &[&'a str], field: &'a FormField) {
    let Some((&name, rest)) = path.split_first() else { return };
    if rest.is_empty() {
        nodes.push(Node { name, field: Some(field), children: Vec::new() });
        return;
    }
    let group = match nodes.iter().position(|n| n.field.is_none() && n.name == name) {
        Some(index) => index,
        None => {
            nodes.push(Node { name, field: None, children: Vec::new() });
            nodes.len() - 1
        }
    };
    insert(&mut nodes[group].children, rest, field);
}

fn show_nodes(ui: &mut egui::Ui, nodes: &[Node], save: &mut Option<SaveBody>) {
    for (index, node) in nodes.iter().enumerate() {
        ui.push_id(index, |ui| {
            let name = if node.name.is_empty() { "[]" } else { node.name };
            match node.field {
                Some(field) => row(ui, name, &field.value, save),
                None => {
                    let title = RichText::new(name).color(theme::color(ui, Tone::Info)).strong();
                    egui::CollapsingHeader::new(title).default_open(true).show(ui, |ui| show_nodes(ui, &node.children, save));
                }
            }
        });
    }
}

fn row(ui: &mut egui::Ui, name: &str, value: &FormValue, save: &mut Option<SaveBody>) {
    ui.horizontal(|ui| {
        let key = RichText::new(name).color(theme::color(ui, Tone::Info)).strong();
        ui.add_sized([140.0, ui.spacing().interact_size.y], egui::Label::new(key).truncate());
        match value {
            FormValue::Text(text) => {
                ui.label(RichText::new(text).monospace());
            }
            FormValue::File { name, content_type, bytes } => {
                ui.label(RichText::new(format!("📎 {name}")).monospace());
                ui.weak(format!("{} · {content_type}", format_bytes(bytes.len() as u64)));
                if ui.small_button("Save…").clicked() {
                    *save = Some(SaveBody { file_name: name.clone(), bytes: bytes.clone() });
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_keys_split_into_a_path() {
        assert_eq!(key_path("a[b][c]"), ["a", "b", "c"]);
        assert_eq!(key_path("tags[]"), ["tags", ""]);
        assert_eq!(key_path("plain"), ["plain"]);
        assert_eq!(key_path("[odd]"), ["[odd]"]);
    }

    #[test]
    fn fields_sharing_a_prefix_group_together() {
        let text = |key: &str| FormField { key: key.to_string(), value: FormValue::Text(String::new()) };
        let fields = [text("user[name]"), text("user[address][city]"), text("page")];
        let mut nodes = Vec::new();
        for field in &fields {
            insert(&mut nodes, &key_path(&field.key), field);
        }
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "user");
        assert_eq!(nodes[0].children.len(), 2);
        assert_eq!(nodes[0].children[1].children[0].name, "city");
    }
}
