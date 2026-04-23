use crate::application::bootstrap::AppContext;
use crate::application::error::AppError;
use crate::application::queries::ListSpacesQuery;
use crate::domain::{SortMode, SpaceId, ViewMode};

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub space_id: Option<SpaceId>,
    pub view: Option<ViewMode>,
    pub sort: Option<SortMode>,
}

pub fn run(context: &AppContext) -> Result<(), AppError> {
    run_with_options(context, LaunchOptions::default())
}

pub fn run_with_options(context: &AppContext, options: LaunchOptions) -> Result<(), AppError> {
    let spaces = context.space_service.list_spaces(ListSpacesQuery {
        include_archived: false,
    })?;
    let state = context.space_service.load_app_state()?;
    let current_space = options
        .space_id
        .or(state.current_space_id)
        .map(|id| id.as_str().to_owned())
        .unwrap_or_else(|| "<none>".to_owned());
    let view = options.view.unwrap_or(state.current_view);
    let sort = options.sort.unwrap_or(state.current_sort);

    println!("oh-my-todo TUI adapter (stage 2)");
    println!("data root: {}", context.data_root().display());
    println!("spaces loaded: {}", spaces.len());
    println!("current space: {}", current_space);
    println!("view: {:?}", view);
    println!("sort: {:?}", sort);

    Ok(())
}
