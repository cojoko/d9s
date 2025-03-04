use crate::app::{App, ColumnsConfig, ConnectionStatus};
use crate::get_runs::{Variables, get_runs, runs_query};
use crate::utils::{format_duration, format_timestamp, get_status_style, truncate};
use crate::views::ViewType;
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

use super::{Run, ViewPoller, ViewUI};

pub struct RunsView;

impl RunsView {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn fetch_initial_data(
        &self,
        app: &mut App,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let runs = self.fetch_runs(app).await?;

        let mut state = app.state.lock().await;
        state.runs = runs;
        drop(state);

        self.populate_runs_items(app).await;
        Ok(())
    }

    pub async fn fetch_runs(
        &self,
        app: &App,
    ) -> Result<Vec<Run>, Box<dyn std::error::Error + Send + Sync>> {
        // Get pipeline_name from view if we're in PipelineRuns view
        let pipeline_name = match &app.view {
            ViewType::PipelineRuns(name) => name.clone(),
            _ => {
                // If we're not in PipelineRuns view but have a selected pipeline, use that
                let state = app.state.lock().await;
                state.selected_pipeline.clone().unwrap_or_default()
            }
        };

        let variables = Variables {
            pipeline_name,
            cursor: app.state.lock().await.cursor.clone(),
            run_ids: vec![],
        };
        let runs_limit = app.config.get_current_context().runs_limit;
        match get_runs(variables, app.dagster_url.clone(), runs_limit).await {
            Ok(data) => {
                let runs = match data.runs_or_error {
                    runs_query::RunsQueryRunsOrError::Runs(runs_data) => runs_data
                        .results
                        .into_iter()
                        .map(|run| Run {
                            run_id: run.run_id,
                            job_name: run.job_name,
                            status: format!("{:?}", run.status),
                            run_config_yaml: run.run_config_yaml,
                            start_time: run.start_time,
                            end_time: run.end_time,
                        })
                        .collect(),
                    _ => vec![],
                };
                Ok(runs)
            }
            Err(e) => Err(e),
        }
    }

    // Populate a target vector

    pub async fn populate_runs_items_into(&self, app: &App, target: &mut Vec<String>) {
        log::debug!("populate_runs_items_into: Starting population");

        let runs = {
            let state_lock = app.state.lock().await;
            state_lock.runs.clone()
        };

        // Format runs into items
        let items = runs
            .iter()
            .map(|r| {
                vec![
                    r.run_id.clone(),
                    r.job_name.clone(),
                    r.status.clone(),
                    format_duration(r.start_time, r.end_time),
                    format_timestamp(r.start_time),
                ]
            })
            .collect::<Vec<Vec<String>>>();

        // Clear target and fill with new data
        target.clear();
        target.push("HEADER".to_string());
        target.push("SEPARATOR".to_string());
        target.extend(items.into_iter().map(|row| row.join(" ")));

        log::debug!(
            "populate_runs_items_into: Created {} formatted items for UI",
            target.len()
        );
    }
    pub async fn populate_runs_items(&self, app: &mut App) {
        // We can't borrow app and app.items simultaneously, so:
        let runs = {
            let state_lock = app.state.lock().await;
            state_lock.runs.clone()
        };

        // Now create the formatted items directly
        let items = runs
            .iter()
            .map(|r| {
                vec![
                    r.run_id.clone(),
                    r.job_name.clone(),
                    r.status.clone(),
                    format_duration(r.start_time, r.end_time),
                    format_timestamp(r.start_time),
                ]
            })
            .collect::<Vec<Vec<String>>>();

        // Create the final items vector with header and separator
        app.items.clear();
        app.items.push("HEADER".to_string());
        app.items.push("SEPARATOR".to_string());
        app.items.extend(items.into_iter().map(|row| row.join(" ")));

        log::debug!(
            "populate_runs_items: Created {} formatted items for UI",
            app.items.len()
        );
    }
}

#[async_trait]
impl ViewPoller for RunsView {
    async fn poll(
        &self,
        app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (should_poll, dagster_url, runs_limit) = {
            let app_lock = app.lock().await;
            (
                matches!(app_lock.view, super::ViewType::Runs)
                    || matches!(app_lock.view, super::ViewType::PipelineRuns(_)),
                app_lock.dagster_url.clone(),
                app_lock.config.get_current_context().runs_limit,
            )
        };
        if !should_poll {
            return Ok(());
        }

        log::debug!("RunsPoller: Starting poll");

        // Get view type to determine if we're filtering by pipeline
        let (cursor, pipeline_name) = {
            let app_lock = app.lock().await;
            let state = app_lock.state.lock().await;
            let pipeline_name = match &app_lock.view {
                ViewType::PipelineRuns(name) => name.clone(),
                _ => state.selected_pipeline.clone().unwrap_or_default(),
            };
            (state.cursor.clone(), pipeline_name)
        };

        let variables = Variables {
            pipeline_name,
            cursor,
            run_ids: vec![],
        };

        let result = get_runs(variables, dagster_url, runs_limit).await;

        match result {
            Ok(data) => {
                let runs = match data.runs_or_error {
                    runs_query::RunsQueryRunsOrError::Runs(runs_data) => runs_data
                        .results
                        .into_iter()
                        .map(|run| Run {
                            run_id: run.run_id,
                            job_name: run.job_name,
                            status: format!("{:?}", run.status),
                            run_config_yaml: run.run_config_yaml,
                            start_time: run.start_time,
                            end_time: run.end_time,
                        })
                        .collect(),
                    _ => vec![],
                };

                if !runs.is_empty() {
                    let mut app_lock = app.lock().await;

                    // Only update if we're still in the Runs or PipelineRuns view
                    if !matches!(app_lock.view, super::ViewType::Runs)
                        && !matches!(app_lock.view, super::ViewType::PipelineRuns(_))
                    {
                        return Ok(());
                    }

                    app_lock.connection_status = ConnectionStatus::Connected;

                    // Check if there's an active filter - either in search mode or with a committed filter
                    let has_filter = app_lock.search_mode || app_lock.has_committed_filter;

                    {
                        let mut state = app_lock.state.lock().await;
                        state.runs = runs;
                    }

                    if has_filter {
                        // If there's an active filter, update unfiltered_items first
                        // then re-apply the filter
                        let mut unfiltered_items = Vec::new();
                        // Update a temporary vector
                        self.populate_runs_items_into(&app_lock, &mut unfiltered_items)
                            .await;
                        // Assign it back
                        app_lock.unfiltered_items = unfiltered_items;

                        // Reapply the search filter
                        app_lock.apply_search_filter();
                    } else {
                        // No active filter, update items directly
                        self.populate_runs_items(&mut app_lock).await;
                    }
                }
                Ok(())
            }
            Err(e) => {
                let mut app_lock = app.lock().await;

                // Only update error status if we're still in the Runs view
                if matches!(app_lock.view, super::ViewType::Runs)
                    || matches!(app_lock.view, super::ViewType::PipelineRuns(_))
                {
                    app_lock.connection_status = ConnectionStatus::Failed(e.to_string());
                }
                Err(e)
            }
        }
    }
}

#[async_trait::async_trait]
impl ViewUI for RunsView {
    fn draw(&self, f: &mut Frame, app: &App, area: Rect) {
    let viewport_height = area.height as usize;
    let viewport_width = area.width;

    let columns_config = ColumnsConfig::new();
    let dynamic_widths = columns_config.calculate_widths(viewport_width - 2); // Account for borders
    let columns = columns_config.get_ordered_columns();

    // Create spans for each visible item
    let visible_items: Vec<Line> = app
        .items
        .iter()
        .skip(app.list_offset) // Skip items above viewport
        .take(viewport_height as usize) // Take only what fits in viewport
        .enumerate()
        .map(|(i, item)| {
            let actual_index = i + app.list_offset;
            let is_selected = actual_index == app.selected_index;

            if actual_index == 0 {
                // Header
                let header = columns
                    .iter()
                    .zip(dynamic_widths.iter())
                    .filter(|&(_, width)| *width > 0)
                    .map(|(&col, &width)| format!("{:<width$}", col.name, width = width))
                    .collect::<Vec<_>>()
                    .join(" ");
                Line::styled(header, Style::default().add_modifier(Modifier::BOLD))
            } else if actual_index == 1 {
                // Separator
                Line::from("-".repeat((viewport_width - 2) as usize))
            } else {
                // Instead of splitting by whitespace, parse the Run data structure format
                // A run consists of: run_id, job_name, status, duration, start_time
                // We know the first 3 fields won't have spaces, so we can extract them
                let parts: Vec<&str> = item.splitn(5, ' ').collect();
                
                let mut style = if parts.len() > 2 {
                    // Get style based on status (the 3rd field)
                    get_status_style(parts[2])
                } else {
                    Style::default()
                };

                if is_selected {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }
                
                // Format the display string based on column widths
                let mut formatted = Vec::new();
                
                for (idx, &width) in dynamic_widths.iter().enumerate() {
                    if width > 0 && idx < parts.len() {
                        let content = if idx == 4 {
                            // For the start_time field, use the rest of the string
                            // to avoid breaking up the timestamp format
                            parts[4]
                        } else {
                            parts[idx]
                        };
                        
                        formatted.push(format!(
                            "{:<width$}",
                            truncate(content, width),
                            width = width
                        ));
                    }
                }

                let line = formatted.join(" ");
                Line::styled(line, style)
            }
        })
        .collect();

    let title = match &app.view {
        ViewType::PipelineRuns(pipeline_name) => {
            format!(" Runs for Pipeline: {} ", pipeline_name)
        }
        _ => " All Runs ".to_string(),
    };

    let paragraph = Paragraph::new(visible_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
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
        Span::raw(" View Details | "),
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
        app.items.push("HEADER".to_string());
        app.items.push("SEPARATOR".to_string());
        app.items.push("Loading runs...".to_string());

        // Restore previous selection and scroll position if available
        app.restore_view_state();

        // If no previous state, set defaults
        if app.selected_index < 2 {
            app.selected_index = 2; // Skip header and separator
            app.list_offset = 0;
        }

        // Fetch fresh data
        if let Err(e) = self.fetch_initial_data(app).await {
            log::error!("Failed to load runs data: {:?}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to load data: {}", e),
            )));
        }

        Ok(())
    }
}
