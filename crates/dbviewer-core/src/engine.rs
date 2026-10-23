use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[default]
    Postgres,
    MySql,
    MariaDb,
}

impl Engine {
    pub const ALL: [Engine; 3] = [Engine::Postgres, Engine::MySql, Engine::MariaDb];

    pub fn label(self) -> &'static str {
        match self {
            Engine::Postgres => "PostgreSQL",
            Engine::MySql => "MySQL",
            Engine::MariaDb => "MariaDB",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Engine::Postgres => 5432,
            Engine::MySql | Engine::MariaDb => 3306,
        }
    }

    pub fn default_user(self) -> &'static str {
        match self {
            Engine::Postgres => "postgres",
            Engine::MySql | Engine::MariaDb => "root",
        }
    }

    pub fn default_database(self) -> &'static str {
        match self {
            Engine::Postgres => "postgres",
            Engine::MySql | Engine::MariaDb => "",
        }
    }

    pub fn keywords(self) -> &'static [&'static str] {
        const COMMON: [&str; 62] = [
            "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "NULL", "IS", "IN", "LIKE", "BETWEEN", "ORDER BY", "GROUP BY", "HAVING", "LIMIT",
            "OFFSET", "JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN", "CROSS JOIN", "ON", "AS", "DISTINCT", "UNION", "ALL", "INSERT INTO",
            "VALUES", "UPDATE", "SET", "DELETE FROM", "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "CREATE INDEX", "COUNT", "SUM", "AVG",
            "MIN", "MAX", "CASE", "WHEN", "THEN", "ELSE", "END", "EXISTS", "ASC", "DESC", "WITH", "COALESCE", "CAST", "TRUE", "FALSE",
            "PRIMARY KEY", "FOREIGN KEY", "REFERENCES", "DEFAULT", "UNIQUE", "CONSTRAINT", "TRUNCATE", "EXPLAIN", "USING",
        ];
        const POSTGRES: [&str; 14] = [
            "ILIKE", "RETURNING", "ANALYZE", "NOW()", "CURRENT_TIMESTAMP", "GENERATE_SERIES", "JSONB", "ARRAY_AGG", "STRING_AGG", "LATERAL",
            "NULLS LAST", "ON CONFLICT", "DO NOTHING", "VACUUM",
        ];
        const MYSQL: [&str; 16] = [
            "SHOW DATABASES", "SHOW TABLES", "SHOW COLUMNS", "SHOW CREATE TABLE", "DESCRIBE", "USE", "IFNULL", "NOW()", "CURDATE()", "CONCAT",
            "GROUP_CONCAT", "AUTO_INCREMENT", "ENGINE", "ON DUPLICATE KEY UPDATE", "STRAIGHT_JOIN", "REPLACE INTO",
        ];
        static POSTGRES_ALL: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
        static MYSQL_ALL: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
        match self {
            Engine::Postgres => POSTGRES_ALL.get_or_init(|| COMMON.iter().chain(POSTGRES.iter()).copied().collect()),
            Engine::MySql | Engine::MariaDb => MYSQL_ALL.get_or_init(|| COMMON.iter().chain(MYSQL.iter()).copied().collect()),
        }
    }

    pub fn quote_ident(self, ident: &str) -> String {
        match self {
            Engine::Postgres => format!("\"{}\"", ident.replace('"', "\"\"")),
            Engine::MySql | Engine::MariaDb => format!("`{}`", ident.replace('`', "``")),
        }
    }

    /// Postgres uses the `E''` form so backslashes mean the same whatever
    /// `standard_conforming_strings` is; MySQL treats backslash as an escape by default.
    pub fn literal(self, value: &str) -> String {
        let escaped: String = value
            .chars()
            .filter(|&c| c != '\0')
            .flat_map(|c| match c {
                '\'' => vec!['\'', '\''],
                '\\' => vec!['\\', '\\'],
                c => vec![c],
            })
            .collect();
        match self {
            Engine::Postgres => format!("E'{escaped}'"),
            Engine::MySql | Engine::MariaDb => format!("'{escaped}'"),
        }
    }

    pub(crate) fn as_text(self, column: &str) -> String {
        match self {
            Engine::Postgres => format!("{column}::text"),
            Engine::MySql | Engine::MariaDb => format!("CAST({column} AS CHAR)"),
        }
    }

    pub(crate) fn like(self) -> &'static str {
        match self {
            Engine::Postgres => "ILIKE",
            Engine::MySql | Engine::MariaDb => "LIKE",
        }
    }

    pub(crate) fn not_like(self) -> &'static str {
        match self {
            Engine::Postgres => "NOT ILIKE",
            Engine::MySql | Engine::MariaDb => "NOT LIKE",
        }
    }

    pub(crate) fn order_by(self, column: &str, descending: bool) -> String {
        let direction = if descending { "DESC" } else { "ASC" };
        match self {
            Engine::Postgres => format!("{column} {direction} NULLS LAST"),
            Engine::MySql | Engine::MariaDb => format!("{column} IS NULL, {column} {direction}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_identifiers() {
        assert_eq!(Engine::Postgres.quote_ident("we\"ird"), "\"we\"\"ird\"");
        assert_eq!(Engine::MySql.quote_ident("we`ird"), "`we``ird`");
    }

    #[test]
    fn escapes_literals_per_engine() {
        assert_eq!(Engine::Postgres.literal("o'\\x"), "E'o''\\\\x'");
        assert_eq!(Engine::MySql.literal("o'\\x"), "'o''\\\\x'");
    }
}
