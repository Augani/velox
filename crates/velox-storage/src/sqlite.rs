use std::path::Path;

use crate::error::StorageError;

pub struct SqlitePool {
    conn: rusqlite::Connection,
}

impl SqlitePool {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self { conn })
    }

    pub fn execute(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::types::ToSql],
    ) -> Result<usize, StorageError> {
        Ok(self.conn.execute(sql, params)?)
    }

    pub fn query_row<T, F>(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::types::ToSql],
        f: F,
    ) -> Result<T, StorageError>
    where
        F: FnOnce(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
    {
        Ok(self.conn.query_row(sql, params, f)?)
    }

    pub fn connection(&self) -> &rusqlite::Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_succeeds() {
        let pool = SqlitePool::open_in_memory();
        assert!(pool.is_ok());
    }

    #[test]
    fn execute_and_query_row() {
        let pool = SqlitePool::open_in_memory().unwrap();
        pool.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
        pool.execute(
            "INSERT INTO test (id, name) VALUES (?1, ?2)",
            &[&1i64, &"hello"],
        )
        .unwrap();
        let name: String = pool
            .query_row("SELECT name FROM test WHERE id = ?1", &[&1i64], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "hello");
    }
}
