use super::atomic::write_string_atomically;
use super::repository::StorageError;
use ron::de::from_str;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

pub fn to_ron_string<T: Serialize>(value: &T) -> Result<String, StorageError> {
    to_string_pretty(value, PrettyConfig::new())
        .map_err(|source| StorageError::RonSerialize { source })
}

pub fn from_ron_str<T: DeserializeOwned>(value: &str) -> Result<T, StorageError> {
    from_str(value).map_err(|source| StorageError::RonDeserialize { source })
}

pub fn read_ron_file<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let contents = fs::read_to_string(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    from_ron_str(&contents)
}

pub fn write_ron_file<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let contents = to_ron_string(value)?;
    write_string_atomically(path, &contents)
}
