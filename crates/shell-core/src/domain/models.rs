use super::{OutputId, Rect, WindowId, WorkspaceId};

/// A compositor output tracked by the application.
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub id: OutputId,
    pub name: String,
    pub geometry: Rect,
    /// Logical-to-physical scale. This is intentionally fractional.
    pub scale: f32,
}

impl Output {
    pub fn new(id: OutputId, name: impl Into<String>, geometry: Rect, scale: f32) -> Self {
        Self {
            id,
            name: name.into(),
            geometry,
            scale,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.scale.is_finite() && self.scale > 0.0
    }
}

/// A compositor workspace translated into an application-level type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub output_id: Option<OutputId>,
    pub name: Option<String>,
    pub active: bool,
}

impl Workspace {
    pub fn new(id: WorkspaceId, output_id: Option<OutputId>) -> Self {
        Self {
            id,
            output_id,
            name: None,
            active: false,
        }
    }
}

/// A compositor window translated into an application-level type.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub id: WindowId,
    pub output_id: Option<OutputId>,
    pub workspace_id: Option<WorkspaceId>,
    pub title: String,
    pub app_id: Option<String>,
    pub geometry: Rect,
    pub focused: bool,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl Window {
    pub fn new(id: WindowId, geometry: Rect) -> Self {
        Self {
            id,
            output_id: None,
            workspace_id: None,
            title: String::new(),
            app_id: None,
            geometry,
            focused: false,
            maximized: false,
            fullscreen: false,
        }
    }
}

/// A compositor monitor. It is intentionally an alias of the output model so
/// surface ownership and compositor state use the same stable identifier.
pub type Monitor = Output;

/// The currently focused window, translated into application-owned data.
pub type FocusedWindow = Window;

/// Fullscreen state reported by the compositor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FullscreenState {
    #[default]
    Inactive,
    Active {
        window_id: WindowId,
    },
}

/// A coherent compositor snapshot consumed by application state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompositorSnapshot {
    pub monitors: Vec<Monitor>,
    pub workspaces: Vec<Workspace>,
    pub windows: Vec<Window>,
    pub focused_output: Option<OutputId>,
    pub focused_workspace: Option<WorkspaceId>,
    pub focused_window: Option<FocusedWindow>,
    pub fullscreen: FullscreenState,
}
