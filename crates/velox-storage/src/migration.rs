use crate::error::StorageError;
use crate::sqlite::SqlitePool;

pub struct Migration {
    pub version: u32,
    pub sql: String,
}

pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    pub fn add(mut self, version: u32, sql: impl Into<String>) -> Self {
        self.migrations.push(Migration {
            version,
            sql: sql.into(),
        });
        self
    }

    pub fn run(&self, pool: &SqlitePool) -> Result<(), StorageError> {
        pool.execute(
            "CREATE TABLE IF NOT EXISTS _migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
            &[],
        )?;

        let mut sorted: Vec<&Migration> = self.migrations.iter().collect();
        sorted.sort_by_key(|m| m.version);

        for migration in sorted {
            let already_applied: bool = pool
                .query_row(
                    "SELECT COUNT(*) > 0 FROM _migrations WHERE version = ?1",
                    &[&migration.version],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if already_applied {
                continue;
            }

            pool.connection()
                .execute_batch(&migration.sql)
                .map_err(|e| {
                    StorageError::Migration(format!(
                        "failed to apply migration v{}: {e}",
                        migration.version
                    ))
                })?;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            pool.execute(
                "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
                &[&migration.version, &now],
            )?;
        }

        Ok(())
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_migrations_in_order() {
        let pool = SqlitePool::open_in_memory().unwrap();
        let runner = MigrationRunner::new()
            .add(1, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .add(2, "ALTER TABLE users ADD COLUMN email TEXT");

        runner.run(&pool).unwrap();

        pool.execute(
            "INSERT INTO users (id, name, email) VALUES (?1, ?2, ?3)",
            &[
                &1i64 as &dyn rusqlite::types::ToSql,
                &"alice",
                &"alice@test.com",
            ],
        )
        .unwrap();

        let email: String = pool
            .query_row("SELECT email FROM users WHERE id = ?1", &[&1i64], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(email, "alice@test.com");
    }

    #[test]
    fn skip_already_applied_migrations() {
        let pool = SqlitePool::open_in_memory().unwrap();
        let runner = MigrationRunner::new().add(1, "CREATE TABLE items (id INTEGER PRIMARY KEY)");

        runner.run(&pool).unwrap();
        runner.run(&pool).unwrap();

        let count: i64 = pool
            .query_row("SELECT COUNT(*) FROM _migrations", &[], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
