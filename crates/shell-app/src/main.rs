use shell_app::{ShellApplication, initialize_logging};
use shell_config::{ConfigLoader, ConfigManager};
use shell_hyprland::HyprlandCompositor;
use shell_linux::LinuxServices;
use shell_platform::{Anchors, ShellLayer, SurfaceSpec};
use shell_theme::DesignTokens;
use shell_ui_gpui::GpuiFrontend;

fn main() {
    let run_gpui_viability = std::env::args().any(|argument| argument == "--gpui-viability");

    if let Err(error) = initialize_logging() {
        eprintln!("failed to initialize logging: {error}");
        std::process::exit(1);
    }

    let frontend = GpuiFrontend::new(SurfaceSpec::new(
        shell_core::OutputId::new(0),
        ShellLayer::Top,
        Anchors::TOP_LEFT_RIGHT,
    ));
    let config_manager = match ConfigManager::new(ConfigLoader::discover()) {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(error = %error, "shell configuration could not be initialized");
            std::process::exit(1);
        }
    };
    let mut application = ShellApplication::new(
        Box::new(HyprlandCompositor::default()),
        Box::new(config_manager.clone()),
        LinuxServices,
        frontend.clone(),
        DesignTokens::default(),
    );

    if let Err(error) = application.start() {
        tracing::error!(error = %error, "shell failed to initialize");
        std::process::exit(1);
    }

    if run_gpui_viability {
        let watcher = match config_manager.watch() {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                tracing::warn!(error = %error, "configuration hot reload is unavailable");
                None
            }
        };
        let result = watcher.map_or_else(
            || frontend.run_viability_app(),
            |watcher| frontend.run_viability_app_with_watcher(watcher),
        );
        if let Err(error) = result {
            tracing::error!(error = %error, "GPUI viability app failed");
            std::process::exit(1);
        }
    } else {
        tracing::info!(
            "shell initialized without opening the GPUI viability window; use --gpui-viability to run it"
        );
    }
}
