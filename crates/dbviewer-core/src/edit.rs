use crate::engine::Engine;
use crate::session::TableRef;

/// A column and its new (or identifying) value; `None` is SQL NULL.
pub type Assignment = (String, Option<String>);

fn value_sql(engine: Engine, value: &Option<String>) -> String {
    value.as_deref().map_or_else(|| "NULL".to_string(), |v| engine.literal(v))
}

/// Untyped literals, so the server coerces each value to its column's type.
pub fn update_sql(table: &TableRef, changes: &[Assignment], key: &[Assignment]) -> String {
    let engine = table.engine;
    let set: Vec<String> = changes.iter().map(|(c, v)| format!("{} = {}", engine.quote_ident(c), value_sql(engine, v))).collect();
    let conditions: Vec<String> = key
        .iter()
        .map(|(c, v)| match v {
            Some(v) => format!("{} = {}", engine.quote_ident(c), engine.literal(v)),
            None => format!("{} IS NULL", engine.quote_ident(c)),
        })
        .collect();
    format!("UPDATE {} SET {} WHERE {}", table.qualified(), set.join(", "), conditions.join(" AND "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::TableKind;

    fn table() -> TableRef {
        TableRef { engine: Engine::Postgres, database: "d".into(), schema: "public".into(), name: "branches".into(), kind: TableKind::Table }
    }

    #[test]
    fn builds_update_with_nulls_and_escaping() {
        let sql = update_sql(
            &table(),
            &[("name".into(), Some("O'Brien".into())), ("note".into(), None)],
            &[("id".into(), Some("7".into())), ("org".into(), Some("a".into()))],
        );
        assert_eq!(
            sql,
            "UPDATE \"public\".\"branches\" SET \"name\" = E'O''Brien', \"note\" = NULL WHERE \"id\" = E'7' AND \"org\" = E'a'"
        );
    }
}
