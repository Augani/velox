use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("key not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_io() {
        let err = StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn error_display_migration() {
        let err = StorageError::Migration("version conflict".into());
        assert_eq!(err.to_string(), "migration error: version conflict");
    }

    #[test]
    fn error_display_not_found() {
        let err = StorageError::NotFound("theme.color".into());
        assert_eq!(err.to_string(), "key not found: theme.color");
    }
}
