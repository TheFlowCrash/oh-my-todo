use crate::application::error::AppError;
use crate::domain::AppState;
use crate::storage::AppRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppStateService {
    repository: Arc<dyn AppRepository>,
}

impl AppStateService {
    pub fn new(repository: Arc<dyn AppRepository>) -> Self {
        Self { repository }
    }

    pub fn load(&self) -> Result<AppState, AppError> {
        self.repository.load_state().map_err(AppError::from)
    }

    pub fn save(&self, state: &AppState) -> Result<(), AppError> {
        self.repository.save_state(state).map_err(AppError::from)
    }

    pub fn update<F>(&self, mutator: F) -> Result<AppState, AppError>
    where
        F: FnOnce(&mut AppState),
    {
        let mut state = self.load()?;
        mutator(&mut state);
        self.save(&state)?;
        Ok(state)
    }
}
