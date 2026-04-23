use oh_my_todo::application::AppError;
use oh_my_todo::{BootstrapOptions, bootstrap, cli, tui};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            if let Some(hint) = error.hint() {
                eprintln!("hint: {hint}");
            }
            error.exit_code()
        }
    }
}

fn run() -> Result<ExitCode, AppError> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        let context = bootstrap(BootstrapOptions::default())?;
        tui::run(&context)?;
        return Ok(ExitCode::SUCCESS);
    }

    let parsed = match cli::parse(&args) {
        Ok(parsed) => parsed,
        Err(error) => {
            let code = error.exit_code();
            error.print().expect("clap error should print");
            return Ok(ExitCode::from(code as u8));
        }
    };

    let context = bootstrap(BootstrapOptions::default())?;
    cli::handlers::dispatch(&context, parsed)?;
    Ok(ExitCode::SUCCESS)
}
