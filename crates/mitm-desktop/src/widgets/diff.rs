use std::time::Duration;
use eframe::egui::{self, RichText};
use similar::{ChangeTag, TextDiff};
use crate::theme::Tone;

pub struct DiffLine {
    pub tag: ChangeTag,
    pub text: String,
}

/// A line diff; the timeout keeps huge bodies from stalling the UI at the cost of a coarser diff.
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    TextDiff::configure()
        .timeout(Duration::from_millis(200))
        .diff_lines(before, after)
        .iter_all_changes()
        .map(|change| DiffLine { tag: change.tag(), text: change.value().trim_end_matches('\n').to_string() })
        .collect()
}

pub fn show(ui: &mut egui::Ui, id_salt: &str, lines: &[DiffLine]) {
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::both().id_salt(id_salt).max_height(260.0).auto_shrink([false, true]).show_rows(ui, row_height, lines.len(), |ui, rows| {
        for line in &lines[rows] {
            let (prefix, tone) = match line.tag {
                ChangeTag::Insert => ("+ ", Some(Tone::Success)),
                ChangeTag::Delete => ("- ", Some(Tone::Danger)),
                ChangeTag::Equal => ("  ", None),
            };
            let mut text = RichText::new(format!("{prefix}{}", line.text)).monospace();
            if let Some(tone) = tone {
                text = text.color(tone.color(ui.visuals())).background_color(tone.tint(ui.visuals(), 28));
            }
            ui.label(text);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_lines_are_tagged() {
        let lines = diff_lines("a\nb\nc\n", "a\nB\nc\n");
        let tags: Vec<ChangeTag> = lines.iter().map(|l| l.tag).collect();
        assert_eq!(tags, [ChangeTag::Equal, ChangeTag::Delete, ChangeTag::Insert, ChangeTag::Equal]);
        assert_eq!(lines[2].text, "B");
    }
}
