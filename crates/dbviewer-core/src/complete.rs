use std::collections::HashMap;

use crate::engine::Engine;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Table,
    Column,
    Database,
    Keyword,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Table => "table",
            Kind::Column => "column",
            Kind::Database => "database",
            Kind::Keyword => "keyword",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    /// Text to insert, already quoted when the name needs it.
    pub text: String,
    pub kind: Kind,
}

/// What the editor knows about the connected server, for one query tab.
pub struct Schema<'a> {
    pub engine: Engine,
    pub databases: &'a [String],
    pub tables: &'a [String],
    /// Column names per table name.
    pub columns: &'a HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    /// Char index where the word being completed starts.
    pub start: usize,
    pub items: Vec<Suggestion>,
}

const MAX_ITEMS: usize = 10;

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Suggestions for the word ending at char index `cursor`, or `None` when there is nothing to offer.
pub fn complete(sql: &str, cursor: usize, schema: &Schema) -> Option<Completion> {
    let chars: Vec<char> = sql.chars().collect();
    let cursor = cursor.min(chars.len());
    let mut start = cursor;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let prefix: String = chars[start..cursor].iter().collect();
    if prefix.is_empty() || prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let lower = prefix.to_lowercase();

    let qualifier = (start > 0 && chars[start - 1] == '.').then(|| {
        let mut q_start = start - 1;
        while q_start > 0 && is_word(chars[q_start - 1]) {
            q_start -= 1;
        }
        chars[q_start..start - 1].iter().collect::<String>().to_lowercase()
    });

    let mut items: Vec<Suggestion> = Vec::new();
    let push = |name: &str, kind: Kind, items: &mut Vec<Suggestion>| {
        if name.to_lowercase().starts_with(&lower) && !items.iter().any(|s| s.text.trim_matches(['"', '`']) == name) {
            items.push(Suggestion { text: quote_if_needed(schema.engine, name), kind });
        }
    };

    match qualifier.as_deref() {
        Some(table) => {
            if let Some(columns) = schema.columns.iter().find(|(name, _)| name.to_lowercase() == table).map(|(_, c)| c) {
                columns.iter().for_each(|c| push(c, Kind::Column, &mut items));
            }
            if schema.databases.iter().any(|d| d.to_lowercase() == table) {
                schema.tables.iter().for_each(|t| push(t, Kind::Table, &mut items));
            }
        }
        None => {
            schema.tables.iter().for_each(|t| push(t, Kind::Table, &mut items));
            let words = mentioned_words(&chars, start);
            for (table, columns) in schema.columns {
                if words.contains(&table.to_lowercase()) {
                    columns.iter().for_each(|c| push(c, Kind::Column, &mut items));
                }
            }
            schema.databases.iter().for_each(|d| push(d, Kind::Database, &mut items));
            let upper = prefix.chars().any(|c| c.is_alphabetic()) && prefix.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase);
            for keyword in schema.engine.keywords() {
                if keyword.to_lowercase().starts_with(&lower) {
                    let text = if upper { keyword.to_string() } else { keyword.to_lowercase() };
                    items.push(Suggestion { text, kind: Kind::Keyword });
                }
            }
        }
    }

    items.retain(|s| s.text != prefix);
    items.sort_by_key(|s| (s.kind as u8, s.text.len(), s.text.to_lowercase()));
    items.truncate(MAX_ITEMS);
    (!items.is_empty()).then_some(Completion { start, items })
}

/// Lowercased words in the statement, excluding the one being typed.
fn mentioned_words(chars: &[char], current_start: usize) -> Vec<String> {
    let text: String = chars[..current_start].iter().chain(chars[current_start..].iter().skip_while(|c| is_word(**c))).collect();
    text.split(|c: char| !is_word(c)).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

fn quote_if_needed(engine: Engine, name: &str) -> String {
    let plain = name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') && !name.starts_with(|c: char| c.is_ascii_digit());
    let plain_mysql = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let needs = match engine {
        Engine::Postgres => !plain,
        Engine::MySql | Engine::MariaDb => !plain_mysql,
    };
    if needs {
        engine.quote_ident(name)
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema<'a>(databases: &'a [String], tables: &'a [String], columns: &'a HashMap<String, Vec<String>>) -> Schema<'a> {
        Schema { engine: Engine::Postgres, databases, tables, columns }
    }

    fn fixture() -> (Vec<String>, Vec<String>, HashMap<String, Vec<String>>) {
        let tables = vec!["users".to_string(), "user_roles".to_string(), "Orders".to_string()];
        let columns = HashMap::from([("users".to_string(), vec!["id".to_string(), "email".to_string()])]);
        (vec!["shop".to_string()], tables, columns)
    }

    fn texts(c: &Completion) -> Vec<&str> {
        c.items.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn completes_tables_before_keywords() {
        let (d, t, c) = fixture();
        let sql = "select * from us";
        let result = complete(sql, sql.len(), &schema(&d, &t, &c)).unwrap();
        assert_eq!(result.start, 14);
        assert_eq!(texts(&result), vec!["users", "user_roles", "using"]);
    }

    #[test]
    fn completes_columns_after_a_dot_and_for_mentioned_tables() {
        let (d, t, c) = fixture();
        let sql = "select users.em";
        assert_eq!(texts(&complete(sql, sql.len(), &schema(&d, &t, &c)).unwrap()), vec!["email"]);
        let sql = "select em from users";
        assert_eq!(texts(&complete(sql, 9, &schema(&d, &t, &c)).unwrap()), vec!["email"]);
        let sql = "select em from orders";
        assert!(complete(sql, 9, &schema(&d, &t, &c)).is_none());
    }

    #[test]
    fn keywords_follow_the_typed_case_and_names_are_quoted_when_needed() {
        let (d, t, c) = fixture();
        let sql = "SEL";
        assert_eq!(texts(&complete(sql, 3, &schema(&d, &t, &c)).unwrap()), vec!["SELECT"]);
        let sql = "select * from ord";
        assert_eq!(texts(&complete(sql, sql.len(), &schema(&d, &t, &c)).unwrap()), vec!["\"Orders\"", "order by"]);
    }

    #[test]
    fn nothing_for_empty_numeric_or_complete_words() {
        let (d, t, c) = fixture();
        assert!(complete("select ", 7, &schema(&d, &t, &c)).is_none());
        assert!(complete("limit 10", 8, &schema(&d, &t, &c)).is_none());
        assert!(complete("from users", 10, &schema(&d, &t, &c)).is_none());
    }
}
