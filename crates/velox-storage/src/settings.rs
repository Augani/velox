use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::StorageError;

pub struct SettingsStore {
    values: Arc<RwLock<HashMap<String, toml::Value>>>,
    path: PathBuf,
}

impl SettingsStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let path = path.into();
        let values = if path.exists() {
            Self::load_from_file(&path)?
        } else {
            HashMap::new()
        };
        Ok(Self {
            values: Arc::new(RwLock::new(values)),
            path,
        })
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let map = self.values.read().unwrap_or_else(|e| e.into_inner());
        let value = map.get(key)?;
        value.clone().try_into().ok()
    }

    pub fn set<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let toml_value = toml::Value::try_from(value)?;
        {
            let mut map = self.values.write().unwrap_or_else(|e| e.into_inner());
            map.insert(key.to_owned(), toml_value);
        }
        self.flush()
    }

    pub fn remove(&self, key: &str) -> bool {
        let mut map = self.values.write().unwrap_or_else(|e| e.into_inner());
        map.remove(key).is_some()
    }

    pub fn keys(&self) -> Vec<String> {
        let map = self.values.read().unwrap_or_else(|e| e.into_inner());
        map.keys().cloned().collect()
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        let map = self.values.read().unwrap_or_else(|e| e.into_inner());
        let table = toml::Value::Table(map.clone().into_iter().collect());
        let serialized = toml::to_string_pretty(&table)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serialized)?;
        Ok(())
    }

    fn load_from_file(path: &Path) -> Result<HashMap<String, toml::Value>, StorageError> {
        let content = std::fs::read_to_string(path)?;
        let table: toml::Value = content.parse::<toml::Value>()?;
        match table {
            toml::Value::Table(map) => Ok(map.into_iter().collect()),
            _ => Ok(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_settings_path() -> PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("velox_test_settings_{id}.toml"))
    }

    #[test]
    fn get_set_roundtrip() {
        let path = temp_settings_path();
        let store = SettingsStore::open(&path).unwrap();
        store.set("name", &"velox".to_string()).unwrap();
        let val: Option<String> = store.get("name");
        assert_eq!(val.as_deref(), Some("velox"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_overwrites_previous() {
        let path = temp_settings_path();
        let store = SettingsStore::open(&path).unwrap();
        store.set("count", &10i64).unwrap();
        store.set("count", &20i64).unwrap();
        let val: Option<i64> = store.get("count");
        assert_eq!(val, Some(20));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_key() {
        let path = temp_settings_path();
        let store = SettingsStore::open(&path).unwrap();
        store.set("temp", &true).unwrap();
        assert!(store.remove("temp"));
        assert!(!store.remove("temp"));
        let val: Option<bool> = store.get("temp");
        assert!(val.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn flush_and_reload() {
        let path = temp_settings_path();
        {
            let store = SettingsStore::open(&path).unwrap();
            store.set("key1", &"value1".to_string()).unwrap();
            store.set("key2", &42i64).unwrap();
        }
        let store = SettingsStore::open(&path).unwrap();
        let v1: Option<String> = store.get("key1");
        let v2: Option<i64> = store.get("key2");
        assert_eq!(v1.as_deref(), Some("value1"));
        assert_eq!(v2, Some(42));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn keys_listing() {
        let path = temp_settings_path();
        let store = SettingsStore::open(&path).unwrap();
        store.set("alpha", &1i64).unwrap();
        store.set("beta", &2i64).unwrap();
        let mut keys = store.keys();
        keys.sort();
        assert_eq!(keys, vec!["alpha", "beta"]);
        let _ = std::fs::remove_file(&path);
    }
}
