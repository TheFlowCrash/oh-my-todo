use super::repository::StorageError;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use ulid::Ulid;

#[cfg(unix)]
use std::fs::File;

pub fn write_string_atomically(path: &Path, contents: &str) -> Result<(), StorageError> {
    let parent = path.parent().ok_or_else(|| {
        StorageError::PathResolution(format!("missing parent directory for {}", path.display()))
    })?;

    fs::create_dir_all(parent).map_err(|source| StorageError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            StorageError::PathResolution(format!("invalid file name for {}", path.display()))
        })?;

    let temp_path = parent.join(format!(".{file_name}.tmp-{}", Ulid::new()));
    let mut temp_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .map_err(|source| StorageError::Io {
            path: temp_path.clone(),
            source,
        })?;

    temp_file
        .write_all(contents.as_bytes())
        .and_then(|_| temp_file.flush())
        .and_then(|_| temp_file.sync_all())
        .map_err(|source| StorageError::Io {
            path: temp_path.clone(),
            source,
        })?;

    drop(temp_file);

    fs::rename(&temp_path, path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    sync_parent_dir(parent)?;
    Ok(())
}

#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> Result<(), StorageError> {
    File::open(path)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) -> Result<(), StorageError> {
    Ok(())
}
