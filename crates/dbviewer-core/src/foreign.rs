use crate::session::TableRef;

/// A single-column foreign key; composite keys aren't offered in the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    pub column: String,
    pub target: TableRef,
    pub target_column: String,
}

const PREFERRED_LABELS: [&str; 10] = ["name", "full_name", "display_name", "title", "label", "company_name", "username", "email", "code", "description"];

fn label_index(columns: &[String], key: &str) -> Option<usize> {
    let lower: Vec<String> = columns.iter().map(|c| c.to_lowercase()).collect();
    let usable = |i: &usize| !columns[*i].eq_ignore_ascii_case(key);
    PREFERRED_LABELS
        .iter()
        .find_map(|preferred| lower.iter().position(|c| c == preferred))
        .or_else(|| (0..columns.len()).filter(usable).find(|&i| lower[i].contains("name")))
        .or_else(|| (0..columns.len()).find(usable))
}

/// The key first, then the best label column, then the rest, at most `max` in all.
pub fn display_columns(columns: &[String], key: &str, max: usize) -> Vec<usize> {
    let key_index = columns.iter().position(|c| c == key);
    let label = label_index(columns, key);
    let mut order: Vec<usize> = key_index.into_iter().chain(label).collect();
    order.dedup();
    let rest: Vec<usize> = (0..columns.len()).filter(|i| !order.contains(i) && !is_sensitive(&columns[*i])).collect();
    order.extend(rest);
    order.truncate(max);
    order
}

fn is_sensitive(column: &str) -> bool {
    let column = column.to_lowercase();
    ["password", "secret", "token", "hash"].iter().any(|word| column.contains(word))
}

/// A human-readable name for a referenced row, or `None` when it has nothing but the key.
pub fn row_label(columns: &[String], row: &[Option<String>], key: &str) -> Option<String> {
    let index = label_index(columns, key)?;
    row.get(index)?.clone().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prefers_name_like_columns() {
        let columns = cols(&["id", "created_at", "company_name", "title"]);
        assert_eq!(display_columns(&columns, "id", 3), vec![0, 3, 1]);
        let row = vec![Some("7".into()), None, Some("Acme".into()), Some("T".into())];
        assert_eq!(row_label(&columns, &row, "id"), Some("T".into()));
    }

    #[test]
    fn falls_back_to_first_non_key_column() {
        let columns = cols(&["id", "slug", "created_at"]);
        assert_eq!(display_columns(&columns, "id", 2), vec![0, 1]);
        assert_eq!(row_label(&columns, &[Some("1".into()), Some("x".into()), None], "id"), Some("x".into()));
    }

    #[test]
    fn hides_secret_columns() {
        let columns = cols(&["id", "name", "password", "api_token", "email"]);
        assert_eq!(display_columns(&columns, "id", 5), vec![0, 1, 4]);
    }

    #[test]
    fn key_only_table_has_no_label() {
        let columns = cols(&["id"]);
        assert_eq!(row_label(&columns, &[Some("1".into())], "id"), None);
    }
}
