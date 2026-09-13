# Milestone 4 — First Real Panel

## Implemented

- `shell-theme` now exposes centralized color, spacing, radius, motion and
  typography tokens.
- `shell-core` defines `ShellCommand::FocusWorkspace` and a thread-safe
  compositor snapshot store.
- `shell-app` owns the asynchronous command path. UI intents are queued as
  domain commands and executed by `CompositorPort`; the UI never invokes
  Hyprland or a system CLI directly.
- `shell-app` forwards the Hyprland event stream to the GPUI frontend.
- `shell-ui-gpui` provides a real top panel with workspace buttons from the
  translated compositor snapshot, visible focus styling, a local clock,
  configuration/compositor status text, and token-driven geometry.
- `shell-app` opens the panel by default when running inside Wayland. Use
  `--panel` to force it, `--no-ui` for headless startup and
  `--gpui-viability` for the Milestone 1 proof of concept.

## Validation

Passed in the repository environment:

```text
cargo fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The live panel evidence must be captured inside the target Hyprland session:

```bash
ATLANTIC_CONFIG_DIR=./config cargo run --release --bin shell-app -- --panel
```

The panel should show the compositor workspaces, update the focused workspace
after a Hyprland event and focus a workspace when its button is clicked.

## Performance baseline procedure

Use the release binary in the target session. Record cold startup with the
shell process launched from a clean environment, then record idle RSS and CPU
after the panel has been visible for 30 seconds:

```bash
cargo build --release --bin shell-app
/usr/bin/time -f 'cold_elapsed_s=%e max_rss_kb=%M user_s=%U sys_s=%S' \
  timeout 35s env ATLANTIC_CONFIG_DIR=./config \
  ./target/release/shell-app --panel
```

The current restricted validation environment has no compositor session, so
the runtime numbers are intentionally left for the Hyprland evidence run and
are not presented as measured results here.
