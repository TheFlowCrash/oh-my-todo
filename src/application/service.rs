use crate::application::error::AppError;
use crate::domain::{
    AppConfig, AppState, Space, SpaceId, Task, TaskId, resolve_space_ref, resolve_task_ref,
};
use crate::storage::AppRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct SpaceService {
    repository: Arc<dyn AppRepository>,
}

#[derive(Clone)]
pub struct TaskService {
    repository: Arc<dyn AppRepository>,
}

impl SpaceService {
    pub fn new(repository: Arc<dyn AppRepository>) -> Self {
        Self { repository }
    }

    pub fn load_app_config(&self) -> Result<AppConfig, AppError> {
        self.repository.load_config().map_err(AppError::from)
    }

    pub fn load_app_state(&self) -> Result<AppState, AppError> {
        self.repository.load_state().map_err(AppError::from)
    }

    pub fn list_spaces(&self) -> Result<Vec<Space>, AppError> {
        self.repository.list_spaces().map_err(AppError::from)
    }

    pub fn load_space(&self, space_id: &SpaceId) -> Result<Space, AppError> {
        self.repository.load_space(space_id).map_err(AppError::from)
    }

    pub fn save_space(&self, space: &Space) -> Result<(), AppError> {
        self.repository.save_space(space).map_err(AppError::from)
    }

    pub fn resolve_space_ref(&self, reference: &str) -> Result<SpaceId, AppError> {
        let spaces = self.list_spaces()?;
        resolve_space_ref(reference, spaces.iter()).map_err(AppError::from)
    }

    pub fn set_current_space(
        &self,
        current_space_id: Option<SpaceId>,
    ) -> Result<AppState, AppError> {
        let mut state = self.load_app_state()?;
        state.current_space_id = current_space_id;
        self.repository.save_state(&state)?;
        Ok(state)
    }
}

impl TaskService {
    pub fn new(repository: Arc<dyn AppRepository>) -> Self {
        Self { repository }
    }

    pub fn list_tasks(&self, space_id: &SpaceId) -> Result<Vec<Task>, AppError> {
        self.repository
            .list_tasks_in_space(space_id)
            .map_err(AppError::from)
    }

    pub fn list_all_tasks(&self) -> Result<Vec<Task>, AppError> {
        self.repository.list_all_tasks().map_err(AppError::from)
    }

    pub fn resolve_task_ref(&self, reference: &str) -> Result<TaskId, AppError> {
        let tasks = self.list_all_tasks()?;
        resolve_task_ref(reference, tasks.iter().map(|task| &task.id)).map_err(AppError::from)
    }

    pub fn save_task(&self, task: &Task) -> Result<(), AppError> {
        self.repository.save_task(task).map_err(AppError::from)
    }

    pub fn load_task(&self, task_id: &TaskId) -> Result<Task, AppError> {
        self.repository.load_task(task_id).map_err(AppError::from)
    }
}
