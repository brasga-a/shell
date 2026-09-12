# Milestone 3 — Hyprland integration

## Implemented

`shell-hyprland` now owns the Hyprland-specific integration boundary:

- resolves `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE`;
- connects to the command socket for JSON state requests and dispatches;
- translates monitors, workspaces and clients into `shell-core` types;
- exposes focused output, focused workspace, focused window and fullscreen
  state;
- implements workspace and window focus commands;
- listens to `.socket2.sock` and translates relevant events into application
  event categories carrying a coherent `CompositorSnapshot`;
- ignores unsupported event names and skips malformed/disappearing client
  entries with a structured warning;
- maps connection, malformed-data and command failures to typed
  `CompositorError` values.

The core boundary contains no Hyprland JSON, socket paths or event strings.
`shell-app` owns the current translated snapshot and can apply snapshots from
the event stream without exposing adapter payloads to the frontend.

## Diagnostic commands

Run these inside the Hyprland session:

```text
cargo run -p shell-hyprland --bin hyprland-poc
cargo run -p shell-hyprland --bin hyprland-poc -- --events
cargo run --bin shell-app
```

The first command prints the translated monitor/workspace/window state. The
second keeps the event socket open and prints translated event categories as
the compositor changes. The application command initializes the composition
root and logs the number of outputs discovered from the snapshot.

## Validation

Passed locally:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The tests cover socket path resolution, event classification, monitor geometry
and scale translation, workspace/window/focus translation, malformed client
recovery and command response handling.

The live Hyprland socket was not accessible from the restricted validation
environment. Therefore G05 is not marked as PASS here. The diagnostic commands
above are the required runtime evidence for monitor/workspace/window state,
focus commands, event delivery, fullscreen changes and the event listener's
single automatic reconnect attempt after a compositor restart.

## Known scope

The event stream refreshes a full translated snapshot after each relevant
event. This deliberately favors a coherent state boundary over exposing
version-dependent Hyprland event payloads. Snapshot collection retries once
when references are inconsistent and canonicalizes entities that disappear
during the read. A later performance gate can replace that strategy only if
measurement shows it is necessary.
