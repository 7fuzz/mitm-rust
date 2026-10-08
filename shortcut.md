# Keyboard Shortcuts

`Ctrl` is `Cmd` on macOS. The same list is in the app under **Settings ▸ Shortcuts**, generated from `crates/mitm-desktop/src/shortcuts.rs`.

Anything without a shortcut is in the command palette (`Ctrl + K`): every tab and utility, proxy modes, import and export.

## Global

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + K` | Command palette |
| `Ctrl + E` | Quick variables for the active environment (Repeater's own when Repeater has focus) |
| `Ctrl + ,` | Open Settings |
| `Ctrl + Tab` | Next tab |
| `Ctrl + Shift + Tab` | Previous tab |
| `Alt + 1` … `Alt + 9` | Focus tab by position |
| `Ctrl + +` / `Ctrl + -` / `Ctrl + 0` | Zoom in, zoom out, reset zoom |

## Intercept

These only fire while the Intercept tab has focus.

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + Enter` | Forward with changes |
| `Ctrl + Shift + Enter` | Forward unmodified |
| `Ctrl + Backspace` | Drop (ignored while typing in a field) |

## Request fields

In URL, header, parameter and body fields of Repeater, Collections, Fuzzer and Intercept:

| Shortcut | Action |
| :--- | :--- |
| `{{` | Variable picker |
| `↑` / `↓`, `Enter` or `Tab`, `Esc` | Move, insert, dismiss in the picker |
| `Ctrl + M` | Wrap the selection in `{{ }}` (mark a fuzz variable) |

Right-click a field for **Decode in Encoding tool**.

## Command palette

| Shortcut | Action |
| :--- | :--- |
| `↑` / `↓` | Move |
| `Enter` | Run |
| `Esc` | Close |
