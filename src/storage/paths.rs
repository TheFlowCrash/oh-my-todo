use crate::domain::{SpaceId, Task, TaskId};
use directories::ProjectDirs;
use std::env;
use std::path::{Path, PathBuf};

use super::repository::StorageError;

const DATA_DIR_ENV: &str = "OH_MY_TODO_DATA_DIR";

#[derive(Debug, Clone)]
pub struct DataPaths {
    root: PathBuf,
}

impl DataPaths {
    pub fn resolve_default() -> Result<Self, StorageError> {
        if let Some(path) = env::var_os(DATA_DIR_ENV) {
            return Ok(Self::from_root(PathBuf::from(path)));
        }

        let project_dirs = ProjectDirs::from("", "", "oh-my-todo").ok_or_else(|| {
            StorageError::PathResolution("unable to determine OS data directory".to_owned())
        })?;

        Ok(Self::from_root(project_dirs.data_local_dir().to_path_buf()))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn spaces_dir(&self) -> PathBuf {
        self.root.join("spaces")
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("config.ron")
    }

    pub fn state_file(&self) -> PathBuf {
        self.config_dir().join("state.ron")
    }

    pub fn space_dir(&self, space_id: &SpaceId) -> PathBuf {
        self.spaces_dir().join(space_id.as_str())
    }

    pub fn space_file(&self, space_id: &SpaceId) -> PathBuf {
        self.space_dir(space_id).join("space.ron")
    }

    pub fn space_todo_dir(&self, space_id: &SpaceId) -> PathBuf {
        self.space_dir(space_id).join("todo")
    }

    pub fn space_archive_dir(&self, space_id: &SpaceId) -> PathBuf {
        self.space_dir(space_id).join("archive")
    }

    pub fn task_path(&self, task: &Task) -> PathBuf {
        self.task_path_for_id(&task.space_id, &task.id, task.storage_bucket())
    }

    pub fn task_path_for_id(&self, space_id: &SpaceId, task_id: &TaskId, bucket: &str) -> PathBuf {
        let base = match bucket {
            "archive" => self.space_archive_dir(space_id),
            _ => self.space_todo_dir(space_id),
        };

        base.join(format!("{}.ron", task_id.as_str()))
    }
}
