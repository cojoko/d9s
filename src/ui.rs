use crate::app::{App, ConnectionStatus};
use crate::views::ContextsView;
use crate::views::{DefaultView, PipelinesView, RunsView, ViewType, ViewUI};
use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::Line,
    widgets::*,
};

pub fn draw(f: &mut Frame, app: &App) {
    log::debug!(
        "Drawing UI frame. Current view: {:?}, Items count: {}",
        app.view,
        app.items.len()
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Context info
            Constraint::Length(1), // Command/search input when visible
            Constraint::Min(0),    // Main content
        ])
        .split(f.area());

    // Context bar with URL and status
    let status_style = match &app.connection_status {
        ConnectionStatus::Connected => Style::default().fg(Color::Green),
        ConnectionStatus::Failed(_) => Style::default().fg(Color::Red),
        ConnectionStatus::Disconnected => Style::default().fg(Color::Yellow),
    };

    let status_text = match &app.connection_status {
        ConnectionStatus::Connected => "Connected",
        ConnectionStatus::Failed(err) => {
            if err.len() > 30 {
                &err[..30]
            } else {
                err
            }
        }
        ConnectionStatus::Disconnected => "Disconnected",
    };

    let context_line = Line::from(vec![
        Span::raw("URL: "),
        Span::styled(&app.dagster_url, Style::default().fg(Color::Blue)),
        Span::raw(" | Status: "),
        Span::styled(status_text, status_style),
    ]);

    let context = Paragraph::new(context_line);
    f.render_widget(context, chunks[0]);

    // Command or search input
    if app.command_mode {
        let input = Paragraph::new(format!(": {}", app.command_input))
            .style(Style::default().bg(Color::DarkGray));
        f.render_widget(input, chunks[1]);
    } else if app.search_mode {
        let input = Paragraph::new(format!("/{}", app.search_input))
            .style(Style::default().bg(Color::DarkGray));
        f.render_widget(input, chunks[1]);
    }

    // Main content area - delegate to the appropriate view
    match &app.view {
        ViewType::Run(_) => {
            if let Some(run_view) = &app.run_view {
                run_view.draw(f, app, chunks[2]);
            }
        }
        ViewType::Runs => {
            let runs_view = RunsView::new();
            runs_view.draw(f, app, chunks[2]);
        }
        ViewType::PipelineRuns(_) => {
            // Use the same runs view but the filtering is handled in the view implementation
            let runs_view = RunsView::new();
            runs_view.draw(f, app, chunks[2]);
        }
        ViewType::Pipelines => {
            let pipelines_view = PipelinesView::new();
            pipelines_view.draw(f, app, chunks[2]);
        }
        ViewType::Contexts => {
            let contexts_view = ContextsView::new();
            contexts_view.draw(f, app, chunks[2]);
        }
        ViewType::Default => {
            let default_view = DefaultView;
            default_view.draw(f, app, chunks[2]);
        }
    }
}
