//! Application-facing domain types and ports.
//!
//! This crate intentionally has no UI toolkit, compositor, or Linux service
//! dependencies. Adapters depend on these contracts; the reverse dependency
//! is not allowed.

mod commands;
mod config;
mod domain;
mod error;
mod metrics;
mod notch;
mod ports;
mod state;

pub use commands::ShellCommand;
pub use config::{
    AnimationConfig, BarConfig, BarPosition, CURRENT_CONFIG_VERSION, ColorConfig,
    CompositorBackend, CompositorConfig, DockConfig, GeneralConfig, KeyBinding, KeybindConfig,
    KeyboardModeConfig, LauncherConfig, ModuleRegistry, MotionConfig, NotchConfig,
    NotificationsConfig, ShellAction, ShellConfig, ThemeConfig, ThemeScale, TypographyConfig,
};
pub use domain::{
    CompositorSnapshot, FocusedWindow, FullscreenState, Monitor, Output, Point, Rect, Size, Window,
    Workspace,
};
pub use domain::{OutputId, SurfaceId, WindowId, WorkspaceId};
pub use error::{CompositorError, ConfigError, PlatformError};
pub use metrics::AppUsageMetrics;
pub use notch::{FocusManager, NotchAnimation, NotchEdge, NotchGeometry, NotchState};
pub use ports::{
    CompositorCapabilities, CompositorEvent, CompositorEventKind, CompositorEventStream,
    CompositorPort, ConfigPort,
};
pub use state::CompositorStateStore;
