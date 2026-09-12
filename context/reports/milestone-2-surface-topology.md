# Milestone 2 — Surface topology and multi-monitor implementation

## Implemented

The platform layer now owns an output registry with explicit per-output state:

```rust
OutputState {
    id,
    name,
    geometry,
    scale,
    surfaces,
}
```

`OutputRegistry` reconciles compositor snapshots and emits transitions for:

- output added;
- output geometry/name/scale changed;
- output removed, including its owned surfaces;
- focused output changed.

Invalid geometry, invalid or non-finite scale, duplicate outputs and unknown
focus targets are rejected before surface state is created or changed.

Surface ownership is represented by `SurfaceOwner` and supports the two ADR-005
experiments:

- `Unified`: one fullscreen surface per output;
- `Independent`: panel, notch and overlay surfaces per output.

Logical/physical point conversion is output-local and keeps fractional scale
explicit. Automated tests cover scale `1.25`, add/change/remove, focus
fallback, surface ownership and invalid output data.

## Live experiment

The `surface-topology-poc` binary creates transparent SHM layer-shell surfaces
for every advertised output and reacts to output hotplug events. Run it inside
the Hyprland session with:

```text
cargo run --bin surface-topology-poc -- --topology=unified
cargo run --bin surface-topology-poc -- --topology=independent
```

Optional behavior switches:

```text
--click-through
--no-exclusive-zone
```

The PoC now starts with empty input regions and applies bounded hitboxes after
the layer configure event. Without `--click-through`, panel and notch surfaces
receive input only inside their own surfaces, while fullscreen `Overlay` and
the idle portion of `Unified` remain click-through. With `--click-through`, all
surfaces use empty input regions.

The existing `layer-shell-poc` remains the focused keyboard/pointer and
single-surface diagnostic tool.

## Decision status

`Independent` is the runtime default in `ShellApplication` because each shell
feature receives explicit output ownership and its own input/layer lifecycle.
ADR-005 remains **Proposed** until the two live experiments are run under
Hyprland with at least two outputs and the G03 matrix is recorded. This
environment has no active Wayland compositor, so claiming a measured G03 pass
would be incorrect.

## Validation

The following gates pass locally:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## G03/G04 gate review

The static and automated portion of the validation passed:

- `cargo fmt --all --check` — exit `0`;
- `cargo check --workspace --all-targets` — exit `0`;
- `cargo clippy --workspace --all-targets -- -D warnings` — exit `0`;
- `cargo test --workspace` — exit `0`, including the topology, fractional-scale,
  focus and buffer-lifecycle tests.

The live experiments were also attempted with both architectures:

```text
cargo run --quiet --bin surface-topology-poc -- --topology=unified
cargo run --quiet --bin surface-topology-poc -- --topology=independent
```

Both stopped before creating a Wayland surface with:
`Could not find wayland compositor`.

Subsequent evidence supplied from a live Hyprland session changes that result
from "no runtime evidence" to "partial runtime evidence":

- `Unified` started successfully and created `HDMI-A-1` with role `Unified`;
- `Independent` started successfully and created `HDMI-A-1` with roles
  `Panel`, `Notch` and `Overlay`;
- both runs used `click_through=false` and `exclusive_zone=true` and remained
  running without the earlier buffer-pool/protocol error.

The screenshots show one output only. They prove initial surface creation, but
do not prove the full gate matrix or multi-output behavior.

Therefore the gate status is deliberately not marked as PASS:

- **G03 — partially validated at runtime.** Initial creation works for both
  architectures on `HDMI-A-1`, but the mandatory Hyprland matrix
  (click-through, dynamic input regions, click-outside, keyboard focus,
  resize/animation, exclusive zone, fullscreen, popup layering and
  multi-monitor behavior) still needs evidence from a live session.
- **G04 — partially validated at runtime.** Initial per-role creation works
  for one output, while a live session with at least two outputs is still
  required to verify independent surface creation/destruction, output
  removal/addition, focused-output transitions, fractional-scale
  rendering/hit testing and no-restart recovery.

The hard-failure checks were not triggered by the offline tests, but they are
not a substitute for the required compositor evidence. ADR-005 remains
**Proposed** until these runtime checks are completed and recorded.
