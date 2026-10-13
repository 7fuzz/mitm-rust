//! Phosphor Regular glyphs, embedded as a font so they work in any label, button or hint text.
//! Use them sparingly: compact controls and state only, never in front of a labelled button or menu item.
#![allow(dead_code)] // the set we draw from, not a list of glyphs in use

pub const ARROW_BEND_UP_RIGHT: &str = "\u{E026}";
pub const ARROW_CLOCKWISE: &str = "\u{E036}";
pub const ARROW_DOWN_LEFT: &str = "\u{E040}";
pub const ARROW_RIGHT: &str = "\u{E06C}";
pub const ARROW_SQUARE_IN: &str = "\u{E5DC}";
pub const ARROW_SQUARE_OUT: &str = "\u{E5DE}";
pub const ARROW_UP_RIGHT: &str = "\u{E092}";
pub const ARROWS_LEFT_RIGHT: &str = "\u{E0A0}";
pub const BELL: &str = "\u{E0CE}";
pub const CARET_DOUBLE_LEFT: &str = "\u{E128}";
pub const CARET_DOUBLE_RIGHT: &str = "\u{E12A}";
pub const CARET_DOWN: &str = "\u{E136}";
pub const CARET_LEFT: &str = "\u{E138}";
pub const CARET_RIGHT: &str = "\u{E13A}";
pub const CARET_UP: &str = "\u{E13C}";
pub const CHECK: &str = "\u{E182}";
pub const CLOCK_COUNTER_CLOCKWISE: &str = "\u{E1A0}";
pub const COPY: &str = "\u{E1CA}";
pub const DOTS_SIX_VERTICAL: &str = "\u{EAE2}";
pub const DOTS_THREE: &str = "\u{E1FE}";
pub const DOWNLOAD_SIMPLE: &str = "\u{E20C}";
pub const EXPORT: &str = "\u{EAF0}";
pub const EYE: &str = "\u{E220}";
pub const EYE_SLASH: &str = "\u{E224}";
pub const FOLDER: &str = "\u{E24A}";
pub const FUNNEL: &str = "\u{E266}";
pub const GEAR: &str = "\u{E270}";
pub const IMAGE: &str = "\u{E2CA}";
pub const LIGHTNING: &str = "\u{E2DE}";
pub const LINK: &str = "\u{E2E2}";
pub const LOCK: &str = "\u{E2FA}";
pub const LOCK_OPEN: &str = "\u{E306}";
pub const MAGNIFYING_GLASS: &str = "\u{E30C}";
pub const MOON: &str = "\u{E330}";
pub const PAPER_PLANE_RIGHT: &str = "\u{E396}";
pub const PAPERCLIP: &str = "\u{E39A}";
pub const PENCIL_SIMPLE: &str = "\u{E3B4}";
pub const PLAY: &str = "\u{E3D0}";
pub const PLUS: &str = "\u{E3D4}";
pub const QR_CODE: &str = "\u{E3E6}";
pub const ROWS: &str = "\u{E5A2}";
pub const STOP: &str = "\u{E46C}";
pub const SUN: &str = "\u{E472}";
pub const TRASH: &str = "\u{E4A6}";
pub const WARNING: &str = "\u{E4E0}";
pub const X: &str = "\u{E4F6}";

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    fn is_emoji(c: char) -> bool {
        matches!(c as u32, 0x1F300..=0x1FAFF | 0x2600..=0x27BF)
    }

    #[test]
    fn ui_strings_use_icons_not_emoji() {
        let mut files = Vec::new();
        rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let offenders: Vec<String> = files
            .iter()
            .flat_map(|file| {
                let text = std::fs::read_to_string(file).unwrap();
                text.lines()
                    .enumerate()
                    .filter(|(_, line)| !line.trim_start().starts_with("//") && line.chars().any(is_emoji))
                    .map(|(n, line)| format!("{}:{}: {}", file.display(), n + 1, line.trim()))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(offenders.is_empty(), "use ui::icons instead of emoji:\n{}", offenders.join("\n"));
    }
}
