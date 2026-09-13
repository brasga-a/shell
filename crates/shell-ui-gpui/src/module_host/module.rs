use std::fmt;

use gpui::AnyElement;
use shell_core::NotchConfig;

/// Stable identity for the initial statically linked Luna modules.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModuleId {
    Launcher,
    Calendar,
    Player,
    Theme,
    Resources,
    Clock,
    Settings,
}

impl ModuleId {
    pub const ALL: [Self; 7] = [
        Self::Launcher,
        Self::Calendar,
        Self::Player,
        Self::Theme,
        Self::Resources,
        Self::Clock,
        Self::Settings,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Launcher => "launcher",
            Self::Calendar => "calendar",
            Self::Player => "player",
            Self::Theme => "theme",
            Self::Resources => "resources",
            Self::Clock => "clock",
            Self::Settings => "settings",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "launcher" => Some(Self::Launcher),
            "calendar" => Some(Self::Calendar),
            "player" => Some(Self::Player),
            "theme" => Some(Self::Theme),
            "resources" => Some(Self::Resources),
            "clock" => Some(Self::Clock),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Route owned by the Notch host. Module activation is not encoded as a
/// feature-specific enum branch in the host renderer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NotchRoute {
    #[default]
    Idle,
    Module(ModuleId),
}

/// Metadata exposed to the host without exposing a concrete module type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleMetadata {
    pub id: ModuleId,
    pub title: &'static str,
    pub aliases: &'static [&'static str],
}

/// Preferred content dimensions. The Notch owns the layer-shell surface and
/// uses this value only to calculate its own target geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModuleSize {
    pub width: f32,
    pub height: f32,
}

impl ModuleSize {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn from_config(config: &NotchConfig, expanded: bool) -> Self {
        if expanded {
            Self::new(config.width as f32, config.expanded_height as f32)
        } else {
            Self::new(
                config.collapsed_width as f32,
                config.collapsed_height as f32,
            )
        }
    }
}

/// Type-erased GPUI element returned by a module render call.
pub type ModuleView = AnyElement;

/// Internal module contract. Implementations own feature state and content;
/// the Notch remains responsible for routing, focus, geometry, and animation.
pub trait NotchModule: 'static {
    fn metadata(&self) -> ModuleMetadata;

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize;

    fn render(&mut self, context: crate::ModuleContext) -> ModuleView;
}
