use crate::app::App;
use crate::views::ViewUI;
use async_trait::async_trait;
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::ViewPoller;

pub struct DefaultView;

impl DefaultView {
    pub fn populate_help_text(app: &mut App) {
        app.items.clear();
        app.items.push("Welcome to d9s!".to_string());
        app.items.push("".to_string());
        app.items.push("Available commands:".to_string());
        app.items
            .push("  :runs - Show all pipeline runs".to_string());
        app.items
            .push("  :pipelines - Show available pipelines".to_string());
        app.items
            .push("  :contexts - Manage connection contexts".to_string());
        app.items
            .push("  :url <url> - Set Dagster GraphQL URL".to_string());
        app.items
            .push("  :context <name> - Switch to a different context".to_string());
        app.items
            .push("  :debug - Log application debug information".to_string());
        app.items.push("  :q - Quit application".to_string());
        app.items.push("".to_string());
        app.items.push("Navigation:".to_string());
        app.items.push("  Press 'q' to quit".to_string());
        app.items
            .push("  Press ':' to enter command mode".to_string());
        app.items.push("  Press 'Esc' to go back".to_string());
    }
}

#[async_trait]
impl ViewPoller for DefaultView {
    async fn poll(
        &self,
        _app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ViewUI for DefaultView {
    fn draw(&self, f: &mut Frame, app: &App, area: Rect) {
        let light_blue = Color::LightBlue;

        let lines: Vec<Line> = app
            .items
            .iter()
            .map(|s| {
                if s.starts_with("Welcome") {
                    Line::styled(
                        s.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if s.starts_with("Available commands:") || s.starts_with("Navigation:") {
                    Line::styled(
                        s.clone(),
                        Style::default().fg(light_blue).add_modifier(Modifier::BOLD),
                    )
                } else if s.starts_with("  :") {
                    // Command with different colors for the command name and description
                    // Admittedly this is a little jank but it works
                    let parts: Vec<&str> = s.splitn(2, " - ").collect();
                    if parts.len() > 1 {
                        Line::from(vec![
                            Span::styled(parts[0], Style::default().fg(Color::Cyan)),
                            Span::raw(" - "),
                            Span::raw(parts[1]),
                        ])
                    } else {
                        Line::from(s.clone())
                    }
                } else {
                    Line::from(s.clone())
                }
            })
            .collect();

        let items = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" d9s ")
                    .title_alignment(Alignment::Center),
            )
            .alignment(Alignment::Center); 

        f.render_widget(items, area);
    }

    async fn restore_state(&self, app: &mut App) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Reset items and repopulate with help text
        app.items.clear();
        DefaultView::populate_help_text(app);

        // Default view doesn't need to restore selection state
        app.selected_index = 0;
        app.list_offset = 0;

        Ok(())
    }
}
