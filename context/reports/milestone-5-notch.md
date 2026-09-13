# Milestone 5 — Notch geometry, animation and focus

## Implemented

The shell now owns a renderer-independent notch model in `shell-core`:

- `NotchState::{Idle, Launcher}` is semantic state, independent from GPUI;
- `NotchGeometry` models width, height, radius, concave corner size and edge;
- `NotchAnimation` interpolates width and height with smoothstep easing and can
  be retargeted from its current frame without resetting the semantic state;
- `FocusManager` centralizes notch keyboard ownership, modal pointer ownership,
  click-outside dismissal and Escape dismissal.

The GPUI frontend creates one dedicated fullscreen layer-shell surface for the
notch. The surface is only an input/rendering coordinator; there is no separate
full-width bar. GPUI receives an explicit logical output size from the focused
compositor output, preventing the anchored layer's requested `0x0` size from
collapsing its internal layout. Its visible body is painted with a GPUI
`PathBuilder` translated to the same centered bounds used by the content. The
body is resized from the animated geometry, so width and height transitions do
not require destroying the Wayland surface.

The layer reserves `notch.collapsed_height` pixels on the top edge using the
same exclusive-zone behavior validated by `layer-shell-poc`. Normal clients
therefore start below the idle notch without introducing a visual bar.

Input behavior is explicit:

- `Idle` installs only the notch rectangles as the Wayland input region;
- `Launcher` expands the region to the fullscreen surface so the coordinator
  can receive one outside click and dismiss the modal state;
- dismissing calls GPUI `blur`, and the next frame restores the notch-only
  region, allowing applications below the transparent surface to receive
  pointer input again.

The idle body is composed from the selected `notch.modules` list. The clock is
rendered inside the notch, and the collapsed width is recalculated from the
known selected modules while the launcher uses the configured expanded width.

The notch-specific values are configurable in `config/modules/notch.toml`:
`modules`, `collapsed_width`, `width`, both heights, `corner_radius`,
`corner_size`, `edge` and animation settings.

## Validation

Passed locally:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
ATLANTIC_CONFIG_DIR=./config cargo run --quiet --bin shell-app -- --no-ui
```

Unit tests cover geometry bounds and transparent corner regions, top/bottom
edge mapping, interruptible retargeting, and focus dismissal behavior.

## Runtime gate evidence

Runtime geometry was also checked in the Hyprland session. The final layer
topology was:

```text
linux-shell-notch         x=0    y=0   w=2560 h=1080
linux-shell-debug-overlay x=2264 y=910 w=280  h=154
```

The rendered idle body was measured at `x=1208`, `y=0`, `w=144`, `h=32` on the
2560x1080 output, with a 32-pixel top exclusive zone. The stabilized screenshot
confirmed one centered body with the clock inside it.

There was no `linux-shell-panel` layer. G07/G08 still require a manual live
interaction pass for complete visual and pointer evidence. Run:

```text
ATLANTIC_CONFIG_DIR=./config cargo run --bin shell-app -- --panel
```

Then verify: click the collapsed center to open `Launcher`, observe width and
height animation, press `Escape`, click outside while expanded, repeat open /
close rapidly, and click through the transparent top corners and the area
outside the collapsed notch. The expected result is no pointer blocking after
dismissal and no focus retained by the shell.
