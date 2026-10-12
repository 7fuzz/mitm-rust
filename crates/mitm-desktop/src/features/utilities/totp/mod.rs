mod view;

pub use view::show;

#[derive(Default)]
pub struct TotpState {
    pub input: String,
}

/// "482913" → "482 913"; other lengths split in half the same way.
pub fn group_code(code: &str) -> String {
    if code.len() < 6 {
        return code.to_string();
    }
    let mid = code.len() / 2;
    format!("{} {}", &code[..mid], &code[mid..])
}
