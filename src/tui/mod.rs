mod app;
mod input;
mod render;

use crate::application::bootstrap::AppContext;
use crate::application::error::AppError;
use crate::domain::{SortMode, SpaceId, ViewMode};
use app::TuiApp;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use ratatui::DefaultTerminal;
use std::io::stdout;

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
    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    let result = run_app(&mut terminal, context, options);
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn run_app(
    terminal: &mut DefaultTerminal,
    context: &AppContext,
    options: LaunchOptions,
) -> Result<(), AppError> {
    let mut app = TuiApp::new(context, options)?;

    while !app.should_quit {
        app.clear_expired_status_message();
        terminal.draw(|frame| render::render(frame, &mut app))?;

        if let Some(timeout) = app.status_message_timeout() {
            if !event::poll(timeout)? {
                app.clear_expired_status_message();
                continue;
            }
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let changed = app.handle_key(context, key)?;
                if changed {
                    app.persist(context)?;
                }
            }
            Event::Mouse(mouse) => {
                let changed = app.handle_mouse(context, mouse)?;
                if changed {
                    app.persist(context)?;
                }
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    app.persist(context)?;
    Ok(())
}
