use crate::{
    CompositorError, CompositorSnapshot, ConfigError, FullscreenState, Output, OutputId,
    ShellConfig, Window, WindowId, Workspace, WorkspaceId,
};

/// Features exposed by a compositor adapter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompositorCapabilities {
    pub monitors: bool,
    pub workspaces: bool,
    pub windows: bool,
    pub focused_output: bool,
    pub focused_workspace: bool,
    pub focused_window: bool,
    pub fullscreen_state: bool,
    pub focus_workspace: bool,
    pub focus_window: bool,
    pub event_stream: bool,
}

/// Application-level categories for compositor events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositorEventKind {
    Output,
    Workspace,
    Window,
    Focus,
    Fullscreen,
}

/// A translated event carrying a coherent state snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositorEvent {
    pub kind: CompositorEventKind,
    pub snapshot: CompositorSnapshot,
}

/// Blocking event source used by an adapter-specific listener.
pub trait CompositorEventStream: Send {
    fn next_event(&mut self) -> Result<CompositorEvent, CompositorError>;
}

/// Application-facing contract for compositor state and commands.
///
/// A compositor adapter translates its native protocol into these domain
/// types. Native payloads must not cross this boundary.
pub trait CompositorPort: Send + Sync {
    fn outputs(&self) -> Result<Vec<Output>, CompositorError>;

    fn workspaces(&self) -> Result<Vec<Workspace>, CompositorError>;

    fn windows(&self) -> Result<Vec<Window>, CompositorError>;

    fn focus_workspace(&self, workspace_id: WorkspaceId) -> Result<(), CompositorError>;

    fn focus_window(&self, _window_id: WindowId) -> Result<(), CompositorError> {
        Err(CompositorError::unsupported("focus window"))
    }

    fn focused_output(&self) -> Result<Option<OutputId>, CompositorError> {
        Err(CompositorError::unsupported("focused output"))
    }

    fn focused_workspace(&self) -> Result<Option<WorkspaceId>, CompositorError> {
        Err(CompositorError::unsupported("focused workspace"))
    }

    fn focused_window(&self) -> Result<Option<Window>, CompositorError> {
        Err(CompositorError::unsupported("focused window"))
    }

    fn fullscreen_state(&self) -> Result<FullscreenState, CompositorError> {
        Err(CompositorError::unsupported("fullscreen state"))
    }

    fn capabilities(&self) -> CompositorCapabilities {
        CompositorCapabilities::default()
    }

    fn snapshot(&self) -> Result<CompositorSnapshot, CompositorError> {
        Err(CompositorError::unsupported("compositor snapshot"))
    }

    fn subscribe_events(&self) -> Result<Box<dyn CompositorEventStream>, CompositorError> {
        Err(CompositorError::unsupported("compositor event stream"))
    }
}

/// Application-facing contract for persistent configuration.
pub trait ConfigPort: Send + Sync {
    fn load(&self) -> Result<ShellConfig, ConfigError>;

    fn save(&self, config: &ShellConfig) -> Result<(), ConfigError>;
}
