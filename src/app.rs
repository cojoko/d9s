use crate::KeyAction;
use crate::config::{Config, ContextConfig};
use crate::views::{
    ContextsView, DefaultView, PipelinesView, Run, RunPoller, RunView, RunsView, ViewPoller,
    ViewType, ViewUI,
};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

// Core Data Structures

/// Main application state containing all runtime information
pub struct App {
    // View state
    pub view: ViewType,
    pub view_history: Vec<ViewType>,
    pub next_view: Option<(ViewType, bool)>, 
    pub view_state_cache: HashMap<ViewType, (usize, usize)>, 
    pub run_view: Option<RunView>,

    // UI state
    pub selected_index: usize,
    pub list_offset: usize,
    pub items: Vec<String>,
    pub unfiltered_items: Vec<String>,

    // Input state
    pub command_mode: bool,
    pub command_input: String,
    pub search_mode: bool,
    pub search_input: String,
    pub has_committed_filter: bool,

    // Data and connection state
    pub dagster_url: String,
    pub connection_status: ConnectionStatus,
    pub state: Arc<Mutex<AppState>>, // Shared state for thread communication
    pub config: Config,
}

/// Data state shared between threads
#[derive(Debug)]
pub struct AppState {
    pub runs: Vec<Run>,
    pub pipelines: Vec<crate::get_pipelines::Pipeline>,
    pub cursor: String,
    pub selected_pipeline: Option<String>,
}

/// Connection status enum for displaying in the UI
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Failed(String),
    Disconnected,
}

/// Column configuration for run list views
#[derive(Debug, Clone, Copy)]
pub struct ColumnConfig {
    pub width: usize,
    pub min_width: usize,
    pub priority: usize, // Lower = higher priority when space is limited
    pub name: &'static str,
}

/// Collection of column configurations for run views
pub struct ColumnsConfig {
    pub run_id: ColumnConfig,
    pub pipeline: ColumnConfig,
    pub status: ColumnConfig,
    pub duration: ColumnConfig,
    pub start_time: ColumnConfig,
}

impl ColumnsConfig {
    pub fn new() -> Self {
        Self {
            run_id: ColumnConfig {
                width: 36,
                min_width: 36,
                priority: 1,
                name: "RUN ID",
            },
            pipeline: ColumnConfig {
                width: 30,
                min_width: 10,
                priority: 2,
                name: "PIPELINE",
            },
            duration: ColumnConfig {
                width: 15,
                min_width: 8,
                priority: 3,
                name: "DURATION",
            },
            status: ColumnConfig {
                width: 15,
                min_width: 7,
                priority: 4,
                name: "STATUS",
            },
            start_time: ColumnConfig {
                width: 25,
                min_width: 12,
                priority: 5,
                name: "START TIME",
            },
        }
    }

    pub fn get_ordered_columns(&self) -> Vec<&ColumnConfig> {
        vec![
            &self.run_id,
            &self.pipeline,
            &self.status,
            &self.duration,
            &self.start_time,
        ]
    }

    /// Calculates column widths based on available space, respecting priorities.
    /// If there's not enough space for all columns, those with lower priority are hidden first.
    /// Any extra space is distributed to columns that need it most.
    pub fn calculate_widths(&self, available_width: u16) -> Vec<usize> {
        let total_spacing = 4; // 4 spaces between 5 columns
        let available = available_width as usize - total_spacing;

        let mut columns = self.get_ordered_columns();
        columns.sort_by_key(|c| c.priority);

        let mut widths: Vec<usize> = columns.iter().map(|c| c.min_width).collect();
        let min_total: usize = widths.iter().sum();

        if available <= min_total {
            let mut current_total = min_total;
            for col in columns.iter().rev() {
                if current_total <= available {
                    break;
                }
                let idx = columns
                    .iter()
                    .position(|c| c.priority == col.priority)
                    .unwrap();
                current_total -= widths[idx];
                widths[idx] = 0;
            }
        } else {
            let extra_space = available - min_total;
            let mut remaining_space = extra_space;

            for (i, col) in columns.iter().enumerate() {
                if remaining_space == 0 {
                    break;
                }
                let current = widths[i];
                let desired = col.width;
                if current < desired {
                    let extra = (desired - current).min(remaining_space);
                    widths[i] += extra;
                    remaining_space -= extra;
                }
            }

            if remaining_space > 0 {
                for i in 0..widths.len() {
                    if widths[i] > 0 {
                        widths[i] += remaining_space / widths.len();
                    }
                }
            }
        }

        widths
    }
}

// Default Implementations

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runs: vec![],
            pipelines: vec![],
            cursor: String::new(),
            selected_pipeline: None,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// App Implementation - Core Functionality

impl App {
    /// Create a new application instance with default settings
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        let current_context = config.get_current_context();

        let mut app = Self {
            // View state
            view: ViewType::Default,
            view_history: Vec::new(),
            next_view: None,
            view_state_cache: HashMap::new(),
            run_view: None,

            // UI state
            selected_index: 0,
            list_offset: 0,
            items: Vec::new(),
            unfiltered_items: Vec::new(),

            // Input state
            command_mode: false,
            command_input: String::new(),
            search_mode: false,
            search_input: String::new(),
            has_committed_filter: false,

            // Data and connection state
            dagster_url: current_context.url,
            connection_status: ConnectionStatus::Disconnected,
            state: Arc::new(Mutex::new(AppState::default())),
            config,
        };

        // Populate help text
        DefaultView::populate_help_text(&mut app);

        app
    }

    /// Apply a key action with the given viewport height
    pub async fn apply_key_action(
        &mut self,
        action: KeyAction,
        viewport_height: usize,
    ) -> Result<(), Box<dyn Error>> {
        match action {
            // Input mode actions
            KeyAction::ToggleCommandMode => self.toggle_command_mode(),
            KeyAction::UpdateCommandInput(c) => self.command_input.push(c),
            KeyAction::ClearCommandInput => self.clear_command_input(),
            KeyAction::ExecuteCommand => self.execute_command().await,

            // Search actions
            KeyAction::ToggleSearchMode => self.enter_search_mode(),
            KeyAction::CommitSearch => self.commit_search(),
            KeyAction::CancelSearch => self.cancel_search(),
            KeyAction::UpdateSearchInput(c) => self.update_search(c),
            KeyAction::ClearSearchInput => self.clear_search_input(),

            // Navigation actions
            KeyAction::NavigateBack => {
                if let Err(e) = self.navigate_back().await {
                    log::error!("Error navigating back: {:?}", e);
                }
            }
            KeyAction::SelectNext(vh) => {
                let vh = if vh == 0 { viewport_height } else { vh };
                self.next_item(vh);
            }
            KeyAction::SelectPrevious(vh) => {
                let vh = if vh == 0 { viewport_height } else { vh };
                self.previous_item(vh);
            }

            // View actions
            KeyAction::ViewDetails => self.enter_run_details_view().await,
            KeyAction::ViewPipelineRuns => self.enter_pipeline_runs_view().await,

            // Context actions
            KeyAction::SwitchContext(context_name) => self.switch_context(context_name),
            KeyAction::AddContext => self.add_context(),
            KeyAction::DeleteContext => self.delete_context(),

            // Scrolling actions
            KeyAction::ScrollDown => self.scroll_down(),
            KeyAction::ScrollUp => self.scroll_up(),
            KeyAction::ScrollLeft => self.scroll_left(),
            KeyAction::ScrollRight => self.scroll_right(),

            // No-op actions
            KeyAction::Ignored | KeyAction::Quit => {}
        }
        Ok(())
    }

    /// Start background polling for various views
    pub async fn start_polling(app: Arc<Mutex<App>>) {
        let mut interval = interval(Duration::from_secs(3));

        loop {
            interval.tick().await;

            let (view, poller) = {
                let app_lock = app.lock().await;
                (app_lock.view.clone(), App::get_poller(&app_lock.view))
            };

            log::debug!("Polling for view: {:?}", view);

            if let Err(e) = poller.poll(app.clone()).await {
                log::error!("Polling error for {:?} view: {:?}", view, e);
            }
        }
    }

    /// Get the appropriate view poller for the current view
    fn get_poller(view: &ViewType) -> Box<dyn ViewPoller + Send> {
        match view {
            ViewType::Runs => Box::new(RunsView::new()),
            ViewType::Default => Box::new(DefaultView),
            ViewType::Run(_) => Box::new(RunPoller),
            ViewType::Contexts => Box::new(ContextsView::new()),
            ViewType::Pipelines => Box::new(PipelinesView::new()),
            ViewType::PipelineRuns(_) => Box::new(RunsView::new()),
        }
    }
}

// View Navigation

impl App {
    /// Enter a new view, optionally resetting navigation history
    pub async fn enter_view(
        &mut self,
        view_type: ViewType,
        reset_history: bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Skip unnecessary view changes
        if self.view == view_type {
            return Ok(());
        }

        if matches!(view_type, ViewType::Runs) {
            let mut state = self.state.lock().await;
            state.selected_pipeline = None;
        }

        log::debug!(
            "Entering view {:?}, reset_history={}",
            view_type,
            reset_history
        );

        // Save current view state
        self.save_view_state();

        // Clear any active search when changing views
        self.search_mode = false;
        self.search_input.clear();
        self.has_committed_filter = false;
        self.unfiltered_items.clear();

        // Handle view history
        self.update_view_history(view_type.clone(), reset_history);

        // Set the new view
        self.view = view_type.clone();

        // Log the current history stack for debugging
        let history_str: Vec<String> = self
            .view_history
            .iter()
            .map(|v| format!("{:?}", v))
            .collect();
        log::debug!("Current history stack: [{}]", history_str.join(" -> "));

        // Get the appropriate view object and call restore_state
        match &self.view {
            ViewType::Default => {
                DefaultView.restore_state(self).await?;
            }
            ViewType::Runs => {
                RunsView::new().restore_state(self).await?;
            }
            ViewType::Contexts => {
                ContextsView::new().restore_state(self).await?;
            }
            ViewType::Pipelines => {
                PipelinesView::new().restore_state(self).await?;
            }
            ViewType::PipelineRuns(_) => {
                // Handle showing runs for a specific pipeline
                RunsView::new().restore_state(self).await?;
            }
            ViewType::Run(run_id) => {
                // Run view is handled specially since it needs to load data for a specific run
                if self.run_view.is_some() {
                    log::debug!("Run view already set up for run_id: {}", run_id);
                } else {
                    log::warn!("Entering run view but run_view is None!");
                }
            }
        }

        Ok(())
    }

    /// Navigate back to the previous view
    pub async fn navigate_back(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        log::debug!(
            "navigate_back called, history depth: {}",
            self.view_history.len()
        );

        // Debug log the current history stack
        let history_str: Vec<String> = self
            .view_history
            .iter()
            .map(|v| format!("{:?}", v))
            .collect();
        log::debug!("Current history stack: [{}]", history_str.join(" -> "));

        // Save current view state before navigating away
        self.save_view_state();

        if let Some(previous_view) = self.view_history.pop() {
            log::debug!("Navigating back to {:?}", previous_view);

            // Update to the previous view without modifying history again
            self.view = previous_view;

            // Get the appropriate view object and call restore_state
            match &self.view {
                ViewType::Default => {
                    DefaultView.restore_state(self).await?;
                }
                ViewType::Runs => {
                    RunsView::new().restore_state(self).await?;
                }
                ViewType::Contexts => {
                    ContextsView::new().restore_state(self).await?;
                }
                ViewType::Pipelines => {
                    PipelinesView::new().restore_state(self).await?;
                }
                ViewType::PipelineRuns(_) => {
                    // When going back to pipeline runs, restore with the pipeline filter
                    RunsView::new().restore_state(self).await?;
                }
                ViewType::Run(run_id) => {
                    if self.run_view.is_some() {
                        // No specific restoration needed for Run view
                        log::debug!("Restored Run view for run_id: {}", run_id);
                    } else {
                        log::warn!("Navigating back to run view but run_view is None!");
                    }
                }
            }

            Ok(true)
        } else if !matches!(self.view, ViewType::Default) {
            // If no history but not in default view, go to default
            log::debug!("No history, falling back to default view");
            self.view = ViewType::Default;
            DefaultView.restore_state(self).await?;
            Ok(true)
        } else {
            log::debug!("At default view with empty history, not navigating");
            Ok(false)
        }
    }

    /// Updates view history when entering a new view
    fn update_view_history(&mut self, view_type: ViewType, reset_history: bool) {
        // If we're resetting history, clear it before changing views
        if reset_history {
            log::debug!("Resetting view history");
            self.view_history.clear();

            // If we're not going to the default view, add default as the base of history
            if !matches!(view_type, ViewType::Default) {
                self.view_history.push(ViewType::Default);
                log::debug!("Added default view to history base");
            }
        } else {
            // Only add to history if actually changing views
            log::debug!("Adding current view {:?} to history", self.view);
            self.view_history.push(self.view.clone());
        }
    }

    /// Save the current view state for future restoration
    pub fn save_view_state(&mut self) {
        // Don't save state for Run view (detail view) or Default view
        if !matches!(&self.view, ViewType::Default) && !matches!(&self.view, ViewType::Run(_)) {
            self.view_state_cache
                .insert(self.view.clone(), (self.selected_index, self.list_offset));
            log::debug!(
                "Stored state for {:?}: selected_index={}, list_offset={}",
                self.view,
                self.selected_index,
                self.list_offset
            );
        }
    }

    /// Restore a previously saved view state
    pub fn restore_view_state(&mut self) {
        if let Some(&(selected_index, list_offset)) = self.view_state_cache.get(&self.view) {
            self.selected_index = selected_index;
            self.list_offset = list_offset;
            log::debug!(
                "Restored state for {:?}: selected_index={}, list_offset={}",
                self.view,
                selected_index,
                list_offset
            );
        }
    }
}

// Selection and Scrolling

impl App {
    /// Select the next item in the list
    pub fn next_item(&mut self, viewport_height: usize) {
        if self.items.len() <= 2 {
            return;
        } // No items beyond header/separator

        let max_index = self.items.len() - 1;
        let next_index = (self.selected_index + 1).min(max_index);

        // Only proceed if we can actually move
        if next_index != self.selected_index {
            let last_visible = self.list_offset + viewport_height - 1;

            // If we would move to the last visible position, scroll first
            if next_index >= last_visible {
                self.list_offset += 1;
            }

            self.selected_index = next_index;
        }
    }

    /// Select the previous item in the list
    pub fn previous_item(&mut self, _viewport_height: usize) {
        if self.items.len() <= 2 {
            return;
        } // No items beyond header/separator

        let prev_index = self.selected_index.max(2).saturating_sub(1).max(2);

        // Only proceed if we can actually move
        if prev_index != self.selected_index {
            self.selected_index = prev_index;

            // If we're moving above the visible area, scroll one line up
            if self.selected_index < self.list_offset {
                self.list_offset = self.list_offset.saturating_sub(1);
            }
        }
    }

    /// Scroll down in the detail view
    fn scroll_down(&mut self) {
        if let ViewType::Run(_) = self.view {
            if let Some(run_view) = &mut self.run_view {
                run_view.scroll_offset = run_view.scroll_offset.saturating_add(1);
            }
        }
    }

    /// Scroll up in the detail view
    fn scroll_up(&mut self) {
        if let ViewType::Run(_) = self.view {
            if let Some(run_view) = &mut self.run_view {
                run_view.scroll_offset = run_view.scroll_offset.saturating_sub(1);
            }
        }
    }

    /// Scroll left in the detail view
    fn scroll_left(&mut self) {
        if let ViewType::Run(_) = self.view {
            if let Some(run_view) = &mut self.run_view {
                run_view.horizontal_scroll = run_view.horizontal_scroll.saturating_sub(1);
            }
        }
    }

    /// Scroll right in the detail view
    fn scroll_right(&mut self) {
        if let ViewType::Run(_) = self.view {
            if let Some(run_view) = &mut self.run_view {
                run_view.horizontal_scroll = run_view.horizontal_scroll.saturating_add(1);
            }
        }
    }
}

// Search and Filtering

impl App {
    /// Apply the current search filter to items
    pub fn apply_search_filter(&mut self) {
        if self.search_input.is_empty() {
            // If search is empty, restore all items
            self.items = self.unfiltered_items.clone();
            return;
        }

        use crate::search::fuzzy_match;

        // Keep header and separator (first two rows in most views)
        let header = self.unfiltered_items.get(0).cloned().unwrap_or_default();
        let separator = self.unfiltered_items.get(1).cloned().unwrap_or_default();

        // Filter the data rows
        let filtered: Vec<String> = self
            .unfiltered_items
            .iter()
            .skip(2) // Skip header and separator
            .filter(|item| fuzzy_match(item, &self.search_input))
            .cloned()
            .collect();

        // Rebuild items with header, separator, and filtered data
        self.items = vec![header, separator];
        self.items.extend(filtered);

        // Reset selection to first item if selection is now out of bounds
        if self.selected_index < 2 || self.selected_index >= self.items.len() {
            self.selected_index = 2.min(self.items.len().saturating_sub(1));
            self.list_offset = 0;
        }

        log::debug!(
            "Applied search filter '{}': {} items reduced to {}",
            self.search_input,
            self.unfiltered_items.len().saturating_sub(2),
            self.items.len().saturating_sub(2)
        );
    }

    /// Enter search mode
    fn enter_search_mode(&mut self) {
        // Entering search mode
        self.search_mode = true;
        self.search_input.clear();

        // Save unfiltered items
        self.unfiltered_items = self.items.clone();

        // Exit command mode if entering search mode
        self.command_mode = false;
    }

    /// Update search with new character and reapply filter
    fn update_search(&mut self, c: char) {
        self.search_input.push(c);
        self.apply_search_filter();
    }

    /// Clear the last character from search input
    fn clear_search_input(&mut self) {
        if !self.search_input.is_empty() {
            self.search_input.pop();
            self.apply_search_filter();
        } else {
            // Exit search mode if backspace on empty search
            self.search_mode = false;
        }
    }

    /// Commit the current search (keep filtered results but exit search mode)
    fn commit_search(&mut self) {
        // Exit search mode but keep filtered results
        self.search_mode = false;
        self.has_committed_filter = true;
    }

    /// Cancel search and restore unfiltered results
    fn cancel_search(&mut self) {
        // Exit search mode and restore unfiltered results
        self.search_mode = false;
        self.search_input.clear();
        self.has_committed_filter = false; // Clear the flag

        // Restore unfiltered items
        if !self.unfiltered_items.is_empty() {
            self.items = self.unfiltered_items.clone();
        }
    }
}

// Command Mode

impl App {
    /// Toggle command input mode
    fn toggle_command_mode(&mut self) {
        self.command_mode = !self.command_mode;
        if self.command_mode {
            self.command_input.clear();
            // Exit search mode if entering command mode
            self.search_mode = false;
        }
    }

    /// Clear last character from command input
    fn clear_command_input(&mut self) {
        self.command_input.pop();
        if self.command_input.is_empty() {
            self.command_mode = false;
        }
    }

    /// Execute the current command
    pub async fn execute_command(&mut self) {
        log::debug!("Executing command: {}", self.command_input);

        if self.command_input.starts_with("url ") {
            let new_url = self.command_input.trim_start_matches("url ").to_string();
            log::debug!("Setting new Dagster URL: {}", new_url);
            self.dagster_url = new_url;
            self.connection_status = ConnectionStatus::Disconnected;
        } else if self.command_input.starts_with("context ") {
            self.execute_context_command().await;
        } else if self.command_input.starts_with("context-add ") {
            self.execute_context_add_command();
        } else {
            self.execute_standard_command().await;
        }

        self.command_mode = false;
        self.command_input.clear();
    }

    /// Execute a context switching command
    async fn execute_context_command(&mut self) {
        let context_name = self
            .command_input
            .trim_start_matches("context ")
            .to_string();
        if let Err(e) = self.config.set_context(&context_name) {
            log::error!("Failed to switch context: {}", e);
            self.connection_status = ConnectionStatus::Failed(format!("Context error: {}", e));
        } else {
            // Update the URL from the new context
            let context_config = self.config.get_current_context();
            self.dagster_url = context_config.url;
            self.connection_status = ConnectionStatus::Disconnected;
            log::debug!("Switched to context: {}", context_name);
        }
    }

    /// Execute a context add command
    fn execute_context_add_command(&mut self) {
        // Parse context-add command: "context-add name url [runs_limit]"
        let args = self
            .command_input
            .trim_start_matches("context-add ")
            .split_whitespace()
            .collect::<Vec<&str>>();

        if args.len() < 2 {
            log::error!(
                "Invalid context-add command. Format: context-add <name> <url> [<runs_limit>]"
            );
            self.connection_status = ConnectionStatus::Failed(
                "Invalid context-add command. Format: context-add <name> <url> [<runs_limit>]"
                    .to_string(),
            );
            return;
        }

        let name = args[0];
        let url = args[1];
        let runs_limit = if args.len() > 2 {
            match args[2].parse::<usize>() {
                Ok(limit) => Some(limit),
                Err(_) => {
                    log::warn!("Invalid runs_limit value: {}, using default", args[2]);
                    None
                }
            }
        } else {
            None
        };

        // Create a new context configuration
        let context_config = ContextConfig {
            url: url.to_string(),
            runs_limit,
        };

        // Add the new context
        if let Err(e) = self.config.add_context(name, context_config) {
            log::error!("Failed to add context: {}", e);
            self.connection_status = ConnectionStatus::Failed(format!("Context error: {}", e));
        } else {
            log::debug!("Added new context: {}", name);

            // Refresh the contexts list if we're on the Contexts view
            if let ViewType::Contexts = self.view {
                self.populate_contexts_list();
            }

            self.connection_status = ConnectionStatus::Connected;
        }
    }

    /// Execute a standard command (run, pipelines, etc.)
    /// Make sure nav history is reset when explicilty navigating via command.
    async fn execute_standard_command(&mut self) {
        match self.command_input.as_str() {
            "contexts" => {
                if let Err(e) = self.enter_view(ViewType::Contexts, true).await {
                    log::error!("Failed to switch to contexts view: {:?}", e);
                }
            }
            "runs" => {
                log::debug!("Switching to runs view");
                {
                    let mut state = self.state.lock().await;
                    state.selected_pipeline = None;
                }
                if let Err(e) = self.enter_view(ViewType::Runs, true).await {
                    log::error!("Failed to switch to runs view: {:?}", e);
                }
            }
            "pipelines" => {
                log::debug!("Switching to pipelines view");
                if let Err(e) = self.enter_view(ViewType::Pipelines, true).await {
                    log::error!("Failed to switch to pipelines view: {:?}", e);
                }
            }
            "debug" => {
                let state = self.state.lock().await;
                log::info!("App Debug Info:");
                log::info!("View: {:?}", self.view);
                log::info!("Items count: {}", self.items.len());
                log::info!("State runs count: {}", state.runs.len());
                log::info!("State pipelines count: {}", state.pipelines.len());
                log::info!("History depth: {}", self.view_history.len());
            }
            "q" => std::process::exit(0),
            _ => {
                log::debug!("Unknown command: {}", self.command_input);
            }
        }
    }
}

// Context Management

impl App {
    /// Populate the list of contexts
    pub fn populate_contexts_list(&mut self) {
        self.items.clear();
        self.items.push(
            "CONTEXT NAME     URL                                      RUNS LIMIT".to_string(),
        );
        self.items.push("-".repeat(80));

        let current = self.config.last_context.clone();

        for (name, context) in &self.config.contexts {
            let prefix = if *name == current { "* " } else { "  " };
            let limit = context
                .runs_limit
                .map_or("default".to_string(), |l| l.to_string());

            let name_col = format!("{}{:<15}", prefix, name);
            let url_col = format!("{:<40}", context.url);
            let limit_col = format!("{:<10}", limit);

            self.items
                .push(format!("{} {} {}", name_col, url_col, limit_col));
        }

        self.selected_index = 2; // Point to first context after header
        self.list_offset = 0;
    }

    /// Enter a specific run details
    async fn enter_run_details_view(&mut self) {
        if matches!(self.view, ViewType::Runs | ViewType::PipelineRuns(_)){
            if self.selected_index >= 2 {
                // Save current view state before switching
                self.save_view_state();

                if let Some(selected_item) = self.items.get(self.selected_index) {
                    let run_id = selected_item
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string();
                    let mut run_view = RunView::new(run_id.clone());
                    let dagster_url = self.dagster_url.clone();

                    // Perform initial fetch
                    if let Err(e) = run_view.fetch_details(&dagster_url).await {
                        log::error!("Failed to fetch initial run details: {}", e);
                        self.connection_status = ConnectionStatus::Failed(e.to_string());
                    } else {
                        self.connection_status = ConnectionStatus::Connected;
                    }

                    // Save the run view (we'll need it later)
                    self.run_view = Some(run_view);

                    // Use enter_view with reset_history=false to preserve navigation history
                    if let Err(e) = self.enter_view(ViewType::Run(run_id), false).await {
                        log::error!("Failed to enter run details view: {:?}", e);
                    }
                }
            }
        }
    }

    /// Enter the pipeline runs view
    async fn enter_pipeline_runs_view(&mut self) {
        if let ViewType::Pipelines = self.view {
            if self.selected_index >= 2 {
                // Save current view state before switching
                self.save_view_state();

                if let Some(selected_item) = self.items.get(self.selected_index) {
                    // Extract pipeline name from selected item (first column)
                    // The pipeline name could contain spaces, so we need to be careful
                    // We know that we format it as a fixed-width column of 30 chars
                    let pipeline_name = selected_item
                        .trim()
                        .chars()
                        .take(30)
                        .collect::<String>()
                        .trim()
                        .to_string();

                    // Store selected pipeline in state
                    {
                        let mut state = self.state.lock().await;
                        state.selected_pipeline = Some(pipeline_name.clone());
                    }

                    // Navigate to the PipelineRuns view for this pipeline
                    if let Err(e) = self
                        .enter_view(ViewType::PipelineRuns(pipeline_name), false)
                        .await
                    {
                        log::error!("Failed to enter pipeline runs view: {:?}", e);
                    }
                }
            }
        }
    }

    /// Switch to a different context
    fn switch_context(&mut self, context_name: String) {
        if let ViewType::Contexts = self.view {
            let name = if context_name.is_empty() {
                // If no name provided, extract from selected item
                if let Some(item) = self.items.get(self.selected_index) {
                    let parts: Vec<&str> = item.trim().split_whitespace().collect();
                    if let Some(name) = parts.get(0) {
                        name.trim_start_matches('*').trim().to_string()
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                context_name
            };

            if let Err(e) = self.config.set_context(&name) {
                self.connection_status = ConnectionStatus::Failed(format!("Context error: {}", e));
            } else {
                let context = self.config.get_current_context();
                self.dagster_url = context.url;
                self.connection_status = ConnectionStatus::Disconnected;
                self.populate_contexts_list();
            }
        }
    }

    /// Enter context add mode
    fn add_context(&mut self) {
        // For now, just enter command mode with a template
        self.command_mode = true;
        self.command_input = "context-add name url".to_string();
    }

    /// Delete the selected context
    fn delete_context(&mut self) {
        if let ViewType::Contexts = self.view {
            if let Some(item) = self.items.get(self.selected_index) {
                let parts: Vec<&str> = item.trim().split_whitespace().collect();
                if let Some(name) = parts.get(0) {
                    let name = name.trim_start_matches('*').trim();
                    if let Err(e) = self.config.remove_context(name) {
                        self.connection_status =
                            ConnectionStatus::Failed(format!("Cannot delete: {}", e));
                    } else {
                        self.populate_contexts_list();
                    }
                }
            }
        }
    }
}
