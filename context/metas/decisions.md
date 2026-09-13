# decisions.md

# Architectural Decisions — Linux Shell UI

This file records the main architectural decisions for the project.

The goal is to avoid repeatedly reopening settled questions and to keep implementation choices explicit, reviewable, and replaceable when new evidence appears.

---

## Decision format

Each decision contains:

- **Status**: proposed, accepted, superseded, rejected.
- **Context**: why the decision exists.
- **Decision**: what the project will do.
- **Consequences**: trade-offs introduced by the decision.
- **Revisit when**: concrete conditions that justify reopening it.

---

## D001 — Rust is the primary implementation language

**Status:** Accepted

### Context

The shell needs tight integration with Linux, Wayland, Hyprland IPC, system services, rendering, concurrency, and low-level platform APIs.

### Decision

Use Rust as the primary language for:

- application core;
- compositor integrations;
- system services;
- configuration;
- IPC;
- geometry/math helpers;
- UI infrastructure;
- GPUI frontend;
- future custom renderer.

Avoid introducing C++ unless a required dependency forces it.

### Consequences

Positive:

- one primary language across the stack;
- strong type and memory safety;
- easier sharing of types across UI, services, and platform adapters;
- good fit for async, IPC, Wayland, and systems integration.

Negative:

- some Linux APIs will still require bindings to C libraries;
- Rust-native alternatives are not always mature enough.

### Revisit when

A critical subsystem has no viable Rust API or binding strategy.

---

## D002 — GPUI is the initial UI toolkit

**Status:** Accepted

### Context

Building a complete retained UI system directly on Wayland + wgpu would dramatically increase scope before validating the shell itself.

### Decision

Use GPUI as the initial presentation layer.

GPUI is treated as an implementation detail of the frontend, not as part of the domain model.

### Consequences

Positive:

- faster development of components and interaction;
- avoids building text, layout, focus, widgets, and animation infrastructure immediately;
- remains entirely within Rust.

Negative:

- GPUI may expose limitations for desktop-shell-specific Wayland behavior;
- some visual code will need rewriting if the frontend is replaced.

### Revisit when

Any of the following becomes a blocker:

- layer-shell integration;
- input-region control;
- multi-monitor behavior;
- fractional scaling;
- custom rendering;
- animation performance;
- unusual Wayland surfaces.

---

## D003 — GPUI must be replaceable

**Status:** Accepted

### Context

The long-term project may require direct Wayland + custom rendering to gain full control.

### Decision

Do not allow GPUI types to cross into the application core, domain state, compositor layer, or Linux service adapters.

The dependency direction is:

```text
shell-ui-gpui
      ↓
 shell-core
```

Never:

```text
shell-core
      ↓
    GPUI
```

### Consequences

The application can later gain a frontend such as:

```text
wayland-client
+ Smithay Client Toolkit
+ wgpu
+ custom UI/rendering
```

without replacing the core.

The GPUI visual layer itself is not expected to migrate automatically.

### Revisit when

Never as a default architectural rule. It may only be superseded by an explicit architecture decision.

---

## D004 — Use hexagonal boundaries around platform integrations

**Status:** Accepted

### Context

Hyprland, PipeWire, NetworkManager, BlueZ, UPower, and similar APIs are infrastructure, not application logic.

### Decision

Define application-facing ports and implement Linux/compositor-specific adapters behind them.

Examples:

```text
CompositorPort
AudioPort
NetworkPort
BluetoothPort
PowerPort
NotificationPort
MediaPort
ConfigPort
```

### Consequences

Platform-specific code stays isolated.

Mocks and fake implementations can be used during UI development and testing.

### Revisit when

A port becomes a meaningless abstraction around a single trivial function. Do not abstract mechanically.

---

## D005 — Hyprland is an adapter, not the shell architecture

**Status:** Accepted

### Context

Hyprland is the initial compositor, but shell state should not be modeled around Hyprland-specific JSON or IPC commands.

### Decision

Translate Hyprland IPC/events into internal types.

Example:

```text
Hyprland workspace event
          ↓
Hyprland adapter
          ↓
WorkspaceChanged
          ↓
ShellState
```

### Consequences

Future compositor support can be implemented independently.

Likely future adapters:

- Niri;
- Sway;
- other wlroots-compatible compositors.

### Revisit when

The project intentionally becomes Hyprland-exclusive.

---

## D006 — Wayland surface semantics are separated from widgets

**Status:** Accepted

### Context

A shell needs concepts not present in ordinary desktop applications:

- layer-shell;
- anchors;
- exclusive zones;
- keyboard interactivity;
- input regions;
- per-output surfaces.

### Decision

Represent those concepts independently from GPUI widgets.

Example:

```rust
struct SurfaceSpec {
    layer: ShellLayer,
    anchors: Anchors,
    exclusive_zone: Option<i32>,
    keyboard: KeyboardMode,
    input_region: InputRegion,
}
```

### Consequences

Surface management can later be implemented by GPUI-specific or low-level Wayland adapters.

### Revisit when

GPUI provides a stable first-class shell abstraction that cleanly covers all required semantics.

---

## D007 — Validate fullscreen unified panel vs independent surfaces with a PoC

**Status:** Proposed

### Context

Ambxst uses a fullscreen transparent `PanelWindow` per monitor containing bar, notch, dock, and related shell content.

That architecture works well with Quickshell, but may not be optimal for GPUI.

### Decision

Do not commit yet to one surface topology.

Test both:

### Option A

```text
one fullscreen transparent shell surface per output
├── panel
├── notch
├── dock
└── overlays
```

### Option B

```text
independent layer-shell surfaces
├── panel
├── notch
├── dock
└── overlays
```

### Evaluation criteria

- input regions;
- click-through behavior;
- focus;
- animations;
- exclusive zones;
- multi-monitor;
- surface resizing;
- compositor behavior.

### Revisit when

Layer-shell PoC is complete.

---

## D008 — Multi-monitor is a first-class concern

**Status:** Accepted

### Context

Shells cannot treat the desktop as a single implicit screen.

Ambxst creates UI instances per output.

### Decision

Maintain explicit output state.

```rust
struct OutputState {
    id: OutputId,
    geometry: Rect,
    scale: f32,
    panel: PanelState,
    notch: NotchState,
    dock: DockState,
}
```

### Consequences

Per-monitor placement, scale, focus, surfaces, and configuration can be handled correctly.

### Revisit when

Never while multi-monitor support remains a project requirement.

---

## D009 — Persistent configuration and runtime state are separate

**Status:** Accepted

### Context

Theme settings and panel configuration are fundamentally different from temporary state such as launcher visibility or active workspace.

### Decision

Maintain separate concepts:

```text
ShellConfig
```

for persisted user configuration, and:

```text
ShellState
```

for runtime state.

### Consequences

Configuration serialization remains clean and runtime state is not accidentally persisted.

### Revisit when

Never without a strong reason.

---

## D010 — Use unidirectional state flow

**Status:** Accepted

### Context

Directly wiring widgets to Linux services creates hidden dependencies and inconsistent state.

### Decision

External events follow:

```text
Linux / Hyprland
      ↓
adapter
      ↓
domain/application event
      ↓
ShellState
      ↓
UI render
```

User interactions follow:

```text
UI intent
   ↓
application command
   ↓
port
   ↓
adapter
   ↓
system
```

### Consequences

UI stays declarative and easier to test.

### Revisit when

Specific high-frequency rendering paths prove too expensive and require specialized direct channels.

---

## D011 — Widgets must not execute shell commands directly

**Status:** Accepted

### Context

Calling tools such as `wpctl`, `nmcli`, or `hyprctl` directly inside UI callbacks couples presentation to system implementation.

### Decision

Forbidden pattern:

```rust
Button::on_click(|| {
    Command::new("wpctl");
});
```

Required pattern:

```text
button
  ↓
AudioCommand
  ↓
AudioPort
  ↓
PipeWire adapter
```

CLI processes may still be used inside adapters as temporary implementations.

### Consequences

Implementation details can be replaced without changing UI code.

### Revisit when

Never as a general rule.

---

## D012 — Prefer native protocols/APIs over CLI subprocesses

**Status:** Accepted

### Context

CLI polling and command spawning are easy to prototype but introduce latency, parsing fragility, and process overhead.

### Decision

Preferred integrations:

```text
D-Bus       → zbus
Hyprland    → IPC sockets
Wayland     → native Wayland protocols
Audio       → PipeWire API/bindings
```

CLI tools are acceptable for early PoCs when they significantly reduce implementation time.

### Consequences

Production integrations will generally be more robust and event-driven.

### Revisit when

The native interface is substantially less reliable or maintainable than the system CLI.

---

## D013 — Do not force “pure Rust” where mature C libraries are the correct dependency

**Status:** Accepted

### Context

Replacing every C/C++ dependency purely for language purity would increase maintenance and risk.

### Decision

Prefer Rust-native libraries when mature, but allow bindings where Linux infrastructure is fundamentally built around an existing native library.

Use Rust as the ownership/integration layer even when the underlying implementation is C.

### Consequences

The project remains pragmatic rather than ideological.

### Revisit when

A stable Rust-native implementation becomes clearly superior.

---

## D014 — No separate backend daemon by default

**Status:** Accepted

### Context

Ambxst contains a separate backend, but this project already uses a systems language for the shell process.

### Decision

Keep initial architecture in one process:

```text
shell
├── GPUI
├── core
├── Hyprland adapter
└── Linux services
```

Create separate daemons only for concrete reasons:

- privilege separation;
- crash isolation;
- independent lifecycle;
- external IPC;
- security boundaries.

### Consequences

Simpler deployment and IPC model.

### Revisit when

A subsystem demonstrably benefits from process isolation.

---

## D015 — Notch is a shell feature, not a special platform primitive

**Status:** Accepted

### Context

Ambxst builds its notch from ordinary layout, custom geometry, masks, and animation.

### Decision

Model the notch as application/UI state:

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

and render it through GPUI/custom geometry.

Wayland only provides the surface on which it exists.

### Consequences

The notch remains portable across rendering backends.

### Revisit when

Never unless compositor-specific behavior becomes required.

---

## D016 — Implement notch geometry as a reusable primitive

**Status:** Accepted

### Context

The Ambxst notch uses concave side corners rather than only standard border radius.

### Decision

Create a reusable shape abstraction:

```rust
struct NotchGeometry {
    width: f32,
    height: f32,
    radius: f32,
    corner_size: f32,
    edge: Edge,
}
```

Render using the best GPUI-supported mechanism:

- path;
- custom paint;
- tessellation;
- shader.

### Consequences

Geometry remains independent of launcher/dashboard content.

### Revisit when

The final design no longer uses a notch-shaped container.

---

## D017 — Centralize focus, modal, and input-region coordination

**Status:** Accepted

### Context

Desktop shell surfaces have overlapping ownership of keyboard focus, pointer input, dismiss behavior, and modal state.

### Decision

Create a central coordinator responsible for:

- active surface;
- keyboard interactivity;
- click-outside behavior;
- input-region expansion;
- modal ownership;
- popup ordering.

Widgets request behavior; they do not directly manipulate Wayland focus.

### Consequences

Avoids multiple features fighting over focus and input.

### Revisit when

Never while multiple interactive surfaces exist.

---

## D018 — Create a shared design system

**Status:** Accepted

### Context

A shell contains many small components that must remain visually consistent.

### Decision

Centralize:

```text
colors
typography
spacing
radius
shadows
motion
icons
```

Do not scatter styling constants throughout feature modules.

### Consequences

Theme changes become manageable and consistent.

### Revisit when

Never as a general design rule.

---

## D019 — Build reusable UI primitives before duplicating feature styling

**Status:** Accepted

### Context

Ambxst benefits from shared primitives such as styled rectangles, sliders, toggles, popups, and search inputs.

### Decision

Create reusable primitives for recurring patterns.

Example structure:

```text
ui/
├── primitives/
├── components/
└── features/
```

Do not create a complete generic UI framework.

### Consequences

Less styling duplication without prematurely building a toolkit.

### Revisit when

A primitive is used only once and abstraction adds more complexity than value.

---

## D020 — Avoid a fake toolkit abstraction over GPUI

**Status:** Accepted

### Context

Trying to wrap every GPUI element behind custom `Button`, `Div`, `Flex`, etc. solely to make migration theoretically automatic would recreate a UI toolkit badly.

### Decision

Allow GPUI-specific code inside `shell-ui-gpui`.

Protect architectural boundaries around state, services, surfaces, and domain logic instead.

### Consequences

A future renderer migration requires rewriting visual components, but avoids maintaining a redundant abstraction layer today.

### Revisit when

The project intentionally begins building its own UI framework.

---

## D021 — Async runtime must not own the rendering architecture

**Status:** Accepted

### Context

System integrations need asynchronous IO, while GPUI owns UI execution and rendering.

### Decision

Use async for services and adapters, likely via Tokio where appropriate, but bridge results into application state through controlled channels/events.

Do not allow arbitrary async tasks to mutate UI state directly.

### Consequences

Clear boundary between background IO and presentation.

### Revisit when

GPUI provides a better integrated runtime model that preserves the same isolation.

---

## D022 — Event-driven integrations are preferred over polling

**Status:** Accepted

### Context

Most compositor and Linux services provide subscriptions, sockets, signals, or event streams.

### Decision

Prefer:

- Hyprland event socket;
- D-Bus signals;
- PipeWire events;
- Wayland callbacks.

Use polling only where no event API exists or where polling is demonstrably simpler and inexpensive.

### Consequences

Lower idle CPU usage and faster state updates.

### Revisit when

An event interface proves unreliable.

---

## D023 — Crate boundaries represent architectural boundaries, not file organization

**Status:** Accepted

### Context

A workspace with dozens of tiny crates creates compile and maintenance overhead.

### Decision

Begin with a limited set of meaningful crates, likely:

```text
shell-core
shell-platform
shell-hyprland
shell-linux
shell-config
shell-theme
shell-ui-gpui
shell-app
```

Split further only when ownership, dependency, testing, or reuse justifies it.

### Consequences

The architecture remains modular without crate explosion.

### Revisit when

A module becomes independently reusable or dependency isolation becomes valuable.

---

## D024 — Use Ambxst as a behavioral reference, not a port target

**Status:** Accepted

### Context

Ambxst is built around Quickshell/QML and its architecture reflects that environment.

### Decision

Reuse concepts such as:

- per-output composition;
- unified shell experience;
- dynamic notch;
- service separation;
- central visibility/focus;
- shared UI primitives;
- persistent configuration;
- reactive state.

Do not translate QML files one-to-one into Rust.

### Consequences

The new project can retain useful UX ideas while adopting architecture suited to Rust and Wayland.

### Revisit when

Never. Literal porting is explicitly not the project goal.

---

# Proof-of-concept gates

Major architecture decisions must be validated in this order.

## Gate 1 — GPUI + Hyprland Layer Shell

Must prove:

- transparent surface;
- `top` and `overlay` layers;
- anchors;
- exclusive zone;
- keyboard interactivity;
- pointer input;
- multi-monitor;
- fractional scaling.

Failure here triggers reevaluation of **D002**.

---

## Gate 2 — Surface topology

Compare:

```text
fullscreen unified surface
```

against:

```text
independent layer surfaces
```

Resolve **D007**.

---

## Gate 3 — Custom notch

Must prove:

- concave geometry;
- smooth resize animation;
- reliable hit testing;
- click-through outside the shape;
- launcher expansion;
- focus acquisition/release.

---

## Gate 4 — Hyprland state adapter

Must prove event-driven handling of:

- monitors;
- workspaces;
- focused workspace;
- focused window;
- fullscreen state.

---

## Gate 5 — Linux service adapters

Initial services:

```text
audio
battery
network
MPRIS
```

The frontend must consume only application-level state and commands.

---

# Current stack candidates

These are candidates, not all final decisions.

```text
Language               Rust

UI                     GPUI

Compositor             Hyprland

Wayland                wayland-client
                       Smithay Client Toolkit where needed
                       wayland-protocols / wlr protocols

Async                   Tokio where appropriate

D-Bus                   zbus

Serialization           serde

Configuration           TOML + serde

Geometry/math           glam / euclid depending on requirements

2D paths                lyon

SVG                     resvg / usvg

Text                    GPUI initially
                       cosmic-text / swash candidates for custom renderer

Images                  image

Logging                 tracing

Error handling          thiserror
                       anyhow at application boundaries
```

Do not introduce a dependency solely because it appears in this list. Dependencies require an actual use case.

---

# Open decisions

These remain intentionally unresolved.

## O001 — Exact GPUI layer-shell implementation

Need to validate the current GPUI Linux backend and determine whether upstream functionality is sufficient or whether a small platform patch/fork is necessary.

## O002 — Unified vs independent Wayland surfaces

Pending PoC.

## O003 — Renderer after GPUI

No decision should be made until GPUI becomes a measurable constraint.

Possible future stack:

```text
SCTK
+ wayland-client
+ wgpu
+ lyon
+ custom text stack
```

## O004 — Audio backend

Need to compare direct PipeWire integration against practical Rust wrappers/bindings.

## O005 — System tray implementation

Need to define StatusNotifierItem / DBusMenu approach and compatibility requirements.

## O006 — Notification daemon ownership

Decide whether the shell itself implements the notification server or consumes another daemon.

## O007 — Lockscreen

Security-sensitive. Must be designed separately rather than treated as a normal overlay.

---

# Decision priority

When decisions conflict, optimize in this order:

```text
1. correctness
2. Wayland/compositor compatibility
3. architectural boundaries
4. user-perceived responsiveness
5. maintainability
6. implementation speed
7. theoretical portability
8. dependency purity
```

---

# Core rule

The project is not:

```text
a GPUI application that happens to behave like a shell
```

It is:

```text
a Linux shell whose first presentation adapter is GPUI
```

That distinction must remain visible in every architectural decision.
