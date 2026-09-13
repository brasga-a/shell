# milestones.md

# Project Milestones — Linux Shell UI

This document defines the execution roadmap for the Rust/Wayland Linux shell project.

It complements:

- [decisions.md](./decisions.md)
- [invariants.md](./invariants.md)
- [gates.md](./gates.md)
- [ADRs](./adrs/README.md)

The project must not advance to the next milestone while a mandatory gate of the current milestone is failing.

---

# Milestone 0 — Repository and Architecture Foundation

## Objective

Create the repository structure and dependency boundaries before implementing visible shell features.

## Steps

### 0.1 — Create Cargo workspace

Initial structure:

```text
crates/
├── shell-core/
├── shell-platform/
├── shell-hyprland/
├── shell-linux/
├── shell-config/
├── shell-theme/
├── shell-ui-gpui/
└── shell-app/
```

### 0.2 — Define core domain primitives

Create initial types:

```text
OutputId
WorkspaceId
WindowId
SurfaceId

Point
Size
Rect

Output
Workspace
Window
```

### 0.3 — Define first application ports

Start with:

```text
CompositorPort
ConfigPort
```

### 0.4 — Establish error boundaries

Initial candidates:

```text
CompositorError
ConfigError
PlatformError
```

### 0.5 — Add observability baseline

Configure `tracing` and `tracing-subscriber`.

### 0.6 — Add repository checks

CI must run:

```text
cargo check
cargo test
cargo fmt --check
cargo clippy
```

## Deliverables

```text
compiling Cargo workspace
dependency graph respecting architecture
initial core types
initial ports
structured logging
CI pipeline
```

## Completion criteria

- `shell-core` compiles without GPUI.
- `shell-core` compiles without Hyprland-specific dependencies.
- `shell-app` is the composition root.
- No UI or compositor implementation leaks into domain types.

## Required references

- [decisions.md — D001, D003, D004, D023](./decisions.md)
- [invariants.md — I001, I002, I016, I023, I024](./invariants.md)
- [gates.md — G00 Repository Foundation](./gates.md)
- [ADR-002 — Hexagonal Architecture](./adrs/ADR-002-hexagonal-architecture-for-shell-boundaries.md)
- [ADR-023 — Cargo Workspace Boundaries](./adrs/ADR-023-cargo-workspace-boundaries.md)

## Exit gate

**G00 — Repository Foundation**

---

# Milestone 1 — GPUI + Wayland Viability

## Objective

Prove that GPUI can operate as the shell frontend before building product features on top of it.

## Steps

### 1.1 — Minimal GPUI application

Validate:

```text
startup
rendering
pointer input
keyboard input
resizing
shutdown
```

### 1.2 — Transparent surface

Render a transparent surface with visible custom content.

### 1.3 — Investigate Linux backend boundary

Identify exactly where GPUI exposes or must be extended for:

```text
Wayland surface creation
layer-shell
output selection
keyboard interactivity
input regions
```

### 1.4 — Layer-shell PoC

Create `Layer::Top`, then `Layer::Overlay`.

### 1.5 — Validate shell semantics

Test:

```text
anchors
exclusive zone
transparent background
keyboard interactivity
specific output targeting
fullscreen application interaction
```

## Deliverables

```text
minimal GPUI Wayland executable
transparent shell surface
layer-shell PoC
documented GPUI backend limitations
```

## Completion criteria

The project can create a reliable shell surface under Hyprland.

If layer-shell cannot be implemented cleanly, stop here and reopen the frontend decision.

## Required references

- [decisions.md — D002, D003, D006](./decisions.md)
- [invariants.md — I001, I007, I038](./invariants.md)
- [gates.md — G01, G02](./gates.md)
- [ADR-001 — GPUI as Initial Frontend](./adrs/ADR-001-gpui-as-the-initial-frontend.md)
- [ADR-004 — Wayland Layer-Shell Integration](./adrs/ADR-004-wayland-layer-shell-integration-strategy.md)

## Exit gates

```text
G01 — GPUI Window Viability
G02 — Layer Shell Viability
```

---

# Milestone 2 — Surface Architecture and Multi-Monitor

## Objective

Resolve the shell's Wayland surface topology and establish correct per-output lifecycle.

## Steps

### 2.1 — Implement unified surface experiment

Prototype:

```text
Output
└── fullscreen transparent surface
    ├── panel region
    ├── notch region
    └── overlay region
```

### 2.2 — Implement independent surface experiment

Prototype:

```text
Output
├── panel surface
├── notch surface
└── overlay surface
```

### 2.3 — Compare input behavior

Test both approaches for:

```text
click-through
input regions
keyboard focus
click-outside detection
```

### 2.4 — Compare lifecycle behavior

Test:

```text
surface resizing
workspace switching
fullscreen windows
output hotplug
output removal
```

### 2.5 — Resolve ADR-005

Select the surface topology using measured behavior rather than preference.

### 2.6 — Implement OutputState

```rust
struct OutputState {
    id: OutputId,
    geometry: Rect,
    scale: f32,
}
```

### 2.7 — Add output hotplug lifecycle

Support add/change/remove without restarting the shell.

### 2.8 — Validate fractional scaling

Verify layout, geometry and hit testing under non-1.0 scale.

## Deliverables

```text
surface topology comparison
accepted surface architecture
multi-monitor state model
output lifecycle manager
fractional scaling validation
```

## Completion criteria

- One surface topology is formally selected.
- Multiple outputs work independently.
- Surface ownership is explicit.
- Output hotplug does not crash the shell.

## Required references

- [decisions.md — D006, D007, D008](./decisions.md)
- [invariants.md — I005, I006, I007, I026, I027](./invariants.md)
- [gates.md — G03, G04](./gates.md)
- [ADR-005 — Shell Surface Topology](./adrs/ADR-005-shell-surface-topology.md)
- [ADR-006 — Multi-Monitor State](./adrs/ADR-006-multi-monitor-state-and-surface-ownership.md)

## Exit gates

```text
G03 — Surface Topology
G04 — Multi-Monitor Correctness
```

---

# Milestone 3 — Hyprland Integration

## Objective

Establish Hyprland as the first compositor adapter without allowing Hyprland-specific data to define application state.

## Steps

### 3.1 — Implement Hyprland IPC client

Support command and state requests.

### 3.2 — Implement event socket listener

Subscribe to relevant compositor events.

### 3.3 — Define compositor domain models

```text
Monitor
Workspace
Window
FocusedWindow
FullscreenState
```

### 3.4 — Translate Hyprland payloads

All raw JSON/string events terminate inside `shell-hyprland`.

### 3.5 — Implement initial CompositorPort

Required operations:

```text
list monitors
list workspaces
list windows
focus workspace
focus window
```

### 3.6 — Add compositor capabilities

Represent optional features explicitly.

### 3.7 — Handle compositor races

Test disappearing windows, output removal, workspace reassignment and `None` focused windows.

## Deliverables

```text
Hyprland adapter
event-driven compositor state
CompositorPort implementation
capability model
reconnect/error handling
```

## Completion criteria

The frontend can consume real compositor state without seeing raw Hyprland payloads.

## Required references

- [decisions.md — D005, D009, D010](./decisions.md)
- [invariants.md — I002, I010, I022, I028, I030](./invariants.md)
- [gates.md — G05](./gates.md)
- [ADR-007 — Hyprland Adapter](./adrs/ADR-007-hyprland-as-a-compositor-adapter.md)
- [ADR-008 — Compositor Capability Model](./adrs/ADR-008-compositor-port-and-capability-model.md)

## Exit gate

**G05 — Hyprland Adapter**

---

# Milestone 4 — First Real Panel

## Objective

Create the first useful shell component using real compositor state and proper application flow.

## Steps

### 4.1 — Create design tokens baseline

Add:

```text
colors
spacing
radius
typography
```

### 4.2 — Build panel container

Implement panel geometry and surface behavior.

### 4.3 — Add workspace module

Display workspaces and focused workspace from real compositor state.

### 4.4 — Add clock

Keep timing/update logic out of arbitrary widget state where practical.

### 4.5 — Add shell command path

Workspace click must flow:

```text
UI
↓
application command
↓
CompositorPort
↓
Hyprland
```

### 4.6 — Record performance baseline

Measure release build:

```text
cold startup
idle RAM
idle CPU
```

## Deliverables

```text
usable top panel
workspace selector
clock
initial theme tokens
first performance baseline
```

## Completion criteria

The panel uses real event-driven compositor state and does not access Hyprland directly.

## Required references

- [decisions.md — D009, D010, D018, D021](./decisions.md)
- [invariants.md — I009, I010, I020, I034](./invariants.md)
- [gates.md — G06](./gates.md)
- [ADR-009 — State Model](./adrs/ADR-009-shell-state-model-and-unidirectional-data-flow.md)
- [ADR-021 — Design System](./adrs/ADR-021-centralized-design-system.md)

## Exit gate

**G06 — Basic Panel**

---

# Milestone 5 — Notch Foundation

## Objective

Validate the main custom visual interaction of the shell.

## Steps

### 5.1 — Define NotchState

Initial semantic states:

```text
Idle
Launcher
```

### 5.2 — Implement NotchGeometry

Represent:

```text
width
height
radius
corner size
edge
```

### 5.3 — Render concave corners

Evaluate in order:

```text
GPUI path/custom paint
↓
lyon/tessellation if required
↓
shader only if justified
```

### 5.4 — Add dynamic dimensions

Support expansion and contraction.

### 5.5 — Add animation

Implement interruptible width/height transitions.

### 5.6 — Implement focus coordinator

Centralize:

```text
keyboard focus
click outside
modal ownership
input-region expansion
```

### 5.7 — Validate transparent hit regions

Transparent geometry must not incorrectly block windows beneath it.

### 5.8 — Stress interaction

Rapidly test open/close, interrupted transitions, Escape, click outside and workspace changes.

## Deliverables

```text
NotchState
NotchGeometry
working concave notch
animation layer
FocusManager
input-region handling
```

## Completion criteria

The notch can expand and collapse smoothly without corrupting focus, application state or pointer behavior.

## Required references

- [decisions.md — D015, D016, D017](./decisions.md)
- [invariants.md — I008, I018, I019](./invariants.md)
- [gates.md — G07, G08](./gates.md)
- [ADR-017 — Focus/Input Coordination](./adrs/ADR-017-focus-keyboard-and-input-region-coordination.md)
- [ADR-018 — Notch Feature Container](./adrs/ADR-018-notch-as-a-feature-container.md)
- [ADR-019 — Custom Geometry](./adrs/ADR-019-custom-geometry-rendering.md)
- [ADR-020 — Animation System](./adrs/ADR-020-animation-system.md)

## Exit gates

```text
G07 — Notch Geometry
G08 — Notch Interaction
```

---

# Milestone 6 — Launcher

## Objective

Turn the notch into a real feature container with the first non-trivial application feature.

## Steps

### 6.1 — Desktop entry discovery

Parse freedesktop application entries.

### 6.2 — Normalize application records

Internal model:

```text
id
name
description
icon
exec information
```

### 6.3 — Build application index

Support efficient filtering.

### 6.4 — Implement launcher UI

Support:

```text
text input
keyboard navigation
selection
Enter
Escape
```

### 6.5 — Implement application execution service

Launching applications must not be performed ad hoc inside widgets.

### 6.6 — Implement icon resolution

Resolve freedesktop icon themes and fallbacks.

### 6.7 — Test responsiveness

Use a realistic application list.

## Deliverables

```text
application discovery service
launcher index
launcher UI
application execution service
icon resolution
```

## Completion criteria

Launcher behavior remains independent from notch rendering internals.

## Required references

- [decisions.md — D015, D018](./decisions.md)
- [invariants.md — I003, I016, I036](./invariants.md)
- [gates.md — G09](./gates.md)
- [ADR-027 — Application Discovery](./adrs/ADR-027-application-discovery-and-launcher-execution.md)
- [ADR-028 — Icons and Assets](./adrs/ADR-028-icons-and-asset-pipeline.md)

## Exit gate

**G09 — Launcher MVP**

---

# Milestone 7 — Linux Service Layer

## Objective

Establish the reusable service architecture for system state.

## Steps

### 7.1 — Async infrastructure

Settle the service-task/event bridge between adapters and application state.

### 7.2 — Audio

Implement volume, mute, commands and event updates.

### 7.3 — Battery / Power

Implement availability, percentage and charging state.

### 7.4 — Network

Implement connected state, current connection and basic status.

### 7.5 — MPRIS

Implement player detection, metadata, playback state and play/pause.

### 7.6 — Share services across consumers

Example:

```text
AudioService
├── panel
├── OSD
└── future settings
```

### 7.7 — Remove temporary CLI paths where appropriate

Production integration should prefer native protocol/API adapters.

## Deliverables

```text
async service bridge
AudioPort implementation
PowerPort implementation
NetworkPort implementation
MediaPort implementation
```

## Completion criteria

Each subsystem has one authoritative state source and failure does not crash the shell.

## Required references

- [decisions.md — D011, D012, D013, D014](./decisions.md)
- [invariants.md — I010, I011, I012, I025, I035](./invariants.md)
- [gates.md — G10, G11](./gates.md)
- [ADR-011 — Async Runtime](./adrs/ADR-011-async-runtime-and-concurrency-model.md)
- [ADR-012 — Linux Service Adapter Pattern](./adrs/ADR-012-linux-service-adapter-pattern.md)
- [ADR-013 — D-Bus Stack](./adrs/ADR-013-d-bus-stack.md)
- [ADR-014 — Audio Backend](./adrs/ADR-014-audio-backend.md)

## Exit gates

```text
G10 — Audio Service
G11 — System Services Baseline
```

---

# Milestone 8 — Notifications, Tray and OSD

## Objective

Complete the core event-driven desktop feedback layer.

## Steps

### 8.1 — Resolve notification ownership

Settle whether the shell owns `org.freedesktop.Notifications`.

### 8.2 — Implement notification ingestion

Internal types:

```text
NotificationId
Notification
NotificationAction
```

### 8.3 — Implement popup lifecycle

Support:

```text
show
timeout
dismiss
action
history
```

### 8.4 — Implement OSD manager

Initial:

```text
volume
brightness or microphone
```

### 8.5 — Integrate system tray

Implement modern StatusNotifierItem support.

### 8.6 — Validate overlay focus rules

OSD must not steal keyboard focus.

## Deliverables

```text
notification subsystem
notification UI
OSD subsystem
system tray baseline
```

## Completion criteria

External asynchronous events can create transient shell UI safely without focus/input corruption.

## Required references

- [decisions.md — D012, D017](./decisions.md)
- [invariants.md — I008, I010, I012](./invariants.md)
- [gates.md — G12, G13](./gates.md)
- [ADR-015 — Notification Ownership](./adrs/ADR-015-notification-daemon-ownership.md)
- [ADR-016 — System Tray](./adrs/ADR-016-system-tray-and-statusnotifieritem.md)
- [ADR-017 — Focus/Input Coordination](./adrs/ADR-017-focus-keyboard-and-input-region-coordination.md)

## Exit gates

```text
G12 — Notifications
G13 — OSD
```

---

# Milestone 9 — Visual System and Shared Components

## Objective

Turn the working shell into a coherent UI system without building an unnecessary generic toolkit.

## Steps

### 9.1 — Complete design tokens

Add:

```text
colors
typography
spacing
radius
shadows
motion
icons
```

### 9.2 — Identify repeated patterns

Extract only patterns already repeated across real features.

Likely candidates:

```text
button
icon button
popup surface
slider
toggle
search input
tooltip
```

### 9.3 — Establish asset pipeline

Define SVG handling, icon lookup, image cache and fallback behavior.

### 9.4 — Standardize motion

Create reusable motion tokens rather than per-feature arbitrary easing.

### 9.5 — Validate at least three features

Required consumers:

```text
panel
notch
launcher
```

## Deliverables

```text
complete shell-theme
shared primitive set
asset conventions
motion tokens
```

## Completion criteria

The UI is visually consistent without introducing a second generic toolkit over GPUI.

## Required references

- [decisions.md — D018, D019, D020](./decisions.md)
- [invariants.md — I020, I021, I022](./invariants.md)
- [gates.md — G14](./gates.md)
- [ADR-021 — Design System](./adrs/ADR-021-centralized-design-system.md)
- [ADR-022 — UI Primitive Policy](./adrs/ADR-022-ui-primitive-extraction-policy.md)
- [ADR-028 — Icons and Assets](./adrs/ADR-028-icons-and-asset-pipeline.md)
- [ADR-029 — Text Rendering](./adrs/ADR-029-text-rendering-strategy.md)

## Exit gate

**G14 — Visual System**

---

# Milestone 10 — Resilience and Recovery

## Objective

Make the shell behave like infrastructure rather than a demo.

## Steps

### 10.1 — Service failure testing

Simulate:

```text
PipeWire unavailable
NetworkManager unavailable
MPRIS unavailable
no battery
```

### 10.2 — Compositor reconnect behavior

Handle Hyprland IPC interruption where practical.

### 10.3 — Invalid configuration recovery

Invalid values must fall back safely.

### 10.4 — Output churn testing

Repeatedly test connect/disconnect/scale/workspace changes.

### 10.5 — Eliminate recoverable panics

Audit `unwrap()`, `expect()` and `panic!()` in production paths.

### 10.6 — Improve diagnostics

Log failures with subsystem/output/surface context.

## Deliverables

```text
reconnect paths
graceful unavailable states
config recovery
panic audit
failure test matrix
```

## Completion criteria

Failure of an optional subsystem does not terminate the usable shell.

## Required references

- [decisions.md — D022, D024, D025](./decisions.md)
- [invariants.md — I012, I013, I023, I024, I028, I032, I033](./invariants.md)
- [gates.md — G15](./gates.md)
- [ADR-024 — Error Handling](./adrs/ADR-024-error-handling-and-recovery.md)
- [ADR-025 — Observability](./adrs/ADR-025-logging-diagnostics-and-observability.md)

## Exit gate

**G15 — Failure Resilience**

---

# Milestone 11 — Performance Baseline and Optimization

## Objective

Measure the actual shell before deciding whether performance work or architectural changes are required.

## Steps

### 11.1 — Build release benchmark profile

Use a repeatable environment.

### 11.2 — Measure idle behavior

Record:

```text
RAM
CPU
startup time
```

### 11.3 — Measure interaction behavior

Record:

```text
notch frame time
launcher filtering latency
workspace event latency
volume event latency
```

### 11.4 — Measure memory stability

Exercise repeated notch open/close, launcher use, workspace switching, notifications and output changes.

### 11.5 — Identify pathological behavior

Only optimize measured bottlenecks.

### 11.6 — Define regression budgets

Budgets are based on baseline data, not arbitrary numbers.

## Deliverables

```text
performance report
baseline metrics
identified bottlenecks
regression budget
```

## Completion criteria

No pathological idle usage, runaway memory growth, event starvation or persistent animation stalls remain.

## Required references

- [decisions.md](./decisions.md)
- [invariants.md — I039, I040](./invariants.md)
- [gates.md — G16](./gates.md)
- [ADR-034 — Performance Budgets](./adrs/ADR-034-performance-budgets-and-regression-policy.md)

## Exit gate

**G16 — Performance Baseline**

---

# Milestone 12 — Shell MVP

## Objective

Reach the first daily-usable shell.

## Required features

```text
panel
workspaces
clock
notch
launcher
audio
network status
battery status
MPRIS
notifications
OSD
system tray baseline
multi-monitor
configuration
theme system
```

## Steps

### 12.1 — Integrate all MVP modules

Remove development-only fake state.

### 12.2 — Validate startup dependencies

Core panel must become usable before optional services finish loading.

### 12.3 — Validate multi-monitor end-to-end

Every MVP feature must behave correctly per output where relevant.

### 12.4 — Validate fullscreen behavior

Test games, video and fullscreen applications.

### 12.5 — Validate configuration startup

Bad config must not brick the session.

### 12.6 — Run invariant review

Use the checklist from `invariants.md`.

### 12.7 — Run all prior mandatory gates

No unresolved mandatory gate failure is accepted.

## Deliverables

```text
daily-usable shell MVP
documented configuration
stable startup
release build
known limitations document
```

## Completion criteria

The user can run a normal Hyprland session without requiring another bar, launcher or notification frontend for MVP functionality.

## Required references

- [decisions.md](./decisions.md)
- [invariants.md](./invariants.md)
- [gates.md — G17](./gates.md)
- [ADR-035 — Ambxst as Behavioral Reference](./adrs/ADR-035-ambxst-as-behavioral-reference-not-port-target.md)

## Exit gate

**G17 — Shell MVP**

---

# Milestone 13 — GPUI Continuation Review

## Objective

Decide whether GPUI remains the long-term presentation layer using real project evidence.

## Steps

### 13.1 — Review platform patch surface

Measure how much custom GPUI/Linux backend code is being maintained.

### 13.2 — Review rendering constraints

Evaluate custom geometry, animations, resizing, text, effects and GPU behavior.

### 13.3 — Review Wayland constraints

Evaluate layer-shell, input regions, output handling, fractional scaling and focus.

### 13.4 — Review developer velocity

Determine whether GPUI is still accelerating or obstructing feature work.

### 13.5 — Compare alternatives using measured needs

Only now compare against:

```text
SCTK
wayland-client
wgpu
lyon
dedicated text stack
```

### 13.6 — Record result

```text
PASS
→ retain GPUI

CONDITIONAL PASS
→ retain GPUI + isolated platform extensions

FAIL
→ begin custom frontend program
```

## Deliverables

```text
GPUI continuation report
updated ADR-001 if necessary
custom renderer plan only if justified
```

## Completion criteria

Frontend strategy is based on observed constraints rather than preference.

## Required references

- [decisions.md — D002, D003](./decisions.md)
- [invariants.md — I001, I038, I039](./invariants.md)
- [gates.md — G18](./gates.md)
- [ADR-001 — GPUI](./adrs/ADR-001-gpui-as-the-initial-frontend.md)
- [ADR-003 — Replaceable Frontend](./adrs/ADR-003-frontend-must-remain-replaceable.md)
- [ADR-033 — Custom Renderer Exit Path](./adrs/ADR-033-future-custom-renderer-exit-path.md)

## Exit gate

**G18 — GPUI Continuation Decision**

---

# Milestone 14 — Post-MVP Desktop Features

## Objective

Expand from a minimal shell into a broader desktop environment only after the core architecture is stable.

## Track 14A — Dock

### Steps

```text
application state integration
active/running indicators
launch/focus behavior
auto-hide
multi-monitor ownership
fullscreen behavior
```

### Gate

[G19 — Dock](./gates.md)

### ADR references

- [ADR-005 — Surface Topology](./adrs/ADR-005-shell-surface-topology.md)
- [ADR-027 — Application Discovery](./adrs/ADR-027-application-discovery-and-launcher-execution.md)

## Track 14B — Overview

### Steps

```text
window visualization
workspace visualization
focus/select window
workspace selection
performance with many windows
```

### Gate

[G20 — Overview](./gates.md)

### ADR references

- [ADR-007 — Hyprland Adapter](./adrs/ADR-007-hyprland-as-a-compositor-adapter.md)
- [ADR-008 — Compositor Capability Model](./adrs/ADR-008-compositor-port-and-capability-model.md)

## Track 14C — Dashboard and Tools

Possible modules:

```text
system resources
media
calendar
network controls
bluetooth controls
power controls
clipboard
```

These must consume existing services rather than create infrastructure clients inside the UI.

### Relevant invariants

- [I034 — Feature modules do not own infrastructure clients](./invariants.md)
- [I035 — Services are shared](./invariants.md)
- [I036 — UI features are composable](./invariants.md)

---

# Milestone 15 — Lockscreen

## Objective

Implement a secure session lock only after the shell core is mature.

## Steps

### 15.1 — Validate session-lock protocol

Use the appropriate Wayland session-lock mechanism.

### 15.2 — Define authentication boundary

Determine how authentication is performed and isolated.

### 15.3 — Cover every output

Including outputs attached while locked.

### 15.4 — Validate exclusive input

No bypass through ordinary shell surfaces.

### 15.5 — Validate crash behavior

Understand compositor/session behavior if lock process dies.

### 15.6 — Security testing

Attempt explicit bypass scenarios.

## Deliverables

```text
security architecture
session-lock implementation
authentication integration
failure behavior documentation
```

## Completion criteria

Lockscreen behavior has been validated as a security mechanism rather than merely a visual overlay.

## Required references

- [invariants.md — I031](./invariants.md)
- [gates.md — G21](./gates.md)
- [ADR-030 — Lockscreen Security](./adrs/ADR-030-lockscreen-security-architecture.md)

## Exit gate

**G21 — Lockscreen Security Gate**

---

# Milestone 16 — Architecture Portability Proof

## Objective

Prove that frontend replaceability is real rather than theoretical.

## Steps

### 16.1 — Run core without GPUI

Create either:

```text
headless executable
or
minimal alternate frontend
```

### 16.2 — Consume ShellState

Verify an alternate consumer can observe application state.

### 16.3 — Issue application commands

Verify commands can execute without GPUI.

### 16.4 — Run compositor/services independently

Validate Hyprland adapter, Linux services and configuration without GPUI.

### 16.5 — Review dependency graph

Ensure no accidental GPUI dependency entered the core during MVP development.

## Deliverables

```text
headless/alternate consumer
dependency review
frontend replacement validation report
```

## Completion criteria

The core and system adapters operate independently from GPUI.

## Required references

- [decisions.md — D003](./decisions.md)
- [invariants.md — I001, I017, I018, I022](./invariants.md)
- [gates.md — G22](./gates.md)
- [ADR-003 — Replaceable Frontend](./adrs/ADR-003-frontend-must-remain-replaceable.md)
- [ADR-033 — Custom Renderer Exit Path](./adrs/ADR-033-future-custom-renderer-exit-path.md)

## Exit gate

**G22 — Backend Replacement Readiness**

---

# Release Mapping

## Technical Prototype

Requires:

```text
Milestone 0
Milestone 1
Milestone 2
Milestone 3
```

At this point the project has proven:

```text
GPUI
Wayland
layer-shell
multi-monitor
Hyprland integration
```

## Alpha

Requires:

```text
Milestones 0–11
```

Expected state:

```text
functional developer shell
real services
real panel/notch/launcher
known UX rough edges acceptable
```

## MVP

Requires:

```text
Milestones 0–12
G00–G17
```

Expected state:

```text
daily-usable core shell
```

## Beta

Requires:

```text
MVP
+
Milestone 13
+
resilience hardening
+
configuration stabilization
```

## Stable

Requires:

```text
no critical focus/input bugs
no shell-crashing optional service failures
multi-monitor validated
fractional scaling validated
performance regression process established
security-sensitive features gated separately
architecture docs match implementation
```

---

# Critical Path

```text
M0  Repository Foundation
 ↓
M1  GPUI + Layer Shell
 ↓
M2  Surface Architecture + Multi-Monitor
 ↓
M3  Hyprland Adapter
 ↓
M4  Panel
 ↓
M5  Notch
 ↓
M6  Launcher
 ↓
M7  Linux Services
 ↓
M8  Notifications / Tray / OSD
 ↓
M9  Visual System
 ↓
M10 Resilience
 ↓
M11 Performance
 ↓
M12 MVP
 ↓
M13 GPUI Continuation Review
```

Post-MVP:

```text
M14 Desktop Features
M15 Lockscreen
M16 Architecture Portability Proof
```

---

# Project Rule

Milestones organize **delivery**.

[decisions.md](./decisions.md) defines what the architecture currently chooses.

[invariants.md](./invariants.md) defines what implementations are not allowed to violate.

[gates.md](./gates.md) defines the evidence required before the project advances.

If a milestone implementation conflicts with one of those documents:

```text
invariant violation
    → implementation is wrong

gate failure
    → milestone is not complete

decision contradicted by evidence
    → update/supersede the decision or ADR
```

Do not change the architecture silently inside feature code.
