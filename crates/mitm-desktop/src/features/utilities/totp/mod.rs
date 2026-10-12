mod view;

pub use view::{qr_buttons, show, show_code};

#[derive(Default)]
pub struct TotpState {
    pub input: String,
    pub qr_error: Option<String>,
}

/// "482913" → "482 913"; other lengths split in half the same way.
pub fn group_code(code: &str) -> String {
    if code.len() < 6 {
        return code.to_string();
    }
    let mid = code.len() / 2;
    format!("{} {}", &code[..mid], &code[mid..])
}
