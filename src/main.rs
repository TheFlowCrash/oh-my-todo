use oh_my_todo::application::AppError;
use oh_my_todo::{BootstrapOptions, bootstrap, cli, tui};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), AppError> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let context = bootstrap(BootstrapOptions::default())?;

    match args.first().map(String::as_str) {
        None | Some("tui") => tui::run(&context),
        Some(_) => cli::run(&context, &args),
    }
}
