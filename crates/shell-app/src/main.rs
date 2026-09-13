use std::{env, sync::mpsc, thread, time::Duration};

use shell_app::{ShellApplication, initialize_logging};
use shell_config::{ConfigLoader, ConfigManager};
use shell_core::{AppUsageMetrics, CompositorEvent, CompositorEventStream};
use shell_hyprland::HyprlandCompositor;
use shell_linux::{LinuxServices, UsageSampler};
use shell_platform::{Anchors, ShellLayer, SurfaceSpec};
use shell_theme::DesignTokens;
use shell_ui_gpui::GpuiFrontend;

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    let run_gpui_viability = arguments
        .iter()
        .any(|argument| argument == "--gpui-viability");
    let force_panel = arguments.iter().any(|argument| argument == "--panel");
    let disable_ui = arguments.iter().any(|argument| argument == "--no-ui");

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
    } else if !disable_ui
        && (force_panel || env::var_os("WAYLAND_DISPLAY").is_some())
        && config_manager.snapshot().bar.enabled
    {
        let compositor_events = match application.subscribe_compositor_events() {
            Ok(stream) => Some(spawn_event_forwarder(stream)),
            Err(error) => {
                tracing::warn!(%error, "compositor event stream is unavailable; panel will show its initial state");
                None
            }
        };
        let usage_metrics = Some(spawn_usage_forwarder());
        let watcher = match config_manager.watch() {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                tracing::warn!(error = %error, "configuration hot reload is unavailable");
                None
            }
        };
        let result = frontend.run_panel_app(
            application.compositor_state(),
            compositor_events,
            usage_metrics,
            application.command_bus().sender(),
            application.state_store(),
            watcher,
        );
        if let Err(error) = result {
            tracing::error!(error = %error, "shell panel failed");
            std::process::exit(1);
        }
    } else {
        tracing::info!(
            "shell initialized without opening a GPUI surface; use --panel or run inside Wayland, or use --gpui-viability for the viability check"
        );
    }
}

fn spawn_event_forwarder(
    mut stream: Box<dyn CompositorEventStream>,
) -> mpsc::Receiver<CompositorEvent> {
    let (sender, receiver) = mpsc::channel();
    let _ = thread::Builder::new()
        .name("hyprland-event-forwarder".to_owned())
        .spawn(move || {
            loop {
                match stream.next_event() {
                    Ok(event) => {
                        if sender.send(event).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "compositor event stream stopped");
                        break;
                    }
                }
            }
        });
    receiver
}

fn spawn_usage_forwarder() -> mpsc::Receiver<AppUsageMetrics> {
    let (sender, receiver) = mpsc::channel();
    let _ = thread::Builder::new()
        .name("shell-usage-sampler".to_owned())
        .spawn(move || {
            let mut sampler = UsageSampler::new();
            loop {
                if sender.send(sampler.sample()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
    receiver
}
