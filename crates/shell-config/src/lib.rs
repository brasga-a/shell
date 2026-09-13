//! TOML configuration adapter.
//!
//! The loader is the only component that knows the on-disk layout. It merges
//! the small, owned TOML files into one validated `ShellConfig` before the
//! application sees it.

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, mpsc},
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use shell_core::{
    BarConfig, CURRENT_CONFIG_VERSION, CompositorConfig, ConfigError, ConfigPort, DockConfig,
    GeneralConfig, KeybindConfig, LauncherConfig, ModuleRegistry, NotchConfig, NotificationsConfig,
    ShellConfig, ThemeConfig,
};

#[derive(Debug)]
pub struct InMemoryConfig {
    config: RwLock<ShellConfig>,
}

impl Default for InMemoryConfig {
    fn default() -> Self {
        Self {
            config: RwLock::new(ShellConfig::default()),
        }
    }
}

impl ConfigPort for InMemoryConfig {
    fn load(&self) -> Result<ShellConfig, ConfigError> {
        self.config
            .read()
            .map(|config| config.clone())
            .map_err(|_| ConfigError::unavailable("configuration lock is poisoned"))
    }

    fn save(&self, config: &ShellConfig) -> Result<(), ConfigError> {
        config.validate()?;

        self.config
            .write()
            .map(|mut current| *current = config.clone())
            .map_err(|_| ConfigError::unavailable("configuration lock is poisoned"))
    }
}

/// Owns discovery, parsing, merging, validation, diagnostics, and persistence
/// for the modular TOML configuration.
#[derive(Clone, Debug)]
pub struct ConfigLoader {
    root: PathBuf,
}

impl ConfigLoader {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Discovers `ATLANTIC_CONFIG_DIR`, XDG config, or the conventional home
    /// directory. The repository `config/` directory is a final development
    /// fallback for environments without a home directory.
    pub fn discover() -> Self {
        if let Some(root) = env::var_os("ATLANTIC_CONFIG_DIR") {
            return Self::at(root);
        }

        if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
            return Self::at(PathBuf::from(root).join("atlantic"));
        }

        if let Some(home) = env::var_os("HOME") {
            return Self::at(PathBuf::from(home).join(".config/atlantic"));
        }

        Self::at(PathBuf::from("config"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Loads a complete configuration, falling back to safe defaults when a
    /// file is missing, malformed, or fails validation during startup.
    pub fn load(&self) -> Result<ShellConfig, ConfigError> {
        match self.load_candidate() {
            Ok(config) => Ok(config),
            Err(error) => {
                tracing::warn!(%error, "invalid TOML configuration; using safe defaults");
                Ok(ShellConfig::default())
            }
        }
    }

    /// Loads and validates the complete configuration tree without applying a
    /// fallback. Hot reload uses this method so an invalid candidate can be
    /// rejected while the previous snapshot remains active.
    pub fn load_candidate(&self) -> Result<ShellConfig, ConfigError> {
        let global: GlobalFile = self.read_or_default("config.toml")?;
        let theme: ThemeFile = self.read_or_default("theme.toml")?;
        let keybinds: KeybindFile = self.read_or_default("keybinds.toml")?;
        self.warn_about_unknown_modules();

        let modules = self.root.join("modules");
        let bar: BarConfigFile = self.read_or_default_from(&modules, "bar.toml")?;
        let notch: NotchConfigFile = self.read_or_default_from(&modules, "notch.toml")?;
        let dock: DockConfigFile = self.read_or_default_from(&modules, "dock.toml")?;
        let launcher: LauncherConfigFile = self.read_or_default_from(&modules, "launcher.toml")?;
        let notifications: NotificationsConfigFile =
            self.read_or_default_from(&modules, "notifications.toml")?;

        let config = ShellConfig {
            schema_version: global.version,
            general: global.general,
            compositor: global.compositor,
            modules: global.modules,
            theme: theme.into_config(),
            keybinds: keybinds.into_config(),
            bar: bar.into_config(),
            notch: notch.into_config(),
            dock: dock.into_config(),
            launcher: launcher.into_config(),
            notifications: notifications.into_config(),
        };

        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, config: &ShellConfig) -> Result<(), ConfigError> {
        config.validate()?;
        fs::create_dir_all(self.root.join("modules")).map_err(|error| ConfigError::Write {
            message: format!("{}: {error}", self.root.display()),
        })?;

        self.write("config.toml", &GlobalFile::from_config(config))?;
        self.write("theme.toml", &ThemeFile::from_config(config))?;
        self.write("keybinds.toml", &KeybindFile::from_config(config))?;
        self.write_path(
            &self.root.join("modules/bar.toml"),
            &BarConfigFile::from_config(config),
        )?;
        self.write_path(
            &self.root.join("modules/notch.toml"),
            &NotchConfigFile::from_config(config),
        )?;
        self.write_path(
            &self.root.join("modules/dock.toml"),
            &DockConfigFile::from_config(config),
        )?;
        self.write_path(
            &self.root.join("modules/launcher.toml"),
            &LauncherConfigFile::from_config(config),
        )?;
        self.write_path(
            &self.root.join("modules/notifications.toml"),
            &NotificationsConfigFile::from_config(config),
        )?;

        Ok(())
    }

    fn read_or_default<T>(&self, relative: &str) -> Result<T, ConfigError>
    where
        T: DeserializeOwned + Default,
    {
        self.read_or_default_from(&self.root, relative)
    }

    fn read_or_default_from<T>(&self, directory: &Path, file: &str) -> Result<T, ConfigError>
    where
        T: DeserializeOwned + Default,
    {
        let path = directory.join(file);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
            Err(error) => {
                return Err(ConfigError::Read {
                    message: format!("{}: {error}", path.display()),
                });
            }
        };

        toml::from_str(&contents).map_err(|error| ConfigError::Parse {
            message: format!("{}: {error}", path.display()),
        })
    }

    fn warn_about_unknown_modules(&self) {
        let modules = self.root.join("modules");
        let entries = match fs::read_dir(&modules) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                tracing::warn!(path = %modules.display(), %error, "could not inspect module configuration directory");
                return;
            }
        };

        const KNOWN: &[&str] = &["bar", "notch", "dock", "launcher", "notifications"];
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if !KNOWN.contains(&stem) {
                tracing::warn!(path = %path.display(), "ignoring unknown module configuration file");
            }
        }
    }

    fn write<T: Serialize>(&self, relative: &str, value: &T) -> Result<(), ConfigError> {
        self.write_path(&self.root.join(relative), value)
    }

    fn write_path<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(value).map_err(|error| ConfigError::Write {
            message: format!("{}: {error}", path.display()),
        })?;
        fs::write(path, contents).map_err(|error| ConfigError::Write {
            message: format!("{}: {error}", path.display()),
        })
    }
}

/// File-backed implementation of the core configuration port.
#[derive(Clone, Debug)]
pub struct FileConfig {
    loader: ConfigLoader,
}

impl FileConfig {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self {
            loader: ConfigLoader::at(root),
        }
    }

    pub fn discover() -> Self {
        Self {
            loader: ConfigLoader::discover(),
        }
    }

    pub fn root(&self) -> &Path {
        self.loader.root()
    }
}

impl ConfigPort for FileConfig {
    fn load(&self) -> Result<ShellConfig, ConfigError> {
        self.loader.load()
    }

    fn save(&self, config: &ShellConfig) -> Result<(), ConfigError> {
        self.loader.save(config)
    }
}

/// Atomically published runtime configuration.
///
/// A reload parses a complete candidate before storing it. Readers always get
/// either the previous snapshot or the new validated snapshot; never a partial
/// merge of TOML files.
#[derive(Clone, Debug)]
pub struct ConfigManager {
    current: Arc<ArcSwap<ShellConfig>>,
    loader: ConfigLoader,
}

impl ConfigManager {
    pub fn new(loader: ConfigLoader) -> Result<Self, ConfigError> {
        let initial = loader.load()?;
        Ok(Self {
            current: Arc::new(ArcSwap::from_pointee(initial)),
            loader,
        })
    }

    pub fn snapshot(&self) -> Arc<ShellConfig> {
        self.current.load_full()
    }

    pub fn loader(&self) -> &ConfigLoader {
        &self.loader
    }

    /// Parses and validates a complete candidate, then atomically publishes it.
    pub fn reload(&self) -> Result<Arc<ShellConfig>, ConfigError> {
        let candidate = Arc::new(self.loader.load_candidate()?);
        self.current.store(candidate.clone());
        Ok(candidate)
    }

    /// Starts a debounced recursive watcher for the complete configuration
    /// directory. The watcher callback only reloads and sends an event; it
    /// never mutates UI state.
    pub fn watch(&self) -> Result<ConfigWatcher, ConfigError> {
        fs::create_dir_all(self.loader.root()).map_err(|error| ConfigError::Write {
            message: format!("{}: {error}", self.loader.root().display()),
        })?;

        let (sender, receiver) = mpsc::channel();
        let manager = self.clone();
        let callback = move |result: DebounceEventResult| {
            let started = Instant::now();
            match result {
                Ok(events) => {
                    let paths = events
                        .into_iter()
                        .filter(|event| is_relevant_event(&event.event.kind))
                        .flat_map(|event| event.event.paths)
                        .collect::<Vec<_>>();
                    if paths.is_empty() {
                        return;
                    }

                    match manager.reload() {
                        Ok(snapshot) => {
                            let duration_ms = started.elapsed().as_millis();
                            tracing::info!(
                                event = "config_reload",
                                result = "success",
                                duration_ms,
                                paths = ?paths,
                                "configuration snapshot replaced"
                            );
                            let _ = sender.send(ConfigEvent::Changed(ConfigChanged {
                                snapshot,
                                paths,
                                duration_ms,
                            }));
                        }
                        Err(error) => {
                            let duration_ms = started.elapsed().as_millis();
                            tracing::warn!(
                                event = "config_reload",
                                result = "rejected",
                                duration_ms,
                                paths = ?paths,
                                %error,
                                "invalid configuration reload; keeping previous snapshot"
                            );
                            let _ = sender.send(ConfigEvent::Rejected(ConfigReloadRejected {
                                paths,
                                error,
                                duration_ms,
                            }));
                        }
                    }
                }
                Err(errors) => {
                    let error = ConfigError::Read {
                        message: errors
                            .into_iter()
                            .map(|error| error.to_string())
                            .collect::<Vec<_>>()
                            .join("; "),
                    };
                    tracing::warn!(
                        event = "config_reload",
                        result = "rejected",
                        %error,
                        "configuration filesystem events could not be debounced"
                    );
                    let _ = sender.send(ConfigEvent::Rejected(ConfigReloadRejected {
                        paths: Vec::new(),
                        error,
                        duration_ms: started.elapsed().as_millis(),
                    }));
                }
            }
        };

        let mut debouncer =
            new_debouncer(Duration::from_millis(200), None, callback).map_err(|error| {
                ConfigError::Unavailable {
                    message: format!("could not create configuration watcher: {error}"),
                }
            })?;
        debouncer
            .watch(self.loader.root(), RecursiveMode::Recursive)
            .map_err(|error| ConfigError::Unavailable {
                message: format!("could not watch {}: {error}", self.loader.root().display()),
            })?;

        Ok(ConfigWatcher {
            _debouncer: debouncer,
            receiver,
        })
    }
}

impl ConfigPort for ConfigManager {
    fn load(&self) -> Result<ShellConfig, ConfigError> {
        Ok((*self.snapshot()).clone())
    }

    fn save(&self, config: &ShellConfig) -> Result<(), ConfigError> {
        self.loader.save(config)?;
        self.current.store(Arc::new(config.clone()));
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ConfigChanged {
    pub snapshot: Arc<ShellConfig>,
    pub paths: Vec<PathBuf>,
    pub duration_ms: u128,
}

#[derive(Clone, Debug)]
pub struct ConfigReloadRejected {
    pub paths: Vec<PathBuf>,
    pub error: ConfigError,
    pub duration_ms: u128,
}

#[derive(Clone, Debug)]
pub enum ConfigEvent {
    Changed(ConfigChanged),
    Rejected(ConfigReloadRejected),
}

/// Receives events produced by a background filesystem watcher.
pub struct ConfigWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
    receiver: mpsc::Receiver<ConfigEvent>,
}

fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

impl ConfigWatcher {
    pub fn try_recv(&self) -> Result<Option<ConfigEvent>, mpsc::TryRecvError> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<ConfigEvent, mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GlobalFile {
    #[serde(default = "default_schema_version")]
    version: u32,
    #[serde(default)]
    general: GeneralConfig,
    #[serde(default)]
    compositor: CompositorConfig,
    #[serde(default)]
    modules: ModuleRegistry,
}

impl Default for GlobalFile {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            general: GeneralConfig::default(),
            compositor: CompositorConfig::default(),
            modules: ModuleRegistry::default(),
        }
    }
}

impl GlobalFile {
    fn from_config(config: &ShellConfig) -> Self {
        Self {
            version: config.schema_version,
            general: config.general.clone(),
            compositor: config.compositor.clone(),
            modules: config.modules.clone(),
        }
    }
}

fn default_schema_version() -> u32 {
    CURRENT_CONFIG_VERSION
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ThemeFile {
    #[serde(default)]
    colors: shell_core::ColorConfig,
    #[serde(default)]
    radius: shell_core::ThemeScale,
    #[serde(default)]
    spacing: shell_core::ThemeScale,
    #[serde(default)]
    motion: shell_core::MotionConfig,
    #[serde(default)]
    typography: shell_core::TypographyConfig,
}

impl ThemeFile {
    fn into_config(self) -> ThemeConfig {
        ThemeConfig {
            colors: self.colors,
            radius: self.radius,
            spacing: self.spacing,
            motion: self.motion,
            typography: self.typography,
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            colors: config.theme.colors.clone(),
            radius: config.theme.radius,
            spacing: config.theme.spacing,
            motion: config.theme.motion,
            typography: config.theme.typography,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct KeybindFile {
    #[serde(default)]
    binding: Vec<shell_core::KeyBinding>,
}

impl KeybindFile {
    fn into_config(self) -> KeybindConfig {
        KeybindConfig {
            bindings: self.binding,
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            binding: config.keybinds.bindings.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BarConfigFile {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    position: shell_core::BarPosition,
    #[serde(default = "default_bar_height")]
    height: u32,
    #[serde(default = "default_true")]
    reserve_space: bool,
    #[serde(default)]
    keyboard: shell_core::KeyboardModeConfig,
    #[serde(default = "default_bar_modules_left")]
    modules_left: Vec<String>,
    #[serde(default = "default_bar_modules_center")]
    modules_center: Vec<String>,
    #[serde(default = "default_bar_modules_right")]
    modules_right: Vec<String>,
}

impl Default for BarConfigFile {
    fn default() -> Self {
        let config = BarConfig::default();
        Self {
            enabled: config.enabled,
            position: config.position,
            height: config.height,
            reserve_space: config.reserve_space,
            keyboard: config.keyboard,
            modules_left: config.modules_left,
            modules_center: config.modules_center,
            modules_right: config.modules_right,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_bar_height() -> u32 {
    BarConfig::default().height
}

fn default_bar_modules_left() -> Vec<String> {
    BarConfig::default().modules_left
}

fn default_bar_modules_center() -> Vec<String> {
    BarConfig::default().modules_center
}

fn default_bar_modules_right() -> Vec<String> {
    BarConfig::default().modules_right
}

impl BarConfigFile {
    fn into_config(self) -> BarConfig {
        let defaults = BarConfig::default();
        BarConfig {
            enabled: self.enabled,
            position: self.position,
            height: if self.height == 0 {
                defaults.height
            } else {
                self.height
            },
            reserve_space: self.reserve_space,
            keyboard: self.keyboard,
            modules_left: if self.modules_left.is_empty() {
                defaults.modules_left
            } else {
                self.modules_left
            },
            modules_center: if self.modules_center.is_empty() {
                defaults.modules_center
            } else {
                self.modules_center
            },
            modules_right: if self.modules_right.is_empty() {
                defaults.modules_right
            } else {
                self.modules_right
            },
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            enabled: config.bar.enabled,
            position: config.bar.position,
            height: config.bar.height,
            reserve_space: config.bar.reserve_space,
            keyboard: config.bar.keyboard,
            modules_left: config.bar.modules_left.clone(),
            modules_center: config.bar.modules_center.clone(),
            modules_right: config.bar.modules_right.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct NotchConfigFile {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_notch_modules")]
    modules: Vec<String>,
    #[serde(default = "default_notch_collapsed_width")]
    collapsed_width: u32,
    #[serde(default = "default_notch_width")]
    width: u32,
    #[serde(default = "default_notch_collapsed_height")]
    collapsed_height: u32,
    #[serde(default = "default_notch_expanded_height")]
    expanded_height: u32,
    #[serde(default = "default_notch_corner_radius")]
    corner_radius: u32,
    #[serde(default = "default_notch_corner_size")]
    corner_size: u32,
    #[serde(default)]
    edge: shell_core::BarPosition,
    #[serde(default)]
    animation: shell_core::AnimationConfig,
}

impl Default for NotchConfigFile {
    fn default() -> Self {
        let config = NotchConfig::default();
        Self {
            enabled: config.enabled,
            modules: config.modules.clone(),
            collapsed_width: config.collapsed_width,
            width: config.width,
            collapsed_height: config.collapsed_height,
            expanded_height: config.expanded_height,
            corner_radius: config.corner_radius,
            corner_size: config.corner_size,
            edge: config.edge,
            animation: config.animation,
        }
    }
}

fn default_notch_width() -> u32 {
    NotchConfig::default().width
}

fn default_notch_collapsed_width() -> u32 {
    NotchConfig::default().collapsed_width
}

fn default_notch_modules() -> Vec<String> {
    NotchConfig::default().modules
}

fn default_notch_collapsed_height() -> u32 {
    NotchConfig::default().collapsed_height
}

fn default_notch_expanded_height() -> u32 {
    NotchConfig::default().expanded_height
}

fn default_notch_corner_radius() -> u32 {
    NotchConfig::default().corner_radius
}

fn default_notch_corner_size() -> u32 {
    NotchConfig::default().corner_size
}

impl NotchConfigFile {
    fn into_config(self) -> NotchConfig {
        let defaults = NotchConfig::default();
        NotchConfig {
            enabled: self.enabled,
            modules: if self.modules.is_empty() {
                defaults.modules.clone()
            } else {
                self.modules
            },
            collapsed_width: if self.collapsed_width == 0 {
                defaults.collapsed_width
            } else {
                self.collapsed_width
            },
            width: if self.width == 0 {
                defaults.width
            } else {
                self.width
            },
            collapsed_height: if self.collapsed_height == 0 {
                defaults.collapsed_height
            } else {
                self.collapsed_height
            },
            expanded_height: if self.expanded_height == 0 {
                defaults.expanded_height
            } else {
                self.expanded_height
            },
            corner_radius: if self.corner_radius == 0 {
                defaults.corner_radius
            } else {
                self.corner_radius
            },
            corner_size: if self.corner_size == 0 {
                defaults.corner_size
            } else {
                self.corner_size
            },
            edge: self.edge,
            animation: self.animation,
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            enabled: config.notch.enabled,
            modules: config.notch.modules.clone(),
            collapsed_width: config.notch.collapsed_width,
            width: config.notch.width,
            collapsed_height: config.notch.collapsed_height,
            expanded_height: config.notch.expanded_height,
            corner_radius: config.notch.corner_radius,
            corner_size: config.notch.corner_size,
            edge: config.notch.edge,
            animation: config.notch.animation.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DockConfigFile {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_dock_position")]
    position: shell_core::BarPosition,
    #[serde(default = "default_true")]
    auto_hide: bool,
    #[serde(default = "default_dock_icon_size")]
    icon_size: u32,
}

impl Default for DockConfigFile {
    fn default() -> Self {
        let config = DockConfig::default();
        Self {
            enabled: config.enabled,
            position: config.position,
            auto_hide: config.auto_hide,
            icon_size: config.icon_size,
        }
    }
}

fn default_dock_icon_size() -> u32 {
    DockConfig::default().icon_size
}

fn default_dock_position() -> shell_core::BarPosition {
    shell_core::BarPosition::Bottom
}

impl DockConfigFile {
    fn into_config(self) -> DockConfig {
        let defaults = DockConfig::default();
        DockConfig {
            enabled: self.enabled,
            position: self.position,
            auto_hide: self.auto_hide,
            icon_size: if self.icon_size == 0 {
                defaults.icon_size
            } else {
                self.icon_size
            },
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            enabled: config.dock.enabled,
            position: config.dock.position,
            auto_hide: config.dock.auto_hide,
            icon_size: config.dock.icon_size,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LauncherConfigFile {
    #[serde(default = "default_true")]
    enabled: bool,
}

impl Default for LauncherConfigFile {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl LauncherConfigFile {
    fn into_config(self) -> LauncherConfig {
        LauncherConfig {
            enabled: self.enabled,
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            enabled: config.launcher.enabled,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct NotificationsConfigFile {
    #[serde(default = "default_true")]
    enabled: bool,
}

impl Default for NotificationsConfigFile {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl NotificationsConfigFile {
    fn into_config(self) -> NotificationsConfig {
        NotificationsConfig {
            enabled: self.enabled,
        }
    }

    fn from_config(config: &ShellConfig) -> Self {
        Self {
            enabled: config.notifications.enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use shell_core::{ConfigPort, ShellConfig};
    use tempfile::tempdir;

    use super::{ConfigEvent, ConfigLoader, ConfigManager, FileConfig};

    #[test]
    fn missing_files_use_safe_defaults() {
        let directory = tempdir().expect("temporary config directory");
        let config = ConfigLoader::at(directory.path())
            .load()
            .expect("missing files are valid");

        assert_eq!(config, ShellConfig::default());
    }

    #[test]
    fn modular_files_are_merged_into_one_model() {
        let directory = tempdir().expect("temporary config directory");
        fs::create_dir_all(directory.path().join("modules")).expect("module directory");
        fs::write(
            directory.path().join("config.toml"),
            "version = 1\n\n[general]\nanimations = false\nstartup_delay_ms = 25\n\n[modules]\ndock = true\n",
        )
        .expect("global config");
        fs::write(
            directory.path().join("theme.toml"),
            "[colors]\nbackground = \"#112233\"\nforeground = \"#F0F0F0\"\n",
        )
        .expect("theme config");
        fs::write(
            directory.path().join("modules/bar.toml"),
            "enabled = true\nposition = \"bottom\"\nheight = 48\n",
        )
        .expect("bar config");

        let config = ConfigLoader::at(directory.path())
            .load()
            .expect("configuration should load");

        assert!(!config.general.animations);
        assert_eq!(config.general.startup_delay_ms, 25);
        assert!(config.modules.dock);
        assert_eq!(config.theme.colors.background, "#112233");
        assert_eq!(config.bar.height, 48);
        assert_eq!(config.bar.position, shell_core::BarPosition::Bottom);
    }

    #[test]
    fn invalid_files_fall_back_without_failing_startup() {
        let directory = tempdir().expect("temporary config directory");
        fs::write(directory.path().join("theme.toml"), "[colors\n").expect("invalid theme");

        let config = ConfigLoader::at(directory.path())
            .load()
            .expect("invalid optional file should not fail startup");

        assert_eq!(config, ShellConfig::default());
    }

    #[test]
    fn file_config_can_persist_the_validated_model() {
        let directory = tempdir().expect("temporary config directory");
        let store = FileConfig::at(directory.path());
        let config = ShellConfig::default();

        store.save(&config).expect("configuration should save");

        assert_eq!(store.load().expect("configuration should load"), config);
        assert!(directory.path().join("modules/bar.toml").exists());
    }

    #[test]
    fn rejected_reload_preserves_previous_snapshot() {
        let directory = tempdir().expect("temporary config directory");
        let manager = ConfigManager::new(ConfigLoader::at(directory.path()))
            .expect("initial configuration should load");

        fs::write(directory.path().join("theme.toml"), "[colors\n").expect("invalid theme");
        assert!(manager.reload().is_err());
        assert_eq!(*manager.snapshot(), ShellConfig::default());

        fs::write(
            directory.path().join("theme.toml"),
            "[colors]\nbackground = \"#112233\"\nforeground = \"#F0F0F0\"\n",
        )
        .expect("valid theme");
        manager
            .reload()
            .expect("valid candidate should replace snapshot");
        assert_eq!(manager.snapshot().theme.colors.background, "#112233");
    }

    #[test]
    fn watcher_debounces_and_publishes_a_valid_snapshot() {
        let directory = tempdir().expect("temporary config directory");
        let manager = ConfigManager::new(ConfigLoader::at(directory.path()))
            .expect("initial configuration should load");
        let watcher = manager.watch().expect("watcher should start");

        fs::write(
            directory.path().join("theme.toml"),
            "[colors]\nbackground = \"#223344\"\nforeground = \"#F0F0F0\"\n",
        )
        .expect("valid theme");

        let event = watcher
            .recv_timeout(Duration::from_secs(3))
            .expect("watcher should publish a reload event");
        match event {
            ConfigEvent::Changed(change) => {
                assert_eq!(change.snapshot.theme.colors.background, "#223344");
                assert!(!change.paths.is_empty());
            }
            ConfigEvent::Rejected(rejected) => {
                panic!("valid configuration was rejected: {}", rejected.error);
            }
        }

        assert!(matches!(
            watcher.recv_timeout(Duration::from_millis(500)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
    }
}
