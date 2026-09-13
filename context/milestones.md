# milestones.md

# Project Milestones — Shell Linux Shell

This document defines the execution roadmap for the Rust/Wayland shell.

It complements:

- [README.md](./README.md)
- [module-crate-migration.md](./module-crate-migration.md)
- [modules/README.md](./modules/README.md)
- [decisions.md](./decisions.md)
- [invariants.md](./invariants.md)
- [gates.md](./gates.md)
- [ADRs](./adrs/README.md)

Mandatory gates still control progression. Module sub-milestones use completion criteria when no dedicated gate exists yet.

---

# Module delivery map

The initial module set is:

```text
crates/modules/
├── launcher/
├── calendar/
├── player/
├── theme/
├── resources/
├── clock/
└── settings/
```

Recommended implementation order:

```text
Clock
  ↓
Launcher
  ↓
Linux service baseline
  ↓
Player + Calendar
  ↓
Resources
  ↓
Theme
  ↓
Settings
```

The Notch remains the host/casca. The module crates provide feature content and are registered by `shell-app`.

---

# Milestone 0 — Repository and Architecture Foundation

## Objective

Create the workspace and dependency boundaries before visible shell features.

## Steps

### 0.1 — Create Cargo workspace

Target structure:

```text
crates/
├── shell-core/
├── shell-platform/
├── shell-hyprland/
├── shell-linux/
├── shell-config/
├── shell-theme/
├── shell-ui-gpui/
├── shell-app/
└── modules/
    ├── launcher/
    ├── calendar/
    ├── player/
    ├── theme/
    ├── resources/
    ├── clock/
    └── settings/
```

Module crates may initially contain only minimal library scaffolding until their implementation milestone.

### 0.2 — Define core domain primitives

```text
OutputId
WorkspaceId
WindowId
SurfaceId
ModuleId

Point
Size
Rect

Output
Workspace
Window
```

### 0.3 — Define first application ports

```text
CompositorPort
ConfigPort
```

### 0.4 — Establish error boundaries

```text
CompositorError
ConfigError
PlatformError
```

### 0.5 — Add observability baseline

Use `tracing` and `tracing-subscriber`.

### 0.6 — Add repository checks

```text
cargo check --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace
```

## Deliverables

```text
compiling Cargo workspace
module crate skeletons
architecture-respecting dependency graph
initial core types and ports
structured logging
CI pipeline
```

## Completion criteria

- `shell-core` compiles without GPUI.
- `shell-core` compiles without Hyprland-specific dependencies.
- `shell-app` is the composition root.
- module crates are workspace members.
- no module crate is required by `shell-ui-gpui` as a concrete dependency.

## References

- [Module migration plan](./module-crate-migration.md)
- [Module architecture](./modules/README.md)
- [decisions.md](./decisions.md)
- [invariants.md](./invariants.md)
- [gates.md — G00](./gates.md)
- [ADR-002](./adrs/ADR-002-hexagonal-architecture-for-shell-boundaries.md)
- [ADR-023](./adrs/ADR-023-cargo-workspace-boundaries.md)

## Exit gate

**G00 — Repository Foundation**

---

# Milestone 1 — GPUI + Wayland Viability

## Objective

Prove GPUI can operate as the shell frontend before feature investment.

## Steps

Validate:

```text
startup
rendering
pointer input
keyboard input
transparent surface
Layer::Top
Layer::Overlay
anchors
exclusive zone
specific output targeting
fullscreen interaction
```

Document exactly where GPUI exposes or must be extended for layer-shell, output selection, keyboard interactivity and input regions.

## Deliverables

```text
minimal GPUI Wayland executable
transparent shell surface
layer-shell PoC
documented backend limitations
```

## Completion criteria

The project can create a reliable shell surface under Hyprland.

If this fails, reopen the frontend decision before implementing modules.

## References

- [gates.md — G01, G02](./gates.md)
- [ADR-001](./adrs/ADR-001-gpui-as-the-initial-frontend.md)
- [ADR-004](./adrs/ADR-004-wayland-layer-shell-integration-strategy.md)

## Exit gates

```text
G01 — GPUI Window Viability
G02 — Layer Shell Viability
```

---

# Milestone 2 — Surface Architecture and Multi-Monitor

## Objective

Resolve shell surface topology and per-output lifecycle.

## Steps

Compare:

```text
A. unified transparent surface per output
B. independent panel/notch/overlay layer surfaces
```

Validate:

```text
click-through
input regions
keyboard focus
click-outside
fullscreen windows
output hotplug
output removal
fractional scaling
```

Resolve ADR-005 based on measured behavior and implement explicit `OutputState`.

## Deliverables

```text
selected surface topology
multi-monitor state model
output lifecycle manager
fractional scaling validation
```

## References

- [gates.md — G03, G04](./gates.md)
- [ADR-005](./adrs/ADR-005-shell-surface-topology.md)
- [ADR-006](./adrs/ADR-006-multi-monitor-state-and-surface-ownership.md)

## Exit gates

```text
G03 — Surface Topology
G04 — Multi-Monitor Correctness
```

---

# Milestone 3 — Hyprland Integration

## Objective

Establish Hyprland as the first compositor adapter.

## Steps

Implement:

```text
IPC client
event socket
monitor/workspace/window domain models
CompositorPort
capability model
reconnect/error handling
```

Raw Hyprland payloads terminate inside `shell-hyprland`.

## Completion criteria

The UI and modules consume compositor state without knowing raw Hyprland payloads.

## References

- [gates.md — G05](./gates.md)
- [ADR-007](./adrs/ADR-007-hyprland-as-a-compositor-adapter.md)
- [ADR-008](./adrs/ADR-008-compositor-port-and-capability-model.md)

## Exit gate

**G05 — Hyprland Adapter**

---

# Milestone 4 — First Real Panel

## Objective

Create the first useful shell surface using real compositor state.

## Steps

Implement:

```text
panel container
workspace selector
basic design tokens
initial performance baseline
```

The panel may show clock information temporarily, but the canonical interactive Clock feature will become `shell-module-clock` in Milestone 5A.

## Completion criteria

The panel uses real event-driven state and never accesses Hyprland directly.

## References

- [gates.md — G06](./gates.md)
- [ADR-009](./adrs/ADR-009-shell-state-model-and-unidirectional-data-flow.md)
- [ADR-021](./adrs/ADR-021-centralized-design-system.md)

## Exit gate

**G06 — Basic Panel**

---

# Milestone 5 — Notch Foundation and Module Host

## Objective

Establish the Notch as the dynamic host for independently owned module crates.

## Steps

### 5.1 — Implement Notch geometry

```text
concave corners
width/height transitions
transparent hit regions
focus ownership
click outside
```

### 5.2 — Create module host API

Add under `shell-ui-gpui`:

```text
module_host/
├── module.rs
├── registry.rs
└── context.rs
```

The exact trait/API may evolve, but it must provide stable module identity, rendering entry point and controlled application context.

### 5.3 — Add module routing

Prefer:

```rust
enum NotchRoute {
    Idle,
    Module(ModuleId),
}
```

instead of hardcoding every feature directly into the Notch state machine.

### 5.4 — Add ModuleRegistry

`shell-app` registers concrete module crates. `shell-ui-gpui` only knows the host contract.

### 5.5 — Add sizing path

```text
module content
→ measured/preferred size
→ Notch target geometry
→ animation
→ surface/input region update
```

Modules do not resize Wayland surfaces directly.

## Deliverables

```text
NotchGeometry
NotchState
FocusManager
ModuleId
NotchModule host contract
ModuleRegistry
module sizing path
```

## Completion criteria

A fake/test module can be registered without modifying Notch rendering internals, activated by route, rendered, resized and dismissed.

## References

- [Module architecture](./modules/README.md)
- [Module migration plan](./module-crate-migration.md)
- [gates.md — G07, G08](./gates.md)
- [ADR-017](./adrs/ADR-017-focus-keyboard-and-input-region-coordination.md)
- [ADR-018](./adrs/ADR-018-notch-as-a-feature-container.md)
- [ADR-019](./adrs/ADR-019-custom-geometry-rendering.md)
- [ADR-020](./adrs/ADR-020-animation-system.md)

## Exit gates

```text
G07 — Notch Geometry
G08 — Notch Interaction
```

---

# Milestone 5A — Clock Module

## Objective

Use the simplest real module to validate the crate/host boundary.

## Steps

Create `shell-module-clock` with:

```text
current date/time
formatting
12h/24h option
live update policy
Notch content layout
```

Register it through `shell-app` and open it through `ModuleId::Clock`.

## Completion criteria

- Clock is a standalone crate.
- Notch does not import `ClockModule` directly.
- the module uses shared primitives/theme.
- opening Clock causes the Notch to size through the common host path.

## Reference

- [Clock module](./modules/clock.md)

---

# Milestone 6 — Launcher Module

## Objective

Validate the module architecture with keyboard input, search and application actions.

## Steps

Implement in `shell-module-launcher`:

```text
.desktop discovery
normalized app records
search/indexing
keyboard navigation
selection
execution intent
icon resolution
```

Application discovery/execution must be provided through a service boundary rather than direct widget subprocesses.

## Deliverables

```text
launcher crate
application discovery service
application execution service
icon resolver
Notch launcher route
```

## Completion criteria

The Launcher crate can be registered/removed from `shell-app` without changing Notch internals.

## References

- [Launcher module](./modules/launcher.md)
- [gates.md — G09](./gates.md)
- [ADR-027](./adrs/ADR-027-application-discovery-and-launcher-execution.md)
- [ADR-028](./adrs/ADR-028-icons-and-asset-pipeline.md)

## Exit gate

**G09 — Launcher MVP**

---

# Milestone 7 — Linux Service Layer

## Objective

Build reusable event-driven services consumed by modules and shell surfaces.

## Steps

Implement:

```text
async service bridge
AudioPort
PowerPort
NetworkPort
MediaPort / MPRIS
resource metrics service baseline
```

Shared services must have one authoritative state source and be injected into consumers.

## Deliverables

```text
audio service
battery/power service
network service
MPRIS/media service
resource metrics service
```

## References

- [gates.md — G10, G11](./gates.md)
- [ADR-011](./adrs/ADR-011-async-runtime-and-concurrency-model.md)
- [ADR-012](./adrs/ADR-012-linux-service-adapter-pattern.md)
- [ADR-013](./adrs/ADR-013-d-bus-stack.md)
- [ADR-014](./adrs/ADR-014-audio-backend.md)

## Exit gates

```text
G10 — Audio Service
G11 — System Services Baseline
```

---

# Milestone 7A — Player Module

## Objective

Validate a module backed by a live event-driven Linux service.

## Steps

Create `shell-module-player` consuming `MediaPort`:

```text
active player
track metadata
play/pause
next/previous
player changes
no-player state
```

## Completion criteria

The Player owns no MPRIS connection. It only consumes media state/actions through the application/service boundary.

## Reference

- [Player module](./modules/player.md)

---

# Milestone 7B — Calendar Module

## Objective

Add a richer content module with local navigation state.

## Steps

Create `shell-module-calendar` with:

```text
current month
month navigation
today highlight
localized weekday labels
responsive Notch layout
```

External account/calendar synchronization is not part of the initial milestone.

## Completion criteria

The module remains self-contained and changes Notch dimensions only through the host layout/sizing path.

## Reference

- [Calendar module](./modules/calendar.md)

---

# Milestone 7C — Resources Module

## Objective

Add the initial Task Manager / Resources experience.

## Steps

Create `shell-module-resources` consuming the resource metrics service:

```text
CPU
memory
swap
disk
optional temperature
bounded history
controlled sampling cadence
```

Process-level management may be added later; the MVP is system-level resource visibility.

## Completion criteria

Metrics collection remains outside the GPUI render path and the module does not spawn monitoring commands per render.

## Reference

- [Resources module](./modules/resources.md)

---

# Milestone 8 — Notifications, Tray and OSD

## Objective

Complete event-driven desktop feedback infrastructure.

## Steps

Implement:

```text
notification ownership decision
notification ingestion/history/actions
OSD manager
StatusNotifierItem tray
focus-safe overlay behavior
```

## References

- [gates.md — G12, G13](./gates.md)
- [ADR-015](./adrs/ADR-015-notification-daemon-ownership.md)
- [ADR-016](./adrs/ADR-016-system-tray-and-statusnotifieritem.md)

## Exit gates

```text
G12 — Notifications
G13 — OSD
```

---

# Milestone 9 — Visual System and Theme Module

## Objective

Establish the shared design system and expose it through the Theme module.

## Steps

### 9.1 — Complete shared design tokens

```text
colors
typography
spacing
radius
shadows
motion
icons
```

### 9.2 — Extract proven shared primitives

```text
button
icon button
popup surface
slider
toggle
search input
tooltip
```

### 9.3 — Implement `shell-module-theme`

The Theme module controls/edits the shared theme model; it does not create a second independent theme system.

It should integrate with:

```text
theme.toml
ConfigPort
config hot reload
validated theme snapshots
```

### 9.4 — Validate propagation

A valid theme change should update Panel, Notch and all registered modules without restart.

## Completion criteria

The UI is visually consistent and the Theme module changes centralized tokens rather than styling modules independently.

## References

- [Theme module](./modules/theme.md)
- [gates.md — G14](./gates.md)
- [ADR-021](./adrs/ADR-021-centralized-design-system.md)
- [ADR-022](./adrs/ADR-022-ui-primitive-extraction-policy.md)

## Exit gate

**G14 — Visual System**

---

# Milestone 10 — Resilience, Config Hot Reload and Settings Module

## Objective

Make Shell resilient and expose stable configuration through the Settings module.

## Steps

### 10.1 — Failure recovery

Test unavailable PipeWire, NetworkManager, MPRIS, battery and compositor IPC.

### 10.2 — Config hot reload

Use the documented flow:

```text
watch directory
→ debounce
→ parse complete candidate
→ validate
→ atomic snapshot replacement
→ ConfigChanged
```

Invalid reloads preserve the last valid snapshot.

### 10.3 — Implement `shell-module-settings`

Settings consumes stable configuration/service capabilities to expose shell preferences.

Initial categories:

```text
general
modules
appearance
input/keybinds
compositor-supported options
```

### 10.4 — Keep boundaries

Settings must not become a direct collection of `hyprctl`, `wpctl`, D-Bus or filesystem calls.

## Completion criteria

- Settings is an individual crate.
- configuration writes are typed and validated.
- invalid config cannot brick the desktop session.
- optional subsystem failure does not crash Settings or the shell.

## References

- [Settings module](./modules/settings.md)
- [Module migration plan](./module-crate-migration.md)
- [gates.md — G15](./gates.md)
- [ADR-010](./adrs/ADR-010-persistent-configuration-model.md)
- [ADR-024](./adrs/ADR-024-error-handling-and-recovery.md)
- [ADR-025](./adrs/ADR-025-logging-diagnostics-and-observability.md)

## Exit gate

**G15 — Failure Resilience**

---

# Milestone 11 — Performance Baseline and Optimization

## Objective

Measure the real shell and module architecture before optimizing.

## Measure

```text
cold startup
idle RAM
idle CPU
Notch frame time
module switch latency
launcher filtering latency
MPRIS update latency
resource sampling overhead
theme reload latency
memory stability after repeated module switching
```

Only optimize measured bottlenecks.

## Module-specific requirement

Lazy initialization is allowed and encouraged where it improves startup behavior, but modules remain statically linked unless a future ADR changes the plugin model.

## References

- [gates.md — G16](./gates.md)
- [ADR-034](./adrs/ADR-034-performance-budgets-and-regression-policy.md)

## Exit gate

**G16 — Performance Baseline**

---

# Milestone 12 — Shell MVP

## Objective

Reach the first daily-usable Shell shell with the initial module architecture proven end-to-end.

## Required shell infrastructure

```text
panel
workspaces
notch
multi-monitor
configuration + hot reload
audio
network
battery
MPRIS
notifications
OSD
system tray baseline
theme system
```

## Required module crates

```text
shell-module-clock
shell-module-launcher
shell-module-calendar
shell-module-player
shell-module-resources
shell-module-theme
shell-module-settings
```

## Steps

### 12.1 — Register all initial modules

`shell-app` composes the final module registry.

### 12.2 — Remove fake module state

All MVP modules use real application/service/config state where applicable.

### 12.3 — Validate Notch transitions

Exercise transitions between different content sizes:

```text
Clock → Launcher
Launcher → Calendar
Calendar → Player
Player → Resources
Resources → Theme
Theme → Settings
Settings → Idle
```

### 12.4 — Validate startup and lazy initialization

Core shell becomes usable before noncritical modules/services finish initialization.

### 12.5 — Validate multi-monitor/fullscreen

Module activation, input regions and dismissal must remain correct across outputs and fullscreen applications.

### 12.6 — Validate crate boundaries

Confirm:

```text
shell-ui-gpui does not depend on concrete module crates
shell-app performs module registration
module crates do not own Wayland surfaces
module crates do not execute raw system commands from widgets
```

### 12.7 — Run all prior mandatory gates

## Deliverables

```text
daily-usable shell MVP
all seven initial module crates
stable Notch module host
configurable module enablement
release build
known limitations document
```

## Completion criteria

The shell can run a normal Hyprland session with the initial module set and without another launcher or equivalent shell frontend for the covered functionality.

## References

- [Module architecture](./modules/README.md)
- [Module migration plan](./module-crate-migration.md)
- [gates.md — G17](./gates.md)
- [ADR-035](./adrs/ADR-035-ambxst-as-behavioral-reference-not-port-target.md)

## Exit gate

**G17 — Shell MVP**

---

# Milestone 13 — GPUI Continuation Review

## Objective

Decide whether GPUI remains the long-term presentation layer using evidence from the real module host and shell.

## Review

```text
platform patch surface
custom geometry
Notch/module transitions
text/effects
layer-shell/input regions
fractional scaling
module developer velocity
performance
```

Possible result:

```text
PASS → retain GPUI
CONDITIONAL PASS → retain GPUI + isolated platform extensions
FAIL → begin custom frontend program
```

A frontend replacement may require presentation rewrites in module crates, but must preserve core/services/config contracts.

## References

- [gates.md — G18](./gates.md)
- [ADR-001](./adrs/ADR-001-gpui-as-the-initial-frontend.md)
- [ADR-003](./adrs/ADR-003-frontend-must-remain-replaceable.md)
- [ADR-033](./adrs/ADR-033-future-custom-renderer-exit-path.md)

## Exit gate

**G18 — GPUI Continuation Decision**

---

# Milestone 14 — Post-MVP Desktop Features

## Objective

Expand beyond the initial module set only after the core/module architecture is stable.

## Track 14A — Dock

```text
application integration
running indicators
launch/focus
auto-hide
multi-monitor
fullscreen behavior
```

Gate: [G19](./gates.md)

## Track 14B — Overview

```text
window visualization
workspace visualization
focus/select window
performance with many windows
```

Gate: [G20](./gates.md)

## Track 14C — Future modules

Potential crates may include:

```text
network
bluetooth
power
clipboard
notifications
weather
AI/assistant
```

A new feature should become a module crate when it represents an independently owned Notch feature boundary, not merely because it has multiple files.

---

# Milestone 15 — Lockscreen

## Objective

Implement a secure Wayland session lock after shell core maturity.

Validate:

```text
session-lock protocol
authentication boundary
all outputs
exclusive input
output hotplug while locked
crash behavior
bypass scenarios
```

The lockscreen is security infrastructure, not a normal Notch module.

## References

- [gates.md — G21](./gates.md)
- [ADR-030](./adrs/ADR-030-lockscreen-security-architecture.md)

## Exit gate

**G21 — Lockscreen Security Gate**

---

# Milestone 16 — Architecture Portability Proof

## Objective

Prove frontend and module composition boundaries are real rather than theoretical.

## Steps

Validate core/services without GPUI and review dependency graphs for accidental coupling.

Additionally prove that module registration is owned by composition rather than the Notch host:

```text
remove one module dependency from shell-app
→ host still compiles
→ remaining shell/module host architecture remains valid
```

A headless or alternate consumer must be able to observe core state and issue application commands without loading GPUI.

## References

- [gates.md — G22](./gates.md)
- [ADR-003](./adrs/ADR-003-frontend-must-remain-replaceable.md)
- [ADR-033](./adrs/ADR-033-future-custom-renderer-exit-path.md)

## Exit gate

**G22 — Backend Replacement Readiness**

---

# Release Mapping

## Technical Prototype

Requires:

```text
M0–M5
```

Proves:

```text
GPUI
Wayland
layer-shell
multi-monitor
Hyprland integration
Notch
module host contract
```

## Alpha

Requires at minimum:

```text
M0–M9
Clock
Launcher
Player
Calendar
Resources
```

Expected state:

```text
functional developer shell
real services
working module registry
several real module crates
known UX rough edges acceptable
```

## MVP

Requires:

```text
M0–M12
all seven initial modules
G00–G17
```

## Beta

Requires:

```text
MVP
M13 GPUI continuation review
resilience hardening
configuration stabilization
```

## Stable

Requires:

```text
no critical focus/input bugs
no shell-crashing optional service failures
multi-monitor validated
fractional scaling validated
module registration boundaries preserved
performance regression process established
security-sensitive features gated separately
architecture docs matching implementation
```

---

# Critical Path

```text
M0   Repository + module crate skeletons
 ↓
M1   GPUI + Layer Shell
 ↓
M2   Surface Architecture + Multi-Monitor
 ↓
M3   Hyprland Adapter
 ↓
M4   Panel
 ↓
M5   Notch + Module Host
 ↓
M5A  Clock
 ↓
M6   Launcher
 ↓
M7   Linux Services
 ├──→ M7A Player
 ├──→ M7B Calendar
 └──→ M7C Resources
 ↓
M8   Notifications / Tray / OSD
 ↓
M9   Visual System + Theme
 ↓
M10  Resilience + Settings
 ↓
M11  Performance
 ↓
M12  MVP / all initial modules
 ↓
M13  GPUI Continuation Review
```

Post-MVP:

```text
M14 Desktop Features / future module crates
M15 Lockscreen
M16 Architecture Portability Proof
```

---

# Project Rule

Milestones organize delivery.

[decisions.md](./decisions.md) defines architectural decisions.

[invariants.md](./invariants.md) defines what implementations may not violate.

[gates.md](./gates.md) defines evidence required before progression.

[modules/README.md](./modules/README.md) defines the module boundary.

[module-crate-migration.md](./module-crate-migration.md) defines the migration required to reach that boundary.

If implementation conflicts with these documents:

```text
invariant violation
→ implementation is wrong

gate failure
→ milestone is not complete

module boundary violation
→ feature must be moved back behind the correct crate/host/service contract

decision contradicted by evidence
→ update or supersede the decision/ADR
```

Do not change module ownership or dependency direction silently inside feature code.
