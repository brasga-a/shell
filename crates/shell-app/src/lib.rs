//! Composition-root support for the shell executable.

use std::{
    sync::{Arc, OnceLock, mpsc},
    thread,
};

use shell_core::{
    CompositorError, CompositorEvent, CompositorEventStream, CompositorPort, CompositorSnapshot,
    CompositorStateStore, ConfigPort, Output, PlatformError, ShellCommand, ShellConfig,
};
use shell_linux::LinuxServices;
use shell_platform::{
    OutputRegistry, OutputStateError, OutputTransition, SurfaceMetrics, SurfaceTopology,
};
use shell_theme::DesignTokens;
use shell_ui_gpui::{GpuiFrontend, ModuleRegistry};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

static LOGGING_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Compose the initial statically linked module set.
///
/// This is intentionally in `shell-app`: the UI host knows only the contract
/// and the application is the sole place that knows concrete module crates.
pub fn build_module_registry() -> Result<ModuleRegistry, PlatformError> {
    let mut registry = ModuleRegistry::new();
    registry
        .register(shell_module_launcher::LauncherModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    registry
        .register(shell_module_calendar::CalendarModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    registry
        .register(shell_module_player::PlayerModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    registry
        .register(shell_module_theme::ThemeModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    registry
        .register(shell_module_resources::ResourcesModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    registry
        .register(shell_module_clock::ClockModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    registry
        .register(shell_module_settings::SettingsModule)
        .map_err(|error| PlatformError::initialization(error.to_string()))?;
    Ok(registry)
}

/// Application-owned asynchronous command path used by feature surfaces.
///
/// The UI only enqueues a domain command. The worker invokes the
/// `CompositorPort`, keeping IPC latency and compositor failures off the GPUI
/// event/render path.
#[derive(Clone)]
pub struct ShellCommandBus {
    sender: mpsc::Sender<ShellCommand>,
}

impl ShellCommandBus {
    fn new(compositor: Arc<dyn CompositorPort>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let _ = thread::Builder::new()
            .name("shell-command-worker".to_owned())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    if let Err(error) = execute_command(compositor.as_ref(), command) {
                        tracing::warn!(%error, ?command, "shell command failed");
                    }
                }
            });
        Self { sender }
    }

    pub fn dispatch(&self, command: ShellCommand) -> Result<(), PlatformError> {
        self.sender
            .send(command)
            .map_err(|error| PlatformError::Runtime {
                message: format!("shell command queue is unavailable: {error}"),
            })
    }

    pub fn sender(&self) -> mpsc::Sender<ShellCommand> {
        self.sender.clone()
    }
}

fn execute_command(
    compositor: &dyn CompositorPort,
    command: ShellCommand,
) -> Result<(), PlatformError> {
    match command {
        ShellCommand::FocusWorkspace(workspace_id) => compositor
            .focus_workspace(workspace_id)
            .map_err(|error| PlatformError::Runtime {
                message: error.to_string(),
            }),
    }
}

/// Initialize structured logging once for the process.
pub fn initialize_logging() -> Result<(), PlatformError> {
    if LOGGING_INITIALIZED.get().is_some() {
        return Ok(());
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()
        .map_err(|error| PlatformError::initialization(error.to_string()))?;

    let _ = LOGGING_INITIALIZED.set(());
    Ok(())
}

pub struct ShellApplication {
    compositor: Arc<dyn CompositorPort>,
    config: Box<dyn ConfigPort>,
    linux_services: LinuxServices,
    frontend: GpuiFrontend,
    design_tokens: DesignTokens,
    output_registry: OutputRegistry,
    compositor_state: CompositorStateStore,
    command_bus: ShellCommandBus,
}

impl ShellApplication {
    pub fn new(
        compositor: Box<dyn CompositorPort>,
        config: Box<dyn ConfigPort>,
        linux_services: LinuxServices,
        frontend: GpuiFrontend,
        design_tokens: DesignTokens,
    ) -> Self {
        let compositor: Arc<dyn CompositorPort> = Arc::from(compositor);
        let command_bus = ShellCommandBus::new(compositor.clone());
        Self {
            compositor,
            config,
            linux_services,
            frontend,
            design_tokens,
            output_registry: OutputRegistry::new(SurfaceTopology::Independent),
            compositor_state: CompositorStateStore::default(),
            command_bus,
        }
    }

    /// Initialize application-owned state without assuming a live compositor.
    pub fn start(&mut self) -> Result<(), PlatformError> {
        let config = self
            .config
            .load()
            .map_err(|error| PlatformError::initialization(error.to_string()))?;

        self.apply_config(&config)?;
        let transitions = self.refresh_compositor_state()?;

        tracing::info!(
            component = "shell-app",
            config_schema = config.schema_version,
            outputs = self.output_registry.outputs().len(),
            output_transitions = transitions.len(),
            "shell composition root initialized"
        );

        Ok(())
    }

    /// Applies one already validated configuration snapshot to application-owned
    /// state. Callers should obtain snapshots from `ConfigManager`, never from
    /// individual TOML files.
    pub fn apply_config(&mut self, config: &ShellConfig) -> Result<(), PlatformError> {
        config
            .validate()
            .map_err(|error| PlatformError::initialization(error.to_string()))?;
        self.design_tokens = DesignTokens::from_config(&config.theme);
        self.frontend.prepare(config);
        self.output_registry
            .set_surface_metrics(SurfaceMetrics::new(
                config.bar.height as f32,
                config.notch.width as f32,
                config.notch.collapsed_height as f32,
            ))
            .map_err(output_state_error)?;
        let _ = &self.linux_services;
        Ok(())
    }

    /// Refreshes output ownership from the compositor without making a
    /// temporarily unavailable IPC connection fatal during startup.
    pub fn refresh_outputs(&mut self) -> Result<Vec<OutputTransition>, PlatformError> {
        match self.compositor.outputs() {
            Ok(outputs) => self.apply_outputs(outputs),
            Err(CompositorError::Unavailable { message })
            | Err(CompositorError::Connection { message }) => {
                tracing::warn!(%message, "compositor output discovery is unavailable");
                Ok(Vec::new())
            }
            Err(error) => Err(PlatformError::initialization(error.to_string())),
        }
    }

    /// Refreshes the complete compositor state when the adapter supports it,
    /// while retaining output-only startup for simpler adapters.
    pub fn refresh_compositor_state(&mut self) -> Result<Vec<OutputTransition>, PlatformError> {
        match self.compositor.snapshot() {
            Ok(snapshot) => self.apply_compositor_snapshot(snapshot),
            Err(CompositorError::Unavailable { message })
            | Err(CompositorError::Connection { message })
            | Err(CompositorError::Unsupported { operation: message }) => {
                tracing::warn!(%message, "compositor snapshot is unavailable; falling back to outputs");
                self.refresh_outputs()
            }
            Err(error) => Err(PlatformError::initialization(error.to_string())),
        }
    }

    pub fn apply_compositor_event(
        &mut self,
        event: CompositorEvent,
    ) -> Result<Vec<OutputTransition>, PlatformError> {
        self.apply_compositor_snapshot(event.snapshot)
    }

    pub fn compositor_state(&self) -> Option<CompositorSnapshot> {
        self.compositor_state.snapshot()
    }

    pub fn state_store(&self) -> CompositorStateStore {
        self.compositor_state.clone()
    }

    pub fn command_bus(&self) -> ShellCommandBus {
        self.command_bus.clone()
    }

    pub fn dispatch_command(&self, command: ShellCommand) -> Result<(), PlatformError> {
        execute_command(self.compositor.as_ref(), command)
    }

    pub fn subscribe_compositor_events(
        &self,
    ) -> Result<Box<dyn CompositorEventStream>, CompositorError> {
        self.compositor.subscribe_events()
    }

    pub fn output_registry(&self) -> &OutputRegistry {
        &self.output_registry
    }

    /// Applies an output snapshot or a translated hotplug event to the
    /// output-owned surface state.
    pub fn apply_outputs(
        &mut self,
        outputs: impl IntoIterator<Item = Output>,
    ) -> Result<Vec<OutputTransition>, PlatformError> {
        self.output_registry
            .reconcile(outputs)
            .map_err(output_state_error)
    }

    fn apply_compositor_snapshot(
        &mut self,
        snapshot: CompositorSnapshot,
    ) -> Result<Vec<OutputTransition>, PlatformError> {
        let transitions = self
            .output_registry
            .reconcile_with_focus(snapshot.monitors.clone(), snapshot.focused_output)
            .map_err(output_state_error)?;
        self.compositor_state.replace(snapshot);
        Ok(transitions)
    }
}

fn output_state_error(error: OutputStateError) -> PlatformError {
    PlatformError::initialization(format!("output state reconciliation failed: {error}"))
}

#[cfg(test)]
mod tests {
    use shell_config::InMemoryConfig;
    use shell_core::{
        CompositorError, CompositorPort, CompositorSnapshot, Output, OutputId, Rect, ShellCommand,
        Window, Workspace, WorkspaceId,
    };
    use shell_linux::LinuxServices;
    use shell_platform::{Anchors, ShellLayer, SurfaceSpec};
    use shell_theme::DesignTokens;
    use shell_ui_gpui::GpuiFrontend;

    use super::ShellApplication;

    struct SnapshotCompositor {
        snapshot: CompositorSnapshot,
    }

    impl CompositorPort for SnapshotCompositor {
        fn outputs(&self) -> Result<Vec<Output>, CompositorError> {
            Ok(self.snapshot.monitors.clone())
        }

        fn workspaces(&self) -> Result<Vec<Workspace>, CompositorError> {
            Ok(self.snapshot.workspaces.clone())
        }

        fn windows(&self) -> Result<Vec<Window>, CompositorError> {
            Ok(self.snapshot.windows.clone())
        }

        fn focus_workspace(&self, _workspace_id: WorkspaceId) -> Result<(), CompositorError> {
            Ok(())
        }

        fn snapshot(&self) -> Result<CompositorSnapshot, CompositorError> {
            Ok(self.snapshot.clone())
        }
    }

    fn output(id: u64, x: f32) -> Output {
        Output::new(
            OutputId::new(id),
            format!("output-{id}"),
            Rect::from_xywh(x, 0.0, 1920.0, 1080.0),
            1.0,
        )
    }

    fn application(snapshot: CompositorSnapshot) -> ShellApplication {
        ShellApplication::new(
            Box::new(SnapshotCompositor { snapshot }),
            Box::new(InMemoryConfig::default()),
            LinuxServices,
            GpuiFrontend::new(SurfaceSpec::new(
                OutputId::new(1),
                ShellLayer::Top,
                Anchors::TOP,
            )),
            DesignTokens::default(),
        )
    }

    #[test]
    fn applies_snapshot_focused_output_atomically_to_registry() {
        let snapshot = CompositorSnapshot {
            monitors: vec![output(1, 0.0), output(2, 1920.0)],
            focused_output: Some(OutputId::new(2)),
            ..CompositorSnapshot::default()
        };
        let mut application = application(snapshot);

        application.start().expect("snapshot is valid");

        assert_eq!(
            application.output_registry().focused_output(),
            Some(OutputId::new(2))
        );
        assert_eq!(
            application.compositor_state().unwrap().focused_output,
            Some(OutputId::new(2))
        );
    }

    #[test]
    fn canonical_snapshot_can_explicitly_clear_focus() {
        let snapshot = CompositorSnapshot {
            monitors: vec![output(1, 0.0)],
            focused_output: Some(OutputId::new(1)),
            ..CompositorSnapshot::default()
        };
        let mut application = application(snapshot);
        application.start().expect("snapshot is valid");

        let transitions = application
            .apply_compositor_event(shell_core::CompositorEvent {
                kind: shell_core::CompositorEventKind::Focus,
                snapshot: CompositorSnapshot {
                    monitors: vec![output(1, 0.0)],
                    focused_output: None,
                    ..CompositorSnapshot::default()
                },
            })
            .expect("focus update is valid");

        assert!(transitions.iter().any(|transition| matches!(
            transition,
            shell_platform::OutputTransition::Focused {
                previous: Some(previous),
                current: None,
            } if *previous == OutputId::new(1)
        )));
        assert_eq!(application.output_registry().focused_output(), None);
    }

    #[test]
    fn workspace_intent_reaches_the_application_command_boundary() {
        let snapshot = CompositorSnapshot::default();
        let application = application(snapshot);

        application
            .dispatch_command(ShellCommand::FocusWorkspace(WorkspaceId::new(2)))
            .expect("the compositor port accepts the command");
    }
}
