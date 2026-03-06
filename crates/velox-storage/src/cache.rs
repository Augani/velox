use std::path::Path;
use std::time::Duration;

use crate::error::StorageError;
use crate::migration::MigrationRunner;
use crate::sqlite::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePressure {
    Low,
    Medium,
    High,
    Critical,
}

pub struct CacheStore {
    pool: SqlitePool,
    max_bytes: u64,
}

impl CacheStore {
    pub fn open(path: impl AsRef<Path>, max_bytes: u64) -> Result<Self, StorageError> {
        let pool = SqlitePool::open(path)?;
        Self::initialize(pool, max_bytes)
    }

    pub fn open_in_memory(max_bytes: u64) -> Result<Self, StorageError> {
        let pool = SqlitePool::open_in_memory()?;
        Self::initialize(pool, max_bytes)
    }

    fn initialize(pool: SqlitePool, max_bytes: u64) -> Result<Self, StorageError> {
        MigrationRunner::new()
            .add(
                1,
                "CREATE TABLE IF NOT EXISTS cache_entries (
                    key TEXT PRIMARY KEY,
                    data BLOB NOT NULL,
                    size INTEGER NOT NULL,
                    expires_at INTEGER,
                    last_accessed INTEGER NOT NULL
                )",
            )
            .run(&pool)?;

        let store = Self { pool, max_bytes };
        store.cleanup_expired()?;
        Ok(store)
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let now = Self::unix_now();
        let result = self.pool.connection().query_row(
            "SELECT data, expires_at FROM cache_entries WHERE key = ?1",
            [key],
            |row| {
                let data: Vec<u8> = row.get(0)?;
                let expires_at: Option<i64> = row.get(1)?;
                Ok((data, expires_at))
            },
        );

        match result {
            Ok((data, expires_at)) => {
                if let Some(exp) = expires_at {
                    if exp <= now {
                        self.pool
                            .execute("DELETE FROM cache_entries WHERE key = ?1", &[&key])?;
                        return Ok(None);
                    }
                }
                self.pool.execute(
                    "UPDATE cache_entries SET last_accessed = ?1 WHERE key = ?2",
                    &[&now, &key],
                )?;
                Ok(Some(data))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Sqlite(e)),
        }
    }

    pub fn put(&self, key: &str, data: &[u8], ttl: Option<Duration>) -> Result<(), StorageError> {
        let now = Self::unix_now();
        let expires_at = ttl.map(|d| now + d.as_secs() as i64);
        let size = data.len() as i64;

        let existing_size = self.entry_size(key)?;
        let current = self.current_bytes()? - existing_size;
        let new_total = current + size as u64;
        if new_total > self.max_bytes {
            let to_free = new_total - self.max_bytes;
            self.evict_lru(to_free)?;
        }

        self.pool.connection().execute(
            "INSERT OR REPLACE INTO cache_entries (key, data, size, expires_at, last_accessed) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![key, data, size, expires_at, now],
        )?;

        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<bool, StorageError> {
        let rows = self
            .pool
            .execute("DELETE FROM cache_entries WHERE key = ?1", &[&key])?;
        Ok(rows > 0)
    }

    pub fn pressure(&self) -> Result<CachePressure, StorageError> {
        let current = self.current_bytes()?;
        let ratio = current as f64 / self.max_bytes.max(1) as f64;
        Ok(if ratio < 0.5 {
            CachePressure::Low
        } else if ratio < 0.75 {
            CachePressure::Medium
        } else if ratio < 0.9 {
            CachePressure::High
        } else {
            CachePressure::Critical
        })
    }

    pub fn cleanup_expired(&self) -> Result<u64, StorageError> {
        let now = Self::unix_now();
        let rows = self.pool.execute(
            "DELETE FROM cache_entries WHERE expires_at IS NOT NULL AND expires_at <= ?1",
            &[&now],
        )?;
        Ok(rows as u64)
    }

    fn entry_size(&self, key: &str) -> Result<u64, StorageError> {
        let result = self.pool.connection().query_row(
            "SELECT size FROM cache_entries WHERE key = ?1",
            [key],
            |row| row.get::<_, i64>(0),
        );
        match result {
            Ok(size) => Ok(size.max(0) as u64),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(StorageError::Sqlite(e)),
        }
    }

    fn current_bytes(&self) -> Result<u64, StorageError> {
        let total: i64 = self.pool.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM cache_entries",
            &[],
            |row| row.get(0),
        )?;
        Ok(total.max(0) as u64)
    }

    fn evict_lru(&self, bytes_to_free: u64) -> Result<(), StorageError> {
        let conn = self.pool.connection();
        let mut stmt =
            conn.prepare("SELECT key, size FROM cache_entries ORDER BY last_accessed ASC")?;
        let entries: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut freed: u64 = 0;
        for (key, size) in entries {
            if freed >= bytes_to_free {
                break;
            }
            self.pool
                .execute("DELETE FROM cache_entries WHERE key = ?1", &[&key])?;
            freed += size as u64;
        }
        Ok(())
    }

    fn unix_now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_roundtrip() {
        let store = CacheStore::open_in_memory(1024 * 1024).unwrap();
        store.put("key1", b"hello world", None).unwrap();
        let data = store.get("key1").unwrap();
        assert_eq!(data.as_deref(), Some(b"hello world".as_slice()));
    }

    #[test]
    fn get_returns_none_for_missing() {
        let store = CacheStore::open_in_memory(1024).unwrap();
        let data = store.get("nonexistent").unwrap();
        assert!(data.is_none());
    }

    #[test]
    fn ttl_expiry() {
        let store = CacheStore::open_in_memory(1024 * 1024).unwrap();
        store
            .put("ephemeral", b"data", Some(Duration::from_secs(0)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        let data = store.get("ephemeral").unwrap();
        assert!(data.is_none());
    }

    #[test]
    fn lru_eviction() {
        let store = CacheStore::open_in_memory(100).unwrap();
        store.put("a", &[0u8; 60], None).unwrap();
        store.put("b", &[1u8; 60], None).unwrap();

        let a = store.get("a").unwrap();
        assert!(a.is_none());
        let b = store.get("b").unwrap();
        assert!(b.is_some());
    }

    #[test]
    fn pressure_levels() {
        let store = CacheStore::open_in_memory(100).unwrap();
        assert_eq!(store.pressure().unwrap(), CachePressure::Low);

        store.put("half", &[0u8; 50], None).unwrap();
        assert_eq!(store.pressure().unwrap(), CachePressure::Medium);

        store.put("more", &[0u8; 30], None).unwrap();
        assert_eq!(store.pressure().unwrap(), CachePressure::High);

        store.put("full", &[0u8; 15], None).unwrap();
        assert_eq!(store.pressure().unwrap(), CachePressure::Critical);
    }

    #[test]
    fn remove_entry() {
        let store = CacheStore::open_in_memory(1024).unwrap();
        store.put("removable", b"data", None).unwrap();
        assert!(store.remove("removable").unwrap());
        assert!(!store.remove("removable").unwrap());
        assert!(store.get("removable").unwrap().is_none());
    }

    #[test]
    fn cleanup_expired_entries() {
        let store = CacheStore::open_in_memory(1024 * 1024).unwrap();
        store
            .put("exp1", b"a", Some(Duration::from_secs(0)))
            .unwrap();
        store
            .put("exp2", b"b", Some(Duration::from_secs(0)))
            .unwrap();
        store.put("keep", b"c", None).unwrap();

        std::thread::sleep(Duration::from_millis(1100));
        let cleaned = store.cleanup_expired().unwrap();
        assert_eq!(cleaned, 2);
        assert!(store.get("keep").unwrap().is_some());
    }
}
