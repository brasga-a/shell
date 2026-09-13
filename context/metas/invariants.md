# invariants.md

# Project Invariants — Linux Shell UI

This file defines rules that must remain true across the codebase.

Unlike architectural decisions, invariants are not preferences. They are constraints that implementations must preserve unless the architecture itself is explicitly redesigned.

---

## I001 — GPUI never leaks into the core

`GPUI` types, traits, handles, elements, contexts, windows, or rendering primitives must not appear in:

```text
shell-core
shell-platform
shell-hyprland
shell-linux
shell-config
```

Allowed dependency direction:

```text
shell-ui-gpui
      ↓
 shell-core
```

Forbidden:

```text
shell-core
      ↓
    GPUI
```

The core must compile without GPUI.

---

## I002 — Hyprland never defines domain types

The application must not expose raw Hyprland IPC payloads outside the Hyprland adapter.

Forbidden:

```rust
struct WorkspaceState {
    raw_hyprland_json: String,
}
```

Required flow:

```text
Hyprland event
    ↓
Hyprland adapter
    ↓
internal event/type
    ↓
application state
```

Examples of internal concepts:

```text
WorkspaceId
WindowId
OutputId
WorkspaceChanged
FocusedWindowChanged
FullscreenChanged
```

---

## I003 — Widgets never call Linux commands directly

UI code must never own system integration.

Forbidden inside widgets:

```text
hyprctl
wpctl
pactl
nmcli
bluetoothctl
brightnessctl
playerctl
systemctl
```

A widget emits an intent.

Example:

```text
VolumeButton
    ↓
AudioCommand::ToggleMute
    ↓
AudioPort
    ↓
Linux audio adapter
```

CLI commands may exist temporarily inside adapters during PoCs.

---

## I004 — Persistent configuration is not runtime state

Persistent configuration and ephemeral application state must remain separate.

Persistent:

```text
theme
panel size
notch settings
animations
preferred modules
user preferences
```

Runtime:

```text
focused window
active workspace
current volume
launcher open
notification list
media state
network state
```

`ShellConfig` must not become a dump for transient state.

`ShellState` must not become the persistence format.

---

## I005 — Every output is explicit

No UI placement logic may assume a single monitor.

All screen-dependent state must identify its output.

Required concept:

```rust
struct OutputState {
    id: OutputId,
    geometry: Rect,
    scale: f32,
}
```

Components that belong to a monitor must always know which `OutputId` they belong to.

---

## I006 — Fractional scale is never assumed to be 1.0

Geometry must not assume integer scale factors.

Forbidden assumptions:

```text
scale == 1
scale is integer
logical px == physical px
```

All rendering and surface calculations must distinguish logical and physical dimensions when necessary.

---

## I007 — Surface semantics are independent from widgets

Wayland concepts such as:

```text
layer
anchor
exclusive zone
keyboard interactivity
input region
output
```

must not be encoded implicitly through arbitrary widget behavior.

They must be represented explicitly through the platform/surface layer.

---

## I008 — Input ownership is centralized

Only the designated focus/input coordinator may arbitrate:

```text
keyboard focus
modal ownership
click-outside behavior
pointer capture
input-region expansion
popup ordering
```

Widgets may request focus behavior.

Widgets must not independently manipulate global shell focus policy.

---

## I009 — State changes flow through application logic

Widgets do not directly mutate unrelated global state.

Required model:

```text
UI intent
   ↓
command/action
   ↓
application logic
   ↓
state change
   ↓
render
```

External events follow:

```text
system event
   ↓
adapter
   ↓
application event
   ↓
state change
   ↓
render
```

---

## I010 — External state must have a single authoritative source

For each external subsystem, one adapter owns synchronization with the real system.

Examples:

```text
Hyprland adapter  → compositor state
Audio adapter     → audio state
Network adapter   → network state
MPRIS adapter     → media state
```

Two independent parts of the program must not poll the same subsystem and maintain competing copies of truth.

---

## I011 — Prefer events over polling

If a subsystem provides reliable events, signals, subscriptions, or sockets, use them.

Preferred:

```text
Hyprland event socket
D-Bus signals
PipeWire callbacks
Wayland events
```

Polling is only valid when:

```text
no reliable event API exists
or
polling is explicitly justified
```

Idle polling loops must not be introduced casually.

---

## I012 — UI must remain responsive while services fail

Failure of:

```text
Bluetooth
NetworkManager
PipeWire
MPRIS
battery service
weather
optional utilities
```

must not crash the shell.

Optional service failure should degrade the relevant feature only.

The panel and core shell must remain usable.

---

## I013 — No optional feature may block shell startup

Non-critical features must initialize lazily or asynchronously.

Examples:

```text
settings
weather
wallpaper browser
AI tools
screenshots
screen recording
advanced dashboards
```

The shell must reach a usable panel state without waiting for them.

---

## I014 — The shell must not require a backend daemon by default

All core functionality should work in the main Rust process unless process separation has a concrete reason.

Valid reasons include:

```text
privilege boundary
security isolation
independent lifecycle
crash isolation
external IPC contract
```

“Cleaner architecture” alone is not sufficient.

---

## I015 — No C++ dependency is introduced without a concrete need

Rust is the primary implementation language.

C/C++ is not forbidden, but any dependency on it must solve a real problem that is not reasonably handled in Rust.

Bindings to mature Linux libraries are acceptable.

Language purity is not more important than correctness or maintainability.

---

## I016 — No dependency is added without an owned use case

A crate must not be introduced merely because it appears in an architectural document.

Before adding a dependency, identify:

```text
what capability requires it
which crate/module owns it
why std or an existing dependency is insufficient
```

Avoid speculative dependency accumulation.

---

## I017 — Geometry utilities remain renderer-agnostic where practical

Math and geometry used by application logic must not depend on GPUI-specific types.

Examples:

```text
Point
Size
Rect
Insets
Transform
NotchGeometry
```

Renderer conversion happens at the frontend boundary.

Rendering-only geometry may remain GPUI-specific.

---

## I018 — Notch state is independent from notch rendering

The application state defines what the notch is doing.

Example:

```rust
enum NotchRoute {
    Idle,
    Launcher,
    Dashboard,
    Notifications,
    PowerMenu,
    Tools,
}
```

It must not encode renderer implementation details such as:

```text
Bezier control points
shader handles
GPUI paths
pixel masks
```

---

## I019 — Animation never owns business state

Animation values are presentation state.

A transition such as:

```text
Idle → Launcher
```

must be semantically complete without knowing:

```text
width = 520
height = 430
spring stiffness = ...
```

Animation interruption must not corrupt application state.

---

## I020 — Design tokens are centralized

Do not scatter visual constants across feature modules.

Centralize at minimum:

```text
colors
typography
spacing
radius
shadows
motion
icons
```

Feature-specific values are allowed only when they are genuinely feature-specific.

---

## I021 — Shared primitives must solve repeated problems

A reusable primitive exists because multiple components need the same behavior or visual contract.

Do not create generic abstractions preemptively.

Forbidden direction:

```text
build our own complete UI toolkit before building the shell
```

Preferred:

```text
extract primitives from repeated real usage
```

---

## I022 — Do not wrap all of GPUI for hypothetical portability

The project must not create fake equivalents of every GPUI primitive solely to make future migration automatic.

Architectural portability lives in:

```text
core
state
ports
services
surface semantics
domain geometry
```

Visual components may be rewritten if the renderer changes.

---

## I023 — Errors must preserve subsystem boundaries

An error from one adapter must not become an untyped global string error everywhere.

Prefer subsystem-specific error types.

Example:

```text
HyprlandError
AudioError
NetworkError
ConfigError
```

Convert to broader application errors only at explicit boundaries.

---

## I024 — Logging is structured

Use structured logging rather than ad-hoc `println!`.

Expected direction:

```text
tracing
```

Logs should identify relevant context such as:

```text
output
workspace
surface
service
event
command
```

Do not log secrets or sensitive user content.

---

## I025 — Runtime tasks do not mutate UI state arbitrarily

Async tasks and service workers communicate through controlled application channels/events.

Forbidden:

```text
random background task
    ↓
direct mutable access to arbitrary GPUI state
```

Required:

```text
service task
    ↓
event/message
    ↓
application state update
    ↓
UI invalidation/render
```

---

## I026 — Surface lifetime follows explicit ownership

Every Wayland/GPUI surface must have a clear owner.

Examples:

```text
OutputShellSurface
PanelSurface
NotchSurface
DockSurface
OverlaySurface
```

Destroying an output must cleanly destroy all surfaces associated with it.

No orphan surfaces.

---

## I027 — Output hotplug must not require restart

Monitor connection/disconnection must be treated as runtime events.

Expected behavior:

```text
output added
    ↓
create OutputState
    ↓
create relevant surfaces

output removed
    ↓
destroy surfaces
    ↓
remove OutputState
```

The shell must remain alive.

---

## I028 — Shell components must tolerate compositor state races

Wayland and compositor events are asynchronous.

The code must tolerate temporary conditions such as:

```text
workspace references removed output
window disappears before follow-up event
focused window becomes None
monitor topology changes mid-transition
```

Do not rely on event ordering unless the protocol guarantees it.

---

## I029 — IDs are not presentation strings

Use typed identifiers internally.

Prefer:

```text
OutputId
WorkspaceId
WindowId
SurfaceId
NotificationId
```

Do not identify entities through labels such as monitor names or window titles when stable IDs exist.

---

## I030 — The shell must degrade gracefully when Hyprland-specific data is unavailable

The project is Hyprland-first, not Hyprland-shaped.

Features that require compositor-specific extensions may be unavailable on another compositor, but the core model must remain valid.

Capabilities should be discoverable.

Example:

```rust
struct CompositorCapabilities {
    workspace_management: bool,
    window_management: bool,
    special_workspaces: bool,
    ...
}
```

---

## I031 — Lock screen is security-sensitive

A lock screen must never be implemented as “just another overlay”.

Before implementation, validate:

```text
session locking protocol
input exclusivity
surface lifecycle
failure behavior
authentication boundary
```

Until those requirements are satisfied, lockscreen functionality is considered incomplete.

---

## I032 — Shell crashes must never be used as control flow

No production path may rely on:

```text
unwrap()
expect()
panic!()
```

for recoverable:

```text
IPC
D-Bus
Wayland
config
filesystem
device
service
```

failures.

Assertions are acceptable for genuine internal invariants.

---

## I033 — Configuration loading must be recoverable

Invalid user configuration must not prevent the shell from starting.

Expected behavior:

```text
load config
   ↓
validate
   ↓
invalid field
   ↓
fallback/default + diagnostic
```

Corrupt configuration should be surfaced clearly without destroying desktop usability.

---

## I034 — Feature modules do not own infrastructure clients

A feature such as `panel`, `notch`, or `launcher` must not instantiate:

```text
Hyprland connection
D-Bus connection
PipeWire connection
NetworkManager client
```

Infrastructure clients belong to adapters/services with application-level interfaces.

---

## I035 — Services are shared, not duplicated per widget

A single system service should normally serve all consumers.

Example:

```text
AudioService
├── panel volume widget
├── OSD
└── settings
```

Not:

```text
3 independent PipeWire clients
```

unless isolation is deliberately required.

---

## I036 — UI features are composable

Panel, notch, launcher, dock, notifications, and OSD must not require each other internally unless the product behavior explicitly demands it.

Example:

```text
Launcher
```

may be presented inside the notch, but launcher application logic should not require notch rendering code.

---

## I037 — Ambxst is reference material, not an architectural authority

When Ambxst and this project's constraints conflict, prefer the architecture appropriate to Rust, GPUI, Wayland, and Hyprland.

Do not reproduce:

```text
QML-specific state patterns
Quickshell-specific surface patterns
backend separation
module boundaries
```

without validating that they solve the same problem here.

---

## I038 — Proof-of-concept failures must change decisions

PoCs exist to invalidate assumptions.

If a PoC proves that a selected approach fails a hard requirement, the architecture must change.

Do not preserve a decision merely because implementation has already started.

---

## I039 — Performance claims require measurement

Do not optimize or reject a technology based only on assumptions about:

```text
RAM
CPU
binary size
GPU usage
latency
frame time
startup time
```

Measure first.

Relevant metrics should include:

```text
cold startup
idle CPU
idle RAM
frame time
input latency
surface resize latency
service event latency
```

---

## I040 — Correctness beats architectural purity

When invariants appear to conflict, priorities are:

```text
1. correctness
2. Wayland/compositor compatibility
3. security
4. architectural boundaries
5. responsiveness
6. maintainability
7. performance
8. implementation speed
9. theoretical portability
10. dependency purity
```

Any deliberate violation must be documented in `decisions.md`.

---

# Review checklist

Before merging a feature, verify:

```text
[ ] GPUI did not leak into core/platform/service crates
[ ] Hyprland-specific types remain inside the adapter
[ ] no widget executes Linux commands directly
[ ] runtime state and persistent config remain separated
[ ] output ownership is explicit
[ ] fractional scaling assumptions were avoided
[ ] focus/input changes go through the coordinator
[ ] external state has one authoritative service
[ ] event APIs are used where available
[ ] optional failures do not crash the shell
[ ] no unnecessary dependency was introduced
[ ] async work does not mutate UI arbitrarily
[ ] surface ownership/lifetime is explicit
[ ] error handling is recoverable
[ ] structured logging is used
[ ] behavior was tested under monitor/workspace/window churn
```

---

# Core invariant

The single most important rule is:

```text
The shell owns the architecture.
GPUI, Hyprland and Linux services are adapters around it.
```

If a design makes the core depend structurally on one of those adapters, the boundary is wrong.
