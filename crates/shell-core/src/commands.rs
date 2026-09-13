use crate::WorkspaceId;

/// Intent emitted by a feature surface and executed by the application layer.
///
/// UI code can construct this value without knowing anything about Hyprland's
/// dispatcher syntax or IPC transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellCommand {
    FocusWorkspace(WorkspaceId),
}
