use crate::engine::Engine;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Gt,
    Gte,
    Lt,
    Lte,
    IsNull,
    NotNull,
}

impl Op {
    pub const ALL: [Op; 12] = [
        Op::Equals,
        Op::NotEquals,
        Op::Contains,
        Op::NotContains,
        Op::StartsWith,
        Op::EndsWith,
        Op::Gt,
        Op::Gte,
        Op::Lt,
        Op::Lte,
        Op::IsNull,
        Op::NotNull,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Op::Equals => "equals",
            Op::NotEquals => "does not equal",
            Op::Contains => "contains",
            Op::NotContains => "does not contain",
            Op::StartsWith => "starts with",
            Op::EndsWith => "ends with",
            Op::Gt => ">",
            Op::Gte => ">=",
            Op::Lt => "<",
            Op::Lte => "<=",
            Op::IsNull => "is NULL",
            Op::NotNull => "is not NULL",
        }
    }

    pub fn needs_value(self) -> bool {
        !matches!(self, Op::IsNull | Op::NotNull)
    }

    fn is_negative(self) -> bool {
        matches!(self, Op::NotEquals | Op::NotContains | Op::NotNull)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition {
    /// `None` matches against every column, as text.
    pub column: Option<String>,
    pub op: Op,
    pub value: String,
}

impl Default for Condition {
    fn default() -> Self {
        Self { column: None, op: Op::Contains, value: String::new() }
    }
}

impl Condition {
    pub fn is_active(&self) -> bool {
        !self.op.needs_value() || !self.value.is_empty()
    }

    fn sql(&self, engine: Engine, columns: &[String]) -> Option<String> {
        if !self.is_active() {
            return None;
        }
        match &self.column {
            Some(column) => Some(self.column_sql(engine, &engine.quote_ident(column), true)),
            None if columns.is_empty() => None,
            None => {
                let parts: Vec<String> = columns.iter().map(|c| self.column_sql(engine, &engine.quote_ident(c), false)).collect();
                let glue = if self.op.is_negative() { " AND " } else { " OR " };
                Some(format!("({})", parts.join(glue)))
            }
        }
    }

    /// `typed` keeps the column's own type for comparisons so numbers order numerically.
    fn column_sql(&self, engine: Engine, column: &str, typed: bool) -> String {
        let lhs = if typed { column.to_string() } else { engine.as_text(column) };
        let text = engine.as_text(column);
        let (like, not_like) = (engine.like(), engine.not_like());
        let value = &self.value;
        match self.op {
            Op::Equals => format!("{lhs} = {}", engine.literal(value)),
            Op::NotEquals => format!("{lhs} <> {}", engine.literal(value)),
            Op::Gt => format!("{lhs} > {}", engine.literal(value)),
            Op::Gte => format!("{lhs} >= {}", engine.literal(value)),
            Op::Lt => format!("{lhs} < {}", engine.literal(value)),
            Op::Lte => format!("{lhs} <= {}", engine.literal(value)),
            Op::Contains => format!("{text} {like} {} ESCAPE '!'", engine.literal(&format!("%{}%", escape_like(value)))),
            Op::NotContains => format!("{text} {not_like} {} ESCAPE '!'", engine.literal(&format!("%{}%", escape_like(value)))),
            Op::StartsWith => format!("{text} {like} {} ESCAPE '!'", engine.literal(&format!("{}%", escape_like(value)))),
            Op::EndsWith => format!("{text} {like} {} ESCAPE '!'", engine.literal(&format!("%{}", escape_like(value)))),
            Op::IsNull => format!("{column} IS NULL"),
            Op::NotNull => format!("{column} IS NOT NULL"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Join {
    #[default]
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sort {
    pub column: String,
    pub descending: bool,
}

/// Newest first by `updated_at`, then `created_at`; otherwise `name`, `id`, or the first column.
pub fn default_sort(columns: &[String]) -> Option<Sort> {
    let find = |wanted: &str| columns.iter().find(|c| c.eq_ignore_ascii_case(wanted));
    let newest = ["updated_at", "created_at"].into_iter().find_map(|w| find(w).map(|c| Sort { column: c.clone(), descending: true }));
    newest
        .or_else(|| ["name", "id"].into_iter().find_map(|w| find(w).map(|c| Sort { column: c.clone(), descending: false })))
        .or_else(|| columns.first().map(|c| Sort { column: c.clone(), descending: false }))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub conditions: Vec<Condition>,
    pub join: Join,
    pub sort: Option<Sort>,
}

impl Filter {
    pub fn active_conditions(&self) -> usize {
        self.conditions.iter().filter(|c| c.is_active()).count()
    }

    /// ` WHERE … ORDER BY …`, or an empty string. `columns` backs the "any column" conditions.
    pub fn sql_suffix(&self, engine: Engine, columns: &[String]) -> String {
        let clauses: Vec<String> = self.conditions.iter().filter_map(|c| c.sql(engine, columns)).collect();
        let mut sql = String::new();
        if !clauses.is_empty() {
            let glue = match self.join {
                Join::And => " AND ",
                Join::Or => " OR ",
            };
            sql.push_str(&format!(" WHERE {}", clauses.join(glue)));
        }
        if let Some(sort) = &self.sort {
            sql.push_str(&format!(" ORDER BY {}", engine.order_by(&engine.quote_ident(&sort.column), sort.descending)));
        }
        sql
    }
}

fn escape_like(value: &str) -> String {
    value.replace('!', "!!").replace('%', "!%").replace('_', "!_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cond(column: Option<&str>, op: Op, value: &str) -> Condition {
        Condition { column: column.map(String::from), op, value: value.into() }
    }

    #[test]
    fn empty_filter_adds_nothing() {
        assert_eq!(Filter::default().sql_suffix(Engine::Postgres, &[]), "");
        let blank = Filter { conditions: vec![Condition::default()], ..Filter::default() };
        assert_eq!(blank.sql_suffix(Engine::Postgres, &["a".into()]), "");
    }

    #[test]
    fn exact_and_contains_joined_with_or() {
        let filter = Filter {
            conditions: vec![cond(Some("company_id"), Op::Equals, "42"), cond(Some("company_name"), Op::Contains, "ac_me")],
            join: Join::Or,
            sort: None,
        };
        assert_eq!(
            filter.sql_suffix(Engine::Postgres, &[]),
            " WHERE \"company_id\" = E'42' OR \"company_name\"::text ILIKE E'%ac!_me%' ESCAPE '!'"
        );
    }

    #[test]
    fn any_column_searches_every_column() {
        let filter = Filter { conditions: vec![cond(None, Op::Contains, "x")], ..Filter::default() };
        let sql = filter.sql_suffix(Engine::Postgres, &["a".into(), "b".into()]);
        assert_eq!(sql, " WHERE (\"a\"::text ILIKE E'%x%' ESCAPE '!' OR \"b\"::text ILIKE E'%x%' ESCAPE '!')");
    }

    #[test]
    fn negative_any_column_requires_all() {
        let filter = Filter { conditions: vec![cond(None, Op::NotContains, "x")], ..Filter::default() };
        assert!(filter.sql_suffix(Engine::Postgres, &["a".into(), "b".into()]).contains(" AND "));
    }

    #[test]
    fn escapes_quotes_and_backslashes() {
        let filter = Filter { conditions: vec![cond(Some("n"), Op::Equals, "o'\\x")], ..Filter::default() };
        assert_eq!(filter.sql_suffix(Engine::Postgres, &[]), " WHERE \"n\" = E'o''\\\\x'");
    }

    #[test]
    fn default_sort_priority() {
        let cols = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let sort = |names: &[&str]| default_sort(&cols(names)).map(|s| (s.column, s.descending));
        assert_eq!(sort(&["id", "name", "created_at", "updated_at"]), Some(("updated_at".into(), true)));
        assert_eq!(sort(&["id", "Created_At", "name"]), Some(("Created_At".into(), true)));
        assert_eq!(sort(&["id", "name"]), Some(("name".into(), false)));
        assert_eq!(sort(&["x", "id"]), Some(("id".into(), false)));
        assert_eq!(sort(&["x", "y"]), Some(("x".into(), false)));
        assert_eq!(sort(&[]), None);
    }

    #[test]
    fn mysql_dialect() {
        let filter = Filter {
            conditions: vec![cond(Some("name"), Op::Contains, "a_b"), cond(None, Op::Equals, "x")],
            join: Join::And,
            sort: Some(Sort { column: "id".into(), descending: false }),
        };
        assert_eq!(
            filter.sql_suffix(Engine::MySql, &["c".into()]),
            " WHERE CAST(`name` AS CHAR) LIKE '%a!_b%' ESCAPE '!' AND (CAST(`c` AS CHAR) = 'x') ORDER BY `id` IS NULL, `id` ASC"
        );
    }

    #[test]
    fn null_checks_and_sort() {
        let filter = Filter {
            conditions: vec![cond(Some("deleted_at"), Op::IsNull, "")],
            join: Join::And,
            sort: Some(Sort { column: "id".into(), descending: true }),
        };
        assert_eq!(filter.sql_suffix(Engine::Postgres, &[]), " WHERE \"deleted_at\" IS NULL ORDER BY \"id\" DESC NULLS LAST");
    }
}
