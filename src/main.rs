use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use env_logger::Builder;
use log::LevelFilter;
use ratatui::prelude::*;
use std::{error::Error, io, time::Duration};
use tokio::time::sleep;

mod app;
mod config;
mod get_pipelines;
mod get_run;
mod get_runs;
mod input;
mod search;
mod ui;
mod utils;
mod views;

use crate::input::{KeyAction, handle_key};
use app::App;
use std::sync::Arc;
use tokio::sync::Mutex;

// How often to refresh the UI when there are no input events
const UI_REFRESH_INTERVAL: Duration = Duration::from_millis(100);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Configure logging to file
    setup_logging()?;
    log::info!("Starting application");

    // Initialize terminal backend
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize app state
    let app = Arc::new(Mutex::new(App::new()));
    let app_clone = app.clone();

    // Spawn background polling task
    tokio::spawn(async move {
        App::start_polling(app_clone).await;
    });

    // Run the main application loop
    let res = run_app(&mut terminal, app).await;

    // Clean up terminal before exit
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Report any errors that occurred
    if let Err(err) = res {
        log::error!("Application error: {:?}", err);
        println!("{err:?}");
    }

    Ok(())
}

/// Sets up logging to write to a file with debug level
fn setup_logging() -> Result<(), Box<dyn Error>> {
    let log_file = std::fs::File::create("debug.log")?;
    Builder::new()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(LevelFilter::Debug)
        .init();
    
    Ok(())
}

/// Main application loop
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: Arc<Mutex<App>>) -> io::Result<()> {
    loop {
        // Handle pending view transitions
        {
            let mut app_guard = app.lock().await;
            if let Some((next_view, reset_history)) = app_guard.next_view.take() {
                if let Err(e) = app_guard.enter_view(next_view, reset_history).await {
                    log::error!("Failed to enter view: {:?}", e);
                }
            }
        }
        
        // Draw the UI
        {
            let app_guard = app.lock().await;
            terminal.draw(|f| ui::draw(f, &app_guard))?;
        }

        // Handle input events with timeout
        if event::poll(UI_REFRESH_INTERVAL)? {
            if let Event::Key(key) = event::read()? {
                // Get minimal app state without holding a long lock
                let (view, command_mode, search_mode, selected_index, viewport_height) = {
                    let app_guard = app.lock().await;
                    (
                        app_guard.view.clone(),
                        app_guard.command_mode,
                        app_guard.search_mode,
                        app_guard.selected_index,
                        (terminal.size()?.height as usize).saturating_sub(3),
                    )
                };

                // Process the key with the current state
                let action = handle_key(key.code, &view, command_mode, search_mode, selected_index);

                // Apply the action with a fresh lock
                match action {
                    KeyAction::Quit => return Ok(()),
                    _ => {
                        let mut app_guard = app.lock().await;
                        if let Err(e) = app_guard.apply_key_action(action, viewport_height).await {
                            log::error!("Error applying key action: {:?}", e);
                        }
                    }
                }
            }
        } else {
            // No input event, just wait a bit to prevent CPU spinning
            sleep(Duration::from_millis(10)).await;
        }
    }
}
