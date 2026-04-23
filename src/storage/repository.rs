use crate::domain::{AppConfig, AppState, Space, SpaceId, Task, TaskId};
use crate::storage::paths::DataPaths;
use crate::storage::serializer::{read_ron_file, write_ron_file};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error at `{path}`: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to serialize RON: {source}")]
    RonSerialize { source: ron::error::Error },
    #[error("failed to deserialize RON: {source}")]
    RonDeserialize { source: ron::error::SpannedError },
    #[error("failed to resolve data path: {0}")]
    PathResolution(String),
    #[error("task `{0}` was not found")]
    TaskNotFound(String),
    #[error("space `{0}` was not found")]
    SpaceNotFound(String),
}

#[derive(Debug, Clone)]
pub struct RepositorySnapshot {
    pub config: AppConfig,
    pub state: AppState,
    pub spaces: Vec<Space>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBucket {
    Todo,
    Archive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTaskRecord {
    pub task: Task,
    pub bucket: TaskBucket,
}

pub trait AppRepository: Send + Sync {
    fn paths(&self) -> &DataPaths;
    fn initialize(&self) -> Result<RepositorySnapshot, StorageError>;
    fn load_config(&self) -> Result<AppConfig, StorageError>;
    fn save_config(&self, config: &AppConfig) -> Result<(), StorageError>;
    fn load_state(&self) -> Result<AppState, StorageError>;
    fn save_state(&self, state: &AppState) -> Result<(), StorageError>;
    fn list_spaces(&self) -> Result<Vec<Space>, StorageError>;
    fn load_space(&self, space_id: &SpaceId) -> Result<Space, StorageError>;
    fn save_space(&self, space: &Space) -> Result<(), StorageError>;
    fn list_task_records_in_space(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<StoredTaskRecord>, StorageError>;
    fn list_all_task_records(&self) -> Result<Vec<StoredTaskRecord>, StorageError>;
    fn list_tasks_in_space(&self, space_id: &SpaceId) -> Result<Vec<Task>, StorageError>;
    fn list_all_tasks(&self) -> Result<Vec<Task>, StorageError>;
    fn load_task(&self, task_id: &TaskId) -> Result<Task, StorageError>;
    fn save_task(&self, task: &Task) -> Result<(), StorageError>;
    fn delete_task(&self, task_id: &TaskId) -> Result<(), StorageError>;
    fn delete_space(&self, space_id: &SpaceId) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct FilesystemRepository {
    paths: DataPaths,
}

impl FilesystemRepository {
    pub fn new(paths: DataPaths) -> Self {
        Self { paths }
    }

    fn ensure_layout(&self) -> Result<(), StorageError> {
        for dir in [
            self.paths.root(),
            &self.paths.config_dir(),
            &self.paths.spaces_dir(),
        ] {
            fs::create_dir_all(dir).map_err(|source| StorageError::Io {
                path: dir.to_path_buf(),
                source,
            })?;
        }

        if !self.paths.config_file().exists() {
            self.save_config(&AppConfig::default())?;
        }

        if !self.paths.state_file().exists() {
            self.save_state(&AppState::default())?;
        }

        Ok(())
    }

    fn ensure_space_layout(&self, space_id: &SpaceId) -> Result<(), StorageError> {
        for dir in [
            self.paths.space_dir(space_id),
            self.paths.space_todo_dir(space_id),
            self.paths.space_archive_dir(space_id),
        ] {
            fs::create_dir_all(&dir).map_err(|source| StorageError::Io { path: dir, source })?;
        }

        Ok(())
    }
}

impl AppRepository for FilesystemRepository {
    fn paths(&self) -> &DataPaths {
        &self.paths
    }

    fn initialize(&self) -> Result<RepositorySnapshot, StorageError> {
        self.ensure_layout()?;
        Ok(RepositorySnapshot {
            config: self.load_config()?,
            state: self.load_state()?,
            spaces: self.list_spaces()?,
        })
    }

    fn load_config(&self) -> Result<AppConfig, StorageError> {
        read_ron_file(&self.paths.config_file())
    }

    fn save_config(&self, config: &AppConfig) -> Result<(), StorageError> {
        write_ron_file(&self.paths.config_file(), config)
    }

    fn load_state(&self) -> Result<AppState, StorageError> {
        read_ron_file(&self.paths.state_file())
    }

    fn save_state(&self, state: &AppState) -> Result<(), StorageError> {
        write_ron_file(&self.paths.state_file(), state)
    }

    fn list_spaces(&self) -> Result<Vec<Space>, StorageError> {
        if !self.paths.spaces_dir().exists() {
            return Ok(Vec::new());
        }

        let mut spaces = Vec::new();
        for entry in read_dir_sorted(&self.paths.spaces_dir())? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let space_file = path.join("space.ron");
            if space_file.exists() {
                spaces.push(read_ron_file(&space_file)?);
            }
        }

        Ok(spaces)
    }

    fn load_space(&self, space_id: &SpaceId) -> Result<Space, StorageError> {
        let path = self.paths.space_file(space_id);
        if !path.exists() {
            return Err(StorageError::SpaceNotFound(space_id.as_str().to_owned()));
        }

        read_ron_file(&path)
    }

    fn save_space(&self, space: &Space) -> Result<(), StorageError> {
        self.ensure_layout()?;
        self.ensure_space_layout(&space.id)?;
        write_ron_file(&self.paths.space_file(&space.id), space)
    }

    fn list_task_records_in_space(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<StoredTaskRecord>, StorageError> {
        self.ensure_layout()?;
        let mut tasks =
            load_task_records_from_dir(&self.paths.space_todo_dir(space_id), TaskBucket::Todo)?;
        tasks.extend(load_task_records_from_dir(
            &self.paths.space_archive_dir(space_id),
            TaskBucket::Archive,
        )?);
        Ok(tasks)
    }

    fn list_all_task_records(&self) -> Result<Vec<StoredTaskRecord>, StorageError> {
        let mut tasks = Vec::new();
        for space in self.list_spaces()? {
            tasks.extend(self.list_task_records_in_space(&space.id)?);
        }
        Ok(tasks)
    }

    fn list_tasks_in_space(&self, space_id: &SpaceId) -> Result<Vec<Task>, StorageError> {
        self.list_task_records_in_space(space_id)
            .map(|records| records.into_iter().map(|record| record.task).collect())
    }

    fn list_all_tasks(&self) -> Result<Vec<Task>, StorageError> {
        self.list_all_task_records()
            .map(|records| records.into_iter().map(|record| record.task).collect())
    }

    fn load_task(&self, task_id: &TaskId) -> Result<Task, StorageError> {
        self.list_all_tasks()?
            .into_iter()
            .find(|task| &task.id == task_id)
            .ok_or_else(|| StorageError::TaskNotFound(task_id.as_str().to_owned()))
    }

    fn save_task(&self, task: &Task) -> Result<(), StorageError> {
        self.ensure_layout()?;
        self.ensure_space_layout(&task.space_id)?;

        let target_path = self.paths.task_path(task);

        write_ron_file(&target_path, task)?;

        for space in self.list_spaces()? {
            for bucket in ["todo", "archive"] {
                let candidate_path = self.paths.task_path_for_id(&space.id, &task.id, bucket);
                if candidate_path != target_path && candidate_path.exists() {
                    fs::remove_file(&candidate_path).map_err(|source| StorageError::Io {
                        path: candidate_path,
                        source,
                    })?;
                }
            }
        }

        Ok(())
    }

    fn delete_task(&self, task_id: &TaskId) -> Result<(), StorageError> {
        for space in self.list_spaces()? {
            for bucket in [TaskBucket::Todo, TaskBucket::Archive] {
                let candidate_path =
                    self.paths
                        .task_path_for_id(&space.id, task_id, bucket.as_str());
                if candidate_path.exists() {
                    fs::remove_file(&candidate_path).map_err(|source| StorageError::Io {
                        path: candidate_path,
                        source,
                    })?;
                }
            }
        }

        Ok(())
    }

    fn delete_space(&self, space_id: &SpaceId) -> Result<(), StorageError> {
        let path = self.paths.space_dir(space_id);
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|source| StorageError::Io { path, source })?;
        }

        Ok(())
    }
}

fn load_task_records_from_dir(
    dir: &Path,
    bucket: TaskBucket,
) -> Result<Vec<StoredTaskRecord>, StorageError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut tasks = Vec::new();
    for entry in read_dir_sorted(dir)? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("ron") {
            continue;
        }

        tasks.push(StoredTaskRecord {
            task: read_ron_file(&path)?,
            bucket,
        });
    }

    Ok(tasks)
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<fs::DirEntry>, StorageError> {
    let mut entries = fs::read_dir(dir)
        .map_err(|source| StorageError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| StorageError::Io {
            path: dir.to_path_buf(),
            source,
        })?;

    entries.sort_by_key(|entry| entry.path());
    Ok(entries)
}

impl TaskBucket {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Archive => "archive",
        }
    }
}
