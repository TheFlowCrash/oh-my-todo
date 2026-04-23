use crate::application::commands::{
    CreateSpaceCommand, RenameSpaceCommand, SetCurrentSpaceCommand,
};
use crate::application::error::AppError;
use crate::application::queries::{ListSpacesQuery, ShowSpaceQuery, SpaceDetails, SpaceSummary};
use crate::application::task_query::derive_space_counts;
use crate::domain::{
    AppConfig, AppState, Space, SpaceState, ensure_non_empty_space_name, resolve_space_ref, slugify,
};
use crate::storage::AppRepository;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct SpaceService {
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

    pub fn load_space(&self, space_id: &crate::domain::SpaceId) -> Result<Space, AppError> {
        self.repository.load_space(space_id).map_err(AppError::from)
    }

    pub fn save_space(&self, space: &Space) -> Result<(), AppError> {
        self.repository.save_space(space).map_err(AppError::from)
    }

    pub fn create_space(&self, command: CreateSpaceCommand) -> Result<Space, AppError> {
        ensure_non_empty_space_name(&command.name)?;

        let spaces = self.repository.list_spaces()?;
        let next_sort_order = spaces
            .iter()
            .map(|space| space.sort_order)
            .max()
            .unwrap_or(-1)
            + 1;
        let mut space = Space::new(command.name, next_sort_order);
        space.slug = next_available_slug(&slugify(&space.name), &spaces);
        self.repository.save_space(&space)?;
        Ok(space)
    }

    pub fn list_spaces(&self, query: ListSpacesQuery) -> Result<Vec<SpaceSummary>, AppError> {
        let mut spaces = self.repository.list_spaces()?;
        spaces.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });

        let state = self.load_app_state()?;
        let mut summaries = Vec::new();
        for space in spaces {
            if !query.include_archived && !space.state.is_active() {
                continue;
            }

            let tasks = self.repository.list_tasks_in_space(&space.id)?;
            summaries.push(SpaceSummary {
                counts: derive_space_counts(&tasks),
                is_current: state.current_space_id.as_ref() == Some(&space.id),
                space,
            });
        }

        Ok(summaries)
    }

    pub fn show_space(&self, query: ShowSpaceQuery) -> Result<SpaceDetails, AppError> {
        let space = self.resolve_space(&query.space_ref, false)?;
        let tasks = self.repository.list_tasks_in_space(&space.id)?;
        let state = self.load_app_state()?;

        Ok(SpaceDetails {
            counts: derive_space_counts(&tasks),
            is_current: state.current_space_id.as_ref() == Some(&space.id),
            space,
        })
    }

    pub fn use_space(&self, command: SetCurrentSpaceCommand) -> Result<Space, AppError> {
        let space = self.resolve_space(&command.space_ref, true)?;
        let mut state = self.load_app_state()?;
        state.current_space_id = Some(space.id.clone());
        self.repository.save_state(&state)?;
        Ok(space)
    }

    pub fn rename_space(&self, command: RenameSpaceCommand) -> Result<Space, AppError> {
        ensure_non_empty_space_name(&command.new_name)?;

        let mut space = self.resolve_space(&command.space_ref, false)?;
        space.rename(command.new_name, OffsetDateTime::now_utc());
        self.repository.save_space(&space)?;
        Ok(space)
    }

    pub fn resolve_space(&self, reference: &str, require_active: bool) -> Result<Space, AppError> {
        let spaces = self.repository.list_spaces()?;
        let space_id = resolve_space_ref(reference, spaces.iter())?;
        let space = spaces
            .into_iter()
            .find(|space| space.id == space_id)
            .expect("resolved space id must exist");

        if require_active && !space.state.is_active() {
            return Err(AppError::ArchivedSpace(space.id.as_str().to_owned()));
        }

        Ok(space)
    }

    pub fn resolve_effective_space(
        &self,
        reference: Option<&str>,
        require_active: bool,
    ) -> Result<Space, AppError> {
        if let Some(reference) = reference {
            return self.resolve_space(reference, require_active);
        }

        let state = self.load_app_state()?;
        let current_space_id = state
            .current_space_id
            .ok_or(AppError::MissingCurrentSpace)?;
        let space = self.repository.load_space(&current_space_id)?;
        if require_active && !matches!(space.state, SpaceState::Active) {
            return Err(AppError::ArchivedSpace(space.id.as_str().to_owned()));
        }

        Ok(space)
    }
}

fn next_available_slug(base: &str, spaces: &[Space]) -> String {
    if spaces.iter().all(|space| space.slug != base) {
        return base.to_owned();
    }

    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if spaces.iter().all(|space| space.slug != candidate) {
            return candidate;
        }
    }

    unreachable!("slug generation always finds a free suffix")
}
