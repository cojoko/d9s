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

pub struct ContextsView;

impl ContextsView {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ViewPoller for ContextsView {
    async fn poll(
        &self,
        _app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // No polling needed for context view
        Ok(())
    }
}

#[async_trait::async_trait]
impl ViewUI for ContextsView {
    fn draw(&self, f: &mut Frame, app: &App, area: Rect) {
        let viewport_height = area.height as usize;

        // Create spans for each visible item
        let visible_items: Vec<Line> = app
            .items
            .iter()
            .skip(app.list_offset)
            .take(viewport_height as usize)
            .enumerate()
            .map(|(i, item)| {
                let actual_index = i + app.list_offset;
                let is_selected = actual_index == app.selected_index;

                if actual_index == 0 {
                    // Header
                    Line::styled(
                        "CONTEXT NAME     URL                                      RUNS LIMIT",
                        Style::default().add_modifier(Modifier::BOLD),
                    )
                } else if actual_index == 1 {
                    // Separator
                    Line::from("-".repeat((area.width - 2) as usize))
                } else {
                    let mut style = Style::default();

                    // Current context in green
                    if item.starts_with('*') {
                        style = style.fg(Color::Green);
                    }

                    // Selection highlighting
                    if is_selected {
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    }

                    Line::styled(item.clone(), style)
                }
            })
            .collect();

        let paragraph = Paragraph::new(visible_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Contexts ")
                .title_alignment(Alignment::Center),
        );

        f.render_widget(paragraph, area);

        // Footer with keybindings
        let footer_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);

        let footer = Line::from(vec![
            Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Add | "),
            Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Delete | "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Select | "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Back"),
        ]);

        let footer_widget = Paragraph::new(footer).alignment(Alignment::Center);
        f.render_widget(footer_widget, footer_area);
    }

    async fn restore_state(&self, app: &mut App) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Populate contexts list
        app.populate_contexts_list();

        // Restore previous selection and scroll position if available
        app.restore_view_state();

        // If no previous state, set defaults
        if app.selected_index < 2 {
            app.selected_index = 2; // Skip header and separator
            app.list_offset = 0;
        }

        Ok(())
    }
}
