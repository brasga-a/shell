use serde::{Deserialize, Serialize};

use crate::ConfigError;

/// Version of the on-disk configuration contract.
pub const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompositorBackend {
    #[default]
    Hyprland,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarPosition {
    #[default]
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyboardModeConfig {
    #[default]
    None,
    Exclusive,
    #[serde(rename = "on-demand")]
    OnDemand,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShellAction {
    #[serde(rename = "launcher.toggle")]
    ToggleLauncher,
    #[serde(rename = "notifications.toggle")]
    ToggleNotifications,
    #[serde(rename = "dock.toggle")]
    ToggleDock,
    #[serde(rename = "notch.toggle")]
    ToggleNotch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub animations: bool,
    pub startup_delay_ms: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            animations: true,
            startup_delay_ms: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct CompositorConfig {
    pub backend: CompositorBackend,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ModuleRegistry {
    pub bar: bool,
    pub notch: bool,
    pub dock: bool,
    pub launcher: bool,
    pub notifications: bool,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self {
            bar: true,
            notch: true,
            dock: false,
            launcher: true,
            notifications: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ColorConfig {
    pub background: String,
    pub foreground: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            background: "#090909".to_string(),
            foreground: "#EAE6E5".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ThemeScale {
    pub sm: u32,
    pub md: u32,
    pub lg: u32,
}

impl Default for ThemeScale {
    fn default() -> Self {
        Self {
            sm: 4,
            md: 8,
            lg: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct MotionConfig {
    pub fast_ms: u64,
    pub normal_ms: u64,
    pub slow_ms: u64,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            fast_ms: 120,
            normal_ms: 220,
            slow_ms: 360,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub colors: ColorConfig,
    pub radius: ThemeScale,
    pub spacing: ThemeScale,
    pub motion: MotionConfig,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyBinding {
    pub keys: Vec<String>,
    pub action: ShellAction,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeybindConfig {
    pub bindings: Vec<KeyBinding>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct AnimationConfig {
    pub enabled: bool,
    pub duration_ms: u64,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_ms: 220,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct BarConfig {
    pub enabled: bool,
    pub position: BarPosition,
    pub height: u32,
    pub reserve_space: bool,
    pub keyboard: KeyboardModeConfig,
    pub modules_left: Vec<String>,
    pub modules_center: Vec<String>,
    pub modules_right: Vec<String>,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: BarPosition::Top,
            height: 32,
            reserve_space: true,
            keyboard: KeyboardModeConfig::None,
            modules_left: vec!["workspaces".to_string()],
            modules_center: vec!["clock".to_string()],
            modules_right: vec![
                "network".to_string(),
                "audio".to_string(),
                "battery".to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct NotchConfig {
    pub enabled: bool,
    pub width: u32,
    pub collapsed_height: u32,
    pub expanded_height: u32,
    pub animation: AnimationConfig,
}

impl Default for NotchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 420,
            collapsed_height: 32,
            expanded_height: 420,
            animation: AnimationConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DockConfig {
    pub enabled: bool,
    pub position: BarPosition,
    pub auto_hide: bool,
    pub icon_size: u32,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: BarPosition::Bottom,
            auto_hide: true,
            icon_size: 42,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct LauncherConfig {
    pub enabled: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct NotificationsConfig {
    pub enabled: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Validated application configuration assembled from the modular TOML files.
///
/// Runtime values such as focused windows, active workspaces, and service
/// status do not belong here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellConfig {
    pub schema_version: u32,
    pub general: GeneralConfig,
    pub compositor: CompositorConfig,
    pub modules: ModuleRegistry,
    pub theme: ThemeConfig,
    pub keybinds: KeybindConfig,
    pub bar: BarConfig,
    pub notch: NotchConfig,
    pub dock: DockConfig,
    pub launcher: LauncherConfig,
    pub notifications: NotificationsConfig,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_CONFIG_VERSION,
            general: GeneralConfig::default(),
            compositor: CompositorConfig::default(),
            modules: ModuleRegistry::default(),
            theme: ThemeConfig::default(),
            keybinds: KeybindConfig::default(),
            bar: BarConfig::default(),
            notch: NotchConfig::default(),
            dock: DockConfig::default(),
            launcher: LauncherConfig::default(),
            notifications: NotificationsConfig::default(),
        }
    }
}

impl ShellConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version == 0 {
            return Err(ConfigError::validation(
                "configuration schema version must be greater than zero",
            ));
        }

        if self.schema_version > CURRENT_CONFIG_VERSION {
            return Err(ConfigError::version(
                self.schema_version,
                CURRENT_CONFIG_VERSION,
            ));
        }

        validate_hex_color(&self.theme.colors.background, "theme.colors.background")?;
        validate_hex_color(&self.theme.colors.foreground, "theme.colors.foreground")?;

        if self.bar.height == 0 {
            return Err(ConfigError::validation(
                "bar.height must be greater than zero",
            ));
        }
        if self.notch.width == 0 || self.notch.expanded_height == 0 {
            return Err(ConfigError::validation(
                "notch.width and notch.expanded_height must be greater than zero",
            ));
        }
        if self.dock.icon_size == 0 {
            return Err(ConfigError::validation(
                "dock.icon_size must be greater than zero",
            ));
        }

        for binding in &self.keybinds.bindings {
            if binding.keys.is_empty() {
                return Err(ConfigError::validation(
                    "key binding must contain at least one key",
                ));
            }
        }

        Ok(())
    }
}

fn validate_hex_color(value: &str, field: &str) -> Result<(), ConfigError> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 || u32::from_str_radix(value, 16).is_err() {
        return Err(ConfigError::validation(format!(
            "{field} must be a six-digit hexadecimal color"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CURRENT_CONFIG_VERSION, ShellConfig};

    #[test]
    fn default_config_uses_current_schema() {
        let config = ShellConfig::default();

        assert_eq!(config.schema_version, CURRENT_CONFIG_VERSION);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn future_schema_is_rejected() {
        let config = ShellConfig {
            schema_version: CURRENT_CONFIG_VERSION + 1,
            ..ShellConfig::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn invalid_theme_color_is_rejected() {
        let mut config = ShellConfig::default();
        config.theme.colors.background = "not-a-color".to_string();

        assert!(config.validate().is_err());
    }
}
