# Milestone 5 — Notch geometry, animation and focus

## Implemented

The shell now owns a renderer-independent notch model in `shell-core`:

- `NotchState::{Idle, Launcher}` is semantic state, independent from GPUI;
- `NotchGeometry` models width, height, radius, concave corner size and edge;
- `NotchAnimation` interpolates width and height with smoothstep easing and can
  be retargeted from its current frame without resetting the semantic state;
- `FocusManager` centralizes notch keyboard ownership, modal pointer ownership,
  click-outside dismissal and Escape dismissal.

The GPUI frontend creates a dedicated fullscreen layer-shell surface for the
notch. Its visible body is painted with a GPUI `PathBuilder`: the upper (or
lower) left and right corners are concave, while the opposite corners use the
configured radius. The body is centered and resized from the animated geometry,
so width and height transitions do not require destroying the Wayland surface.

Input behavior is explicit:

- `Idle` installs only the notch rectangles as the Wayland input region;
- `Launcher` expands the region to the fullscreen surface so the coordinator
  can receive one outside click and dismiss the modal state;
- dismissing calls GPUI `blur`, and the next frame restores the notch-only
  region, allowing applications below the transparent surface to receive
  pointer input again.

The notch-specific values are configurable in `config/modules/notch.toml`:
`collapsed_width`, `width`, both heights, `corner_radius`, `corner_size`,
`edge` and animation settings.

## Validation

Passed locally:

```text
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
ATLANTIC_CONFIG_DIR=./config cargo run --quiet --bin shell-app -- --no-ui
```

Unit tests cover geometry bounds and transparent corner regions, top/bottom
edge mapping, interruptible retargeting, and focus dismissal behavior.

## Runtime gate evidence

G07/G08 still require a live Hyprland session for visual and compositor
evidence. Run:

```text
ATLANTIC_CONFIG_DIR=./config cargo run --bin shell-app -- --panel
```

Then verify: click the collapsed center to open `Launcher`, observe width and
height animation, press `Escape`, click outside while expanded, repeat open /
close rapidly, and click through the transparent top corners and the area
outside the collapsed notch. The expected result is no pointer blocking after
dismissal and no focus retained by the shell.
