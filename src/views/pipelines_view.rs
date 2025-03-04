use crate::app::{App, ConnectionStatus};
use crate::get_pipelines::{Pipeline, get_pipelines};
use crate::utils::{get_status_style, truncate};
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

use super::{ViewPoller, ViewUI};

pub struct PipelinesView;

impl PipelinesView {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn fetch_initial_data(
        &self,
        app: &mut App,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pipelines = self.fetch_pipelines(app).await?;

        // Store pipelines in app state
        let mut state = app.state.lock().await;
        state.pipelines = pipelines;
        drop(state);

        self.populate_pipelines_items(app).await;
        Ok(())
    }

    pub async fn fetch_pipelines(
        &self,
        app: &App,
    ) -> Result<Vec<Pipeline>, Box<dyn std::error::Error + Send + Sync>> {
        match get_pipelines(app.dagster_url.clone()).await {
            Ok(pipelines) => Ok(pipelines),
            Err(e) => Err(e),
        }
    }

    // Populate a target vector
    pub async fn populate_pipelines_items_into(&self, app: &App, target: &mut Vec<String>) {
        log::debug!("populate_pipelines_items_into: Starting population");

        let pipelines = {
            let state_lock = app.state.lock().await;
            state_lock.pipelines.clone()
        };

        target.clear();
        target.push("PIPELINE NAME                                        REPOSITORY LOCATION                  LAST RUN STATUS".to_string());

        // Add separator
        target.push("-".repeat(80));

        // Add the pipeline data
        for pipeline in &pipelines {
            // Skip asset jobs as per requirement
            if pipeline.is_asset_job {
                continue;
            }

            let status = pipeline
                .last_run_status
                .clone()
                .unwrap_or_else(|| "None".to_string());

            let name_col = format!("{:<50}", truncate(&pipeline.name, 50));
            let repo_col = format!("{:<33}", truncate(&pipeline.repository_location, 33));
            let status_col = status;

            target.push(format!("{} {} {}", name_col, repo_col, status_col));
        }

        log::debug!(
            "populate_pipelines_items_into: Created {} formatted items for target",
            target.len()
        );
    }

    pub async fn populate_pipelines_items(&self, app: &mut App) {
        // We can't borrow app and app.items simultaneously, so:
        let pipelines = {
            let state_lock = app.state.lock().await;
            state_lock.pipelines.clone()
        };

        // Now create the formatted items directly
        let mut items = Vec::new();

        // Add header with properly aligned column names
        items.push("PIPELINE NAME                                        REPOSITORY LOCATION                  LAST RUN STATUS".to_string());

        // Add separator
        items.push("-".repeat(80));

        // Add the pipeline data
        for pipeline in &pipelines {
            // Skip asset jobs, I'm not sure if we actually want to do this.
            // TODO: Look into how to get the identifier for an asset job, then display them in
            // pipelines.
            if pipeline.is_asset_job {
                continue;
            }

            let status = pipeline
                .last_run_status
                .clone()
                .unwrap_or_else(|| "None".to_string());

            let name_col = format!("{:<50}", truncate(&pipeline.name, 50));
            let repo_col = format!("{:<33}", truncate(&pipeline.repository_location, 33));
            let status_col = status;

            items.push(format!("{} {} {}", name_col, repo_col, status_col));
        }

        app.items = items;

        log::debug!(
            "populate_pipelines_items: Created {} formatted items for UI",
            app.items.len()
        );
    }
}

#[async_trait]
impl ViewPoller for PipelinesView {
    async fn poll(
        &self,
        app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (should_poll, dagster_url) = {
            let app_lock = app.lock().await;
            (
                matches!(app_lock.view, super::ViewType::Pipelines),
                app_lock.dagster_url.clone(),
            )
        };

        if !should_poll {
            return Ok(());
        }

        log::debug!("PipelinesPoller: Starting poll");

        match get_pipelines(dagster_url).await {
            Ok(pipelines) => {
                let mut app_lock = app.lock().await;

                // Only update if we're still in the Pipelines view
                if !matches!(app_lock.view, super::ViewType::Pipelines) {
                    return Ok(());
                }

                app_lock.connection_status = ConnectionStatus::Connected;

                // Check if there's an active filter
                let has_filter = !app_lock.unfiltered_items.is_empty()
                    && app_lock.unfiltered_items != app_lock.items;

                {
                    let mut state = app_lock.state.lock().await;
                    state.pipelines = pipelines;
                }

                if has_filter {
                    // If there's an active filter, update unfiltered_items first
                    // then re-apply the filter
                    let mut unfiltered_items = app_lock.unfiltered_items.clone();
                    // Update a temporary vector
                    self.populate_pipelines_items_into(&app_lock, &mut unfiltered_items)
                        .await;
                    // Assign it back
                    app_lock.unfiltered_items = unfiltered_items;
                    app_lock.apply_search_filter();
                } else {
                    // No active filter, update items directly
                    self.populate_pipelines_items(&mut app_lock).await;
                }
                Ok(())
            }
            Err(e) => {
                let mut app_lock = app.lock().await;

                // Only update error status if we're still in the Pipelines view
                if matches!(app_lock.view, super::ViewType::Pipelines) {
                    app_lock.connection_status = ConnectionStatus::Failed(e.to_string());
                }

                Err(e)
            }
        }
    }
}

#[async_trait::async_trait]
impl ViewUI for PipelinesView {
    fn draw(&self, f: &mut Frame, app: &App, area: Rect) {
        let viewport_height = area.height as usize;
        let viewport_width = area.width;

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
                    Line::styled(item.clone(), Style::default().add_modifier(Modifier::BOLD))
                } else if actual_index == 1 {
                    // Separator
                    Line::from("-".repeat((viewport_width - 2) as usize))
                } else {
                    // For pipeline rows, parse the status for styling
                    let parts: Vec<&str> = item.split_whitespace().collect();
                    let status_idx = parts.len().saturating_sub(1); // Last word is status

                    // Create spans for each part of the item
                    let mut style = Style::default();

                    // Apply status styling if we can find it
                    if let Some(status) = parts.get(status_idx) {
                        style = get_status_style(status);
                    }

                    // Add selection highlighting
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
                .title(" Pipelines ")
                .title_alignment(Alignment::Center),
        );

        f.render_widget(paragraph, area);

        // Footer with keybindings
        let footer_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);

        let footer = Line::from(vec![
            Span::styled("↑/k", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("↓/j", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Navigate | "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" View Runs | "),
            Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Search | "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Back"),
        ]);

        let footer_widget = Paragraph::new(footer).alignment(Alignment::Center);
        f.render_widget(footer_widget, footer_area);
    }

    async fn restore_state(&self, app: &mut App) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Clear any existing data
        app.items.clear();

        // Show loading state
        app.items.push(
            "PIPELINE NAME                                        REPOSITORY LOCATION                  LAST RUN STATUS".to_string(),
        );
        app.items.push("-".repeat(80));
        app.items.push("Loading pipelines...".to_string());

        // Restore previous selection and scroll position if available
        app.restore_view_state();

        // If no previous state, set defaults
        if app.selected_index < 2 {
            app.selected_index = 2; // Skip header and separator
            app.list_offset = 0;
        }

        // Fetch fresh data
        if let Err(e) = self.fetch_initial_data(app).await {
            log::error!("Failed to load pipelines data: {:?}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to load data: {}", e),
            )));
        }

        Ok(())
    }
}
