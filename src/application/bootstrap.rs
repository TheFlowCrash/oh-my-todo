use crate::application::error::AppError;
use crate::application::service::{SpaceService, TaskService};
use crate::storage::{AppRepository, DataPaths, FilesystemRepository, RepositorySnapshot};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct BootstrapOptions {
    pub data_root: Option<PathBuf>,
}

#[derive(Clone)]
pub struct AppContext {
    repository: Arc<dyn AppRepository>,
    pub startup: RepositorySnapshot,
    pub space_service: SpaceService,
    pub task_service: TaskService,
}

impl AppContext {
    pub fn data_root(&self) -> &Path {
        self.repository.paths().root()
    }
}

pub fn bootstrap(options: BootstrapOptions) -> Result<AppContext, AppError> {
    let paths = match options.data_root {
        Some(root) => DataPaths::from_root(root),
        None => DataPaths::resolve_default()?,
    };

    let repository: Arc<dyn AppRepository> = Arc::new(FilesystemRepository::new(paths));
    let startup = repository.initialize()?;
    let space_service = SpaceService::new(repository.clone());
    let task_service = TaskService::new(repository.clone());

    Ok(AppContext {
        repository,
        startup,
        space_service,
        task_service,
    })
}
