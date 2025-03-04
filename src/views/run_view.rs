use crate::app::{App, ConnectionStatus};
use crate::get_run::{get_run, run_query};
use crate::views::ViewUI;
use async_trait::async_trait;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::ViewPoller;

#[derive(Clone, Debug)]
pub struct Run {
    pub run_id: String,
    pub job_name: String,
    pub status: String,
    pub run_config_yaml: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
}

pub struct RunView {
    pub run_id: String,
    pub details: Option<Run>,
    pub scroll_offset: usize,
    pub horizontal_scroll: usize,
}

impl RunView {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            details: None,
            scroll_offset: 0,
            horizontal_scroll: 0,
        }
    }

    pub async fn fetch_details(
        &mut self,
        dagster_url: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match get_run(self.run_id.clone(), dagster_url.to_string()).await {
            Ok(data) => match data.run_or_error {
                run_query::RunQueryRunOrError::Run(run_data) => {
                    self.details = Some(Run {
                        run_id: run_data.run_id,
                        job_name: run_data.job_name,
                        status: format!("{:?}", run_data.status),
                        run_config_yaml: run_data.run_config_yaml,
                        start_time: run_data.start_time,
                        end_time: run_data.end_time,
                    });
                    Ok(())
                }
                run_query::RunQueryRunOrError::RunNotFoundError(err) => Err(Box::new(
                    std::io::Error::new(std::io::ErrorKind::Other, err.message),
                )),
                run_query::RunQueryRunOrError::PythonError(err) => Err(Box::new(
                    std::io::Error::new(std::io::ErrorKind::Other, err.message),
                )),
            },
            Err(e) => Err(e),
        }
    }
}

pub struct RunPoller;

#[async_trait]
impl ViewPoller for RunPoller {
    async fn poll(
        &self,
        app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (should_poll, dagster_url, run_id) = {
            let app_lock = app.lock().await;
            match &app_lock.view {
                super::ViewType::Run(id) => (true, app_lock.dagster_url.clone(), id.clone()),
                _ => (false, String::new(), String::new()),
            }
        };

        if !should_poll {
            return Ok(());
        }

        match get_run(run_id, dagster_url).await {
            Ok(data) => {
                let mut app_lock = app.lock().await;
                match data.run_or_error {
                    run_query::RunQueryRunOrError::Run(run_data) => {
                        if let Some(run_view) = &mut app_lock.run_view {
                            run_view.details = Some(Run {
                                run_id: run_data.run_id,
                                job_name: run_data.job_name,
                                status: format!("{:?}", run_data.status),
                                run_config_yaml: run_data.run_config_yaml,
                                start_time: run_data.start_time,
                                end_time: run_data.end_time,
                            });
                        }
                        app_lock.connection_status = ConnectionStatus::Connected;
                    }
                    run_query::RunQueryRunOrError::RunNotFoundError(err) => {
                        app_lock.connection_status = ConnectionStatus::Failed(err.message);
                    }
                    run_query::RunQueryRunOrError::PythonError(err) => {
                        app_lock.connection_status = ConnectionStatus::Failed(err.message);
                    }
                }
                Ok(())
            }
            Err(e) => {
                let mut app_lock = app.lock().await;
                app_lock.connection_status = ConnectionStatus::Failed(e.to_string());
                Err(e)
            }
        }
    }
}

#[async_trait]
impl ViewPoller for RunView {
    async fn poll(
        &self,
        app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        RunPoller.poll(app).await
    }
}

#[async_trait::async_trait]
impl ViewUI for RunView {
    fn draw(&self, f: &mut Frame, _app: &App, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render all content in a single block
        if let Some(details) = &self.details {
            log::debug!("Rendering run details: {}", details.run_id);

            let content_block = Block::default()
                .borders(Borders::ALL)
                .title(" Run Details ")
                .title_alignment(Alignment::Center);

            let main_area = chunks[0];
            f.render_widget(content_block, main_area);

            // Create inner area for scrollable content
            let inner_area = main_area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            });

            let mut content_lines = Vec::new();

            // Format timestamps with both UTC and local time
            let format_full_timestamp = crate::utils::format_full_timestamp;

            // Add all information
            content_lines.extend_from_slice(&[
                Line::from(vec![
                    Span::styled(
                        "Run ID:      ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&details.run_id),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Status:      ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &details.status,
                        crate::utils::get_status_style(&details.status),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Job Name:    ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&details.job_name),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Start Time:  ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format_full_timestamp(details.start_time)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "End Time:    ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format_full_timestamp(details.end_time)),
                ]),
                Line::from(""), // Empty line as separator
                Line::from(vec![Span::styled(
                    "Run Configuration:",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""), // Empty line before YAML
            ]);

            // Add YAML configuration with proper indentation
            let yaml_lines: Vec<Line> = details
                .run_config_yaml
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect();

            content_lines.extend(yaml_lines);

            let content = Paragraph::new(content_lines)
                .scroll((self.scroll_offset as u16, self.horizontal_scroll as u16));
            f.render_widget(content, inner_area);

            // Footer with keybindings
            let footer = Line::from(vec![
                Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" Back | "),
                Span::styled("↑/k", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" Scroll Up | "),
                Span::styled("↓/j", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" Scroll Down | "),
                Span::styled("←/h", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" Scroll Left | "),
                Span::styled("→/l", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" Scroll Right"),
            ]);

            let footer_widget = Paragraph::new(footer).alignment(Alignment::Center);
            f.render_widget(footer_widget, chunks[1]);
        } else {
            // Loading state
            let loading = Paragraph::new("Loading run details...")
                .block(Block::default())
                .alignment(Alignment::Center);
            f.render_widget(loading, chunks[0]);
        }
    }

    async fn restore_state(&self, _app: &mut App) -> Result<(), Box<dyn Error + Send + Sync>> {
        // RunView state restoration is minimal since the view is created fresh each time
        log::debug!("Restoring RunView state for run_id: {}", self.run_id);

        // We already have the run view set up, so nothing more to do
        Ok(())
    }
}
