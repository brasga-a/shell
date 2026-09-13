use std::sync::mpsc;

use gpui::SharedString;
use shell_core::{CompositorSnapshot, NotchConfig, ShellCommand};
use shell_theme::DesignTokens;

use super::ModuleId;

/// Renderer/application state made available to a module during one render.
///
/// The context deliberately contains domain values and an application command
/// sender only. It does not expose a Wayland surface, compositor socket, or
/// filesystem handle to feature modules.
#[derive(Clone, Debug)]
pub struct ModuleContext {
    pub module: ModuleId,
    pub module_key: String,
    pub tokens: DesignTokens,
    pub notch: NotchConfig,
    pub clock: SharedString,
    pub query: String,
    pub snapshot: CompositorSnapshot,
    pub commands: mpsc::Sender<ShellCommand>,
}
