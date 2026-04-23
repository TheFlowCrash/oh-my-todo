use crate::application::bootstrap::AppContext;
use crate::application::error::AppError;

pub fn run(context: &AppContext) -> Result<(), AppError> {
    let spaces = context.space_service.list_spaces()?;
    let state = context.space_service.load_app_state()?;

    println!("oh-my-todo TUI adapter (stage 1)");
    println!("data root: {}", context.data_root().display());
    println!("spaces loaded: {}", spaces.len());
    println!("view: {:?}", state.current_view);
    println!("sort: {:?}", state.current_sort);

    Ok(())
}
