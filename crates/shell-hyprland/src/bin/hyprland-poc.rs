use shell_core::CompositorPort;
use shell_hyprland::HyprlandCompositor;

fn main() {
    let listen = std::env::args().any(|argument| argument == "--events");
    let compositor = HyprlandCompositor::default();

    let snapshot = match compositor.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("hyprland-poc failed: {error}");
            std::process::exit(1);
        }
    };

    println!(
        "hyprland-poc: monitors={} workspaces={} windows={} focused_output={:?} focused_workspace={:?} focused_window={:?} fullscreen={:?}",
        snapshot.monitors.len(),
        snapshot.workspaces.len(),
        snapshot.windows.len(),
        snapshot.focused_output,
        snapshot.focused_workspace,
        snapshot.focused_window.as_ref().map(|window| window.id),
        snapshot.fullscreen,
    );

    if !listen {
        return;
    }

    let mut events = match compositor.subscribe_events() {
        Ok(events) => events,
        Err(error) => {
            eprintln!("hyprland-poc event stream failed: {error}");
            std::process::exit(1);
        }
    };
    loop {
        match events.next_event() {
            Ok(event) => println!(
                "hyprland-poc: event={:?} monitors={} workspaces={} windows={}",
                event.kind,
                event.snapshot.monitors.len(),
                event.snapshot.workspaces.len(),
                event.snapshot.windows.len(),
            ),
            Err(error) => {
                eprintln!("hyprland-poc event stream stopped: {error}");
                std::process::exit(1);
            }
        }
    }
}
