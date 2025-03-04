use crate::views::ViewType;
use crossterm::event::KeyCode;

#[derive(Debug, Clone)]
pub enum KeyAction {
    Quit,
    NavigateBack,
    ToggleCommandMode,
    UpdateCommandInput(char),
    ClearCommandInput,
    ExecuteCommand,
    ToggleSearchMode,
    CommitSearch,
    CancelSearch,
    UpdateSearchInput(char),
    ClearSearchInput,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    SelectNext(usize),     
    SelectPrevious(usize), 
    ViewDetails,          
    ViewPipelineRuns,    
    Ignored,
    SwitchContext(String),
    AddContext,
    DeleteContext,
}

pub fn handle_key(
    key: KeyCode,
    view: &ViewType,
    command_mode: bool,
    search_mode: bool,
    selected_index: usize,
) -> KeyAction {
    if command_mode {
        match key {
            KeyCode::Esc => KeyAction::ToggleCommandMode,
            KeyCode::Char(c) => KeyAction::UpdateCommandInput(c),
            KeyCode::Backspace => KeyAction::ClearCommandInput,
            KeyCode::Enter => KeyAction::ExecuteCommand,
            _ => KeyAction::Ignored,
        }
    } else if search_mode {
        match key {
            KeyCode::Esc => KeyAction::CancelSearch,
            KeyCode::Char(c) => KeyAction::UpdateSearchInput(c),
            KeyCode::Backspace => KeyAction::ClearSearchInput,
            KeyCode::Enter => KeyAction::CommitSearch,
            _ => KeyAction::Ignored,
        }
    } else {
        match key {
            KeyCode::Char('q') => KeyAction::Quit,
            KeyCode::Char(':') => KeyAction::ToggleCommandMode,
            // Only allow search toggle in searchable views
            KeyCode::Char('/') => match view {
                ViewType::Runs | ViewType::PipelineRuns(_) | ViewType::Pipelines => {
                    KeyAction::ToggleSearchMode
                }
                // Ignore search for other views
                _ => KeyAction::Ignored,
            },
            KeyCode::Esc => KeyAction::NavigateBack,
            _ => match view {
                ViewType::Run(_) => handle_run_view_key(key, selected_index),
                ViewType::Runs => handle_runs_view_key(key, selected_index),
                ViewType::Contexts => handle_contexts_view_key(key, selected_index),
                ViewType::Pipelines => handle_pipelines_view_key(key, selected_index),
                ViewType::PipelineRuns(_) => handle_runs_view_key(key, selected_index),
                ViewType::Default => handle_default_view_key(key),
            },
        }
    }
}
fn handle_run_view_key(key: KeyCode, _selected_index: usize) -> KeyAction {
    match key {
        KeyCode::Char('j') | KeyCode::Down => KeyAction::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => KeyAction::ScrollUp,
        KeyCode::Char('h') | KeyCode::Left => KeyAction::ScrollLeft,
        KeyCode::Char('l') | KeyCode::Right => KeyAction::ScrollRight,
        _ => KeyAction::Ignored,
    }
}

fn handle_runs_view_key(key: KeyCode, selected_index: usize) -> KeyAction {
    match key {
        KeyCode::Char('j') | KeyCode::Down => KeyAction::SelectNext(0), // viewport_height will be filled later
        KeyCode::Char('k') | KeyCode::Up => KeyAction::SelectPrevious(0), // viewport_height will be filled later
        KeyCode::Enter if selected_index >= 2 => {
            // We'll get the run ID from the selected item in apply_key_action
            KeyAction::ViewDetails
        }
        _ => KeyAction::Ignored,
    }
}

fn handle_default_view_key(key: KeyCode) -> KeyAction {
    match key {
        _ => KeyAction::Ignored,
    }
}

fn handle_contexts_view_key(key: KeyCode, selected_index: usize) -> KeyAction {
    match key {
        KeyCode::Char('j') | KeyCode::Down => KeyAction::SelectNext(0),
        KeyCode::Char('k') | KeyCode::Up => KeyAction::SelectPrevious(0),
        KeyCode::Enter if selected_index >= 2 => KeyAction::SwitchContext(String::new()),
        KeyCode::Char('a') => KeyAction::AddContext,
        KeyCode::Char('d') if selected_index >= 2 => KeyAction::DeleteContext,
        _ => KeyAction::Ignored,
    }
}

fn handle_pipelines_view_key(key: KeyCode, selected_index: usize) -> KeyAction {
    match key {
        KeyCode::Char('j') | KeyCode::Down => KeyAction::SelectNext(0), // viewport_height will be filled later
        KeyCode::Char('k') | KeyCode::Up => KeyAction::SelectPrevious(0), // viewport_height will be filled later
        KeyCode::Enter if selected_index >= 2 => {
            // We'll get the pipeline name later since we don't have access to items here
            KeyAction::ViewPipelineRuns
        }
        _ => KeyAction::Ignored,
    }
}
