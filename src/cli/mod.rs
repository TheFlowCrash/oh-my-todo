use crate::application::bootstrap::AppContext;
use crate::application::error::AppError;

pub fn run(context: &AppContext, args: &[String]) -> Result<(), AppError> {
    let spaces = context.space_service.list_spaces()?;
    let current_space = context.space_service.load_app_state()?.current_space_id;

    println!("oh-my-todo CLI adapter (stage 1)");
    println!("data root: {}", context.data_root().display());
    println!("spaces loaded: {}", spaces.len());
    println!(
        "current space: {}",
        current_space
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("<none>")
    );
    if args.is_empty() {
        println!("received no CLI subcommand");
    } else {
        println!("received args: {}", args.join(" "));
    }

    Ok(())
}
