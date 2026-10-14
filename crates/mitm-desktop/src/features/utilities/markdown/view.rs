use std::sync::{Arc, Mutex};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use crate::app::command::{AppCommand, Commands};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::message_pane::SaveBody;
use crate::widgets::segmented::segmented;
use crate::widgets::split;
use super::{mermaid, segments, MarkdownState, Segment, ViewMode};

pub fn show(ui: &mut egui::Ui, state: &mut MarkdownState, cmds: &mut Commands) {
    toolbar(ui, state, cmds);
    ui.add_space(6.0);

    match state.prefs.view {
        ViewMode::Editor => editor(ui, &mut state.source),
        ViewMode::Preview => preview(ui, state),
        ViewMode::Split => {
            let mut fraction = state.prefs.split;
            let [mut first, mut second] = split::oriented(ui, "md_split", &mut fraction, state.prefs.stacked);
            state.prefs.split = fraction;
            editor(&mut first, &mut state.source);
            preview(&mut second, state);
        }
    }
}

fn toolbar(ui: &mut egui::Ui, state: &mut MarkdownState, cmds: &mut Commands) {
    ui.horizontal_wrapped(|ui| {
        segmented(
            ui,
            &mut state.prefs.view,
            &[(ViewMode::Editor, "Editor"), (ViewMode::Split, "Split"), (ViewMode::Preview, "Preview")],
        );
        if state.prefs.view == ViewMode::Split {
            ui.add_space(4.0);
            let hint = if state.prefs.stacked { "Side by side" } else { "Stacked" };
            if icon_button(ui, icons::ROWS, hint).clicked() {
                state.prefs.stacked = !state.prefs.stacked;
            }
        }
        ui.add_space(8.0);
        ui.checkbox(&mut state.prefs.mermaid, "Mermaid").on_hover_text("Render mermaid code blocks as diagrams");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear").clicked() {
                state.source.clear();
                state.file_name = None;
            }
            if ui.button("Sample").clicked() {
                state.source = SAMPLE.to_string();
            }
            if icon_button(ui, icons::COPY, "Copy source").clicked() {
                ui.ctx().copy_text(state.source.clone());
            }
            if ui.button("Open").clicked() {
                cmds.push(AppCommand::PickMarkdownFile);
            }
            if ui.add_enabled(!state.source.is_empty(), egui::Button::new("Save")).clicked() {
                cmds.push(AppCommand::SaveBody(SaveBody {
                    file_name: state.save_name(),
                    bytes: state.source.clone().into_bytes(),
                }));
            }
        });
    });
}

fn editor(ui: &mut egui::Ui, source: &mut String) {
    egui::ScrollArea::both().id_salt("md_editor").auto_shrink([false, false]).show(ui, |ui| {
        ui.add_sized(
            ui.available_size(),
            egui::TextEdit::multiline(source).code_editor().desired_width(f32::INFINITY),
        );
    });
}

fn preview(ui: &mut egui::Ui, state: &MarkdownState) {
    egui::ScrollArea::vertical().id_salt("md_preview").auto_shrink([false, false]).show(ui, |ui| {
        // Cap the content to the pane so regular text wraps; tables and diagrams get their own
        // horizontal scroll instead of widening the whole preview.
        let width = ui.available_width();
        ui.set_max_width(width);
        if state.source.trim().is_empty() {
            ui.weak("Nothing to preview");
            return;
        }
        for (i, segment) in segments(&state.source).into_iter().enumerate() {
            match segment {
                Segment::Markdown(text) => commonmark(ui, i, text),
                Segment::Table(md) => {
                    side_scroll(ui, ("md_table", i), width, |ui| commonmark(ui, i, md));
                }
                Segment::Mermaid(code) if state.prefs.mermaid => {
                    ui.push_id(("md_mermaid", i), |ui| mermaid::render(ui, "diagram", code));
                    ui.add_space(6.0);
                }
                Segment::Mermaid(code) => {
                    side_scroll(ui, ("md_mermaid_src", i), width, |ui| {
                        commonmark(ui, i, &format!("```\n{}\n```", code));
                    });
                }
            }
        }
    });
}

/// Puts wide content (a table, a code block) in its own horizontal scroll area bounded to the pane.
fn side_scroll(ui: &mut egui::Ui, salt: impl egui::AsIdSalt, width: f32, content: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::horizontal().id_salt(salt).max_width(width).show(ui, content);
}

fn commonmark(ui: &mut egui::Ui, salt: usize, text: &str) {
    // The cache isn't Clone, which egui's temp storage requires.
    let cache = ui.data_mut(|d| d.get_temp_mut_or_default::<Arc<Mutex<CommonMarkCache>>>(egui::Id::new("md_util_cache")).clone());
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    ui.push_id(("md_segment", salt), |ui| {
        CommonMarkViewer::new().show(ui, &mut cache, text);
    });
}

const SAMPLE: &str = r#"# Markdown util

A **live** preview with side-by-side _raw_ and parsed views.

## Features

- Headings, lists, and `inline code`
- Tables
- [Links](https://example.com)
- Mermaid diagrams

| Tool | Purpose |
| ---- | ------- |
| Repeater | Replay requests |
| Fuzzer | Vary inputs |

```rust
fn main() {
    println!("regular code stays code");
}
```

## Flowchart

```mermaid
flowchart TD
    A[Client] -->|request| B{Proxy}
    B -->|match rule| C[Rewrite]
    B -->|pass| D[Upstream]
    C --> D
    D -.response.-> A
```

## Sequence

```mermaid
sequenceDiagram
    participant C as Client
    participant P as Proxy
    participant S as Server
    C->>P: GET /login
    P->>S: GET /login
    S-->>P: 200 OK
    P-->>C: 200 OK
    Note over C,P: TLS intercepted
```
"#;
