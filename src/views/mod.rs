use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::app::App;

mod contexts_view;
mod default_view;
mod pipelines_view;
mod run_view;
mod runs_view;
pub use contexts_view::ContextsView;
pub use default_view::DefaultView;
pub use pipelines_view::PipelinesView;
use ratatui::{Frame, prelude::*};
pub use run_view::{Run, RunPoller, RunView};
pub use runs_view::RunsView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewType {
    Default,
    Runs,
    Run(String),
    Contexts,
    Pipelines,
    PipelineRuns(String), 
}

// Implement Hash for ViewType so it can be used as a key in HashMap
impl Hash for ViewType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ViewType::Default => {
                state.write_u8(0);
            }
            ViewType::Runs => {
                state.write_u8(1);
            }
            ViewType::Run(run_id) => {
                state.write_u8(2);
                run_id.hash(state);
            }
            ViewType::Contexts => {
                state.write_u8(3);
            }
            ViewType::Pipelines => {
                state.write_u8(4);
            }
            ViewType::PipelineRuns(pipeline_name) => {
                state.write_u8(5);
                pipeline_name.hash(state);
            }
        }
    }
}

impl Default for ViewType {
    fn default() -> Self {
        Self::Default
    }
}

#[async_trait::async_trait]
pub trait ViewPoller {
    async fn poll(
        &self,
        app: Arc<Mutex<App>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait::async_trait]
pub trait ViewUI {
    fn draw(&self, f: &mut Frame, app: &App, area: Rect);

    // Method to restore view state
    async fn restore_state(&self, _app: &mut App) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Default implementation does nothing - views can override as needed
        Ok(())
    }
}

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait View: ViewUI + ViewPoller {
    // Nothing to see here for now
    // We are just documenting that a complete view should implement both UI and polling.
}

// Implement the combined View trait for our view types that implement both ViewUI and ViewPoller
#[async_trait::async_trait]
impl View for DefaultView {}

#[async_trait::async_trait]
impl View for RunsView {}

#[async_trait::async_trait]
impl View for ContextsView {}

#[async_trait::async_trait]
impl View for PipelinesView {}

// RunView is separate and implements both traits individually
#[async_trait::async_trait]
impl View for RunView {}
