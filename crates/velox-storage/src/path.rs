use std::path::{Path, PathBuf};

use crate::error::StorageError;

pub struct StoragePaths {
    config_dir: PathBuf,
    data_dir: PathBuf,
    cache_dir: PathBuf,
}

impl StoragePaths {
    pub fn new(qualifier: &str, org: &str, app: &str) -> Option<Self> {
        let dirs = directories::ProjectDirs::from(qualifier, org, app)?;
        Some(Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
        })
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn ensure_dirs(&self) -> Result<(), StorageError> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_paths_creation() {
        let paths = StoragePaths::new("com", "velox", "test-app");
        if let Some(paths) = paths {
            assert!(!paths.config_dir().as_os_str().is_empty());
            assert!(!paths.data_dir().as_os_str().is_empty());
            assert!(!paths.cache_dir().as_os_str().is_empty());
        }
    }

    #[test]
    fn ensure_dirs_creates_directories() {
        let tmp = std::env::temp_dir().join("velox_test_paths");
        let paths = StoragePaths {
            config_dir: tmp.join("config"),
            data_dir: tmp.join("data"),
            cache_dir: tmp.join("cache"),
        };
        let result = paths.ensure_dirs();
        assert!(result.is_ok());
        assert!(paths.config_dir().exists());
        assert!(paths.data_dir().exists());
        assert!(paths.cache_dir().exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
