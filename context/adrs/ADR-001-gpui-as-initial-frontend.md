# ADR-001 — GPUI as the Initial Frontend

- **Status:** Accepted
- **Date:** 2026-09-11
- **Decision owners:** Project maintainers
- **Related decisions:** D002, D003, D006, D007, D017, D020
- **Related gates:** G01, G02, G03, G04, G07, G08, G18

---

## Context

The project is a Linux desktop shell written primarily in Rust and initially targeting Wayland + Hyprland.

The shell is expected to provide components such as:

```text
panel
notch
launcher
dock
notifications
OSD
system controls
workspace UI
media controls
```

A low-level implementation based directly on:

```text
wayland-client
+ Smithay Client Toolkit
+ wgpu
+ custom layout
+ custom text
+ custom input/focus handling
```

would provide maximum platform and rendering control, but it would significantly increase the initial scope.

Before validating the shell architecture and product behavior, the project would also need to implement or integrate substantial UI infrastructure:

```text
layout
text rendering
widgets
focus
input dispatch
scrolling
animation
component lifecycle
render invalidation
accessibility foundations
```

The project therefore needs a frontend that:

1. keeps the implementation predominantly in Rust;
2. provides a productive UI/component model;
3. supports custom rendering and animation;
4. can coexist with Linux/Wayland-specific platform integration;
5. does not become an architectural dependency of the application core;
6. can be replaced later if desktop-shell requirements exceed its capabilities.

GPUI is a strong candidate because it provides a native Rust UI framework with a rendering and component model already proven in a complex desktop application.

However, GPUI is primarily an application UI framework, not a dedicated Wayland desktop-shell toolkit.

The main technical uncertainty is therefore not ordinary UI rendering. It is whether GPUI can satisfy or be extended to satisfy shell-specific Wayland requirements such as:

```text
wlr-layer-shell
anchors
exclusive zones
keyboard interactivity
custom input regions
transparent overlay surfaces
per-output surfaces
fractional scaling
surface lifecycle under output hotplug
```

These capabilities must be validated through project gates rather than assumed.

---

## Decision

Use **GPUI as the initial presentation frontend** for the Linux shell.

GPUI is explicitly treated as a **replaceable presentation adapter**, not as part of the core architecture.

The dependency direction must remain:

```text
shell-ui-gpui
      ↓
 shell-core
```

and never:

```text
shell-core
      ↓
    GPUI
```

GPUI-specific types must remain inside the frontend/platform integration boundary.

The project will not attempt to hide all GPUI APIs behind a generic custom widget toolkit solely for theoretical portability.

Instead, portability is preserved at the architectural boundaries that matter:

```text
application state
domain types
commands
events
system service ports
compositor ports
surface semantics
renderer-independent geometry where practical
```

If GPUI is replaced later, visual components may be rewritten.

The application core, compositor integration, Linux services, configuration, and domain state should remain reusable.

---

## Decision drivers

The decision is primarily driven by:

### Development velocity

The project should validate shell behavior before investing in a custom UI engine.

GPUI provides higher-level primitives than a direct Wayland + wgpu stack.

### Rust-first implementation

Using GPUI allows the shell UI and surrounding application architecture to remain within Rust.

### Custom UI requirements

The project requires highly custom visuals such as:

```text
animated notch geometry
custom surfaces
overlays
dynamic resizing
non-standard shapes
transitions
```

A generic native widget toolkit is less attractive for this style of interface.

### Architectural exit path

GPUI can remain isolated as a presentation adapter.

The project can later migrate toward:

```text
wayland-client
+ Smithay Client Toolkit
+ wgpu
+ custom renderer/UI layer
```

without changing the core application model.

---

## Alternatives considered

### Quickshell + QML

#### Advantages

```text
excellent fit for Wayland shells
first-class layer-shell workflows
high productivity
mature QML layout/animation model
existing shell ecosystem
```

#### Disadvantages

```text
introduces Qt/QML as a second primary technology stack
less aligned with the Rust-first goal
core shell UI would live outside Rust
future custom Rust rendering path becomes less direct
```

#### Decision

Rejected as the primary frontend.

It remains a useful architectural and behavioral reference, especially through projects such as Ambxst.

---

### Direct Wayland + SCTK + wgpu

#### Advantages

```text
maximum Wayland control
maximum rendering control
minimal framework lock-in
ideal long-term platform ownership
```

#### Disadvantages

The project would immediately inherit responsibility for major UI infrastructure:

```text
layout
text
input
focus
widgets
animation
render lifecycle
accessibility
```

This would turn the initial shell project into a UI framework project.

#### Decision

Rejected for the initial implementation.

Retained as the primary fallback / long-term custom frontend option.

---

### GTK4

#### Advantages

```text
mature
Linux-native ecosystem
good accessibility
strong application toolkit
Rust bindings available
```

#### Disadvantages

The project is not primarily a conventional desktop application.

The shell requires unusual surfaces, custom visuals, and renderer-level control where GTK's application-oriented abstraction may become restrictive.

#### Decision

Rejected for the primary shell frontend.

---

### Iced

#### Advantages

```text
Rust-native
declarative model
cross-platform
established ecosystem
```

#### Disadvantages

The project would still need significant work around shell-specific Wayland behavior, and GPUI is considered a better fit for the desired custom desktop UI architecture.

#### Decision

Not selected.

May be reevaluated if GPUI fails early platform gates.

---

### egui

#### Advantages

```text
simple
fast iteration
Rust-native
excellent for tooling
```

#### Disadvantages

Immediate-mode UI is not considered the best fit for the intended desktop shell, component hierarchy, visual design, and long-lived application surfaces.

#### Decision

Rejected.

---

## Architectural constraints

The following constraints are mandatory while GPUI is used.

### GPUI isolation

GPUI must not appear in:

```text
shell-core
shell-hyprland
shell-linux
shell-config
```

Renderer-specific code belongs in:

```text
shell-ui-gpui
```

or an explicitly named GPUI platform crate/module.

---

### Surface semantics remain explicit

Shell-specific concepts must not be represented only as GPUI implementation details.

The project should maintain an internal model similar to:

```rust
struct SurfaceSpec {
    layer: ShellLayer,
    anchors: Anchors,
    exclusive_zone: Option<i32>,
    keyboard: KeyboardMode,
    input_region: InputRegion,
}
```

This represents shell/Wayland intent.

The GPUI platform layer implements that intent.

---

### Application state remains renderer-independent

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

Valid.

Example:

```rust
struct NotchState {
    gpui_window: WindowHandle,
    path: gpui::Path,
}
```

Invalid in the application core.

---

### No fake portability layer

Do not create wrappers for every GPUI primitive such as:

```text
UniversalDiv
UniversalButton
UniversalFlex
UniversalText
```

only to avoid a theoretical future rewrite.

That would amount to building a second UI toolkit on top of GPUI.

Abstraction should happen around architectural boundaries, not every visual primitive.

---

## Consequences

### Positive

The project can implement visible shell functionality much earlier.

The team can focus first on:

```text
Wayland shell integration
Hyprland events
panel behavior
notch interaction
services
multi-monitor
```

instead of first implementing a complete renderer and UI framework.

GPUI also keeps the project predominantly Rust-based.

The core remains structurally capable of supporting another frontend.

---

### Negative

Some GPUI UI code will likely be discarded if the frontend changes.

The project may require:

```text
Linux-specific GPUI extensions
a small GPUI patch
a maintained fork
custom surface integration
```

depending on upstream Wayland capabilities.

There is also a risk that application-oriented assumptions inside GPUI conflict with desktop-shell surface requirements.

---

## Risks

### R1 — Incomplete layer-shell support

If GPUI cannot expose or coexist cleanly with the required Wayland surface semantics, it may not be viable as the frontend.

**Mitigation:** G02 is a hard gate before substantial UI implementation.

---

### R2 — Input-region limitations

A fullscreen transparent shell surface is only viable if transparent/non-interactive areas can avoid blocking application input.

**Mitigation:** evaluate input regions and independent surfaces during G03.

---

### R3 — Multi-monitor lifecycle issues

Desktop shells must survive:

```text
output hotplug
output removal
different scale factors
focus migration
```

**Mitigation:** G04 must be completed before assuming the platform layer is stable.

---

### R4 — Excessive upstream patching

A small isolated Linux backend extension is acceptable.

A large permanent fork that effectively requires maintaining GPUI's platform backend is not.

**Mitigation:** measure patch surface at G18.

---

### R5 — Hidden framework coupling

Developers may gradually move application behavior into GPUI components for convenience.

**Mitigation:** enforce `invariants.md`, especially GPUI/core dependency rules.

---

## Validation gates

This ADR is accepted provisionally subject to technical validation.

### G01 — GPUI window viability

Must prove basic native Wayland rendering and input.

### G02 — Layer Shell viability

Must prove shell-specific surface behavior.

This is the main go/no-go gate for this ADR.

### G03 — Surface topology

Must determine whether GPUI works better with:

```text
one unified fullscreen surface
```

or:

```text
multiple independent shell surfaces
```

### G04 — Multi-monitor correctness

Must prove per-output lifecycle and fractional scaling behavior.

### G07/G08 — Notch

Must prove that custom geometry, resizing, animation, focus, and input behavior are viable.

### G18 — GPUI continuation decision

After MVP, reevaluate whether GPUI remains the long-term frontend.

---

## Conditions that reopen this ADR

Reopen ADR-001 if any of the following occurs:

```text
GPUI cannot reliably support layer-shell
input regions cannot be implemented correctly
exclusive zones are unreliable
per-output surfaces are unstable
fractional scaling causes unacceptable correctness issues
custom geometry/rendering is excessively constrained
animation/resizing performance is unacceptable
the project requires a large permanent GPUI fork
upstream GPUI direction diverges significantly from project needs
```

Do not reopen this ADR merely because:

```text
a custom renderer would be more technically elegant
wgpu offers more control
another toolkit has one convenient component
rewriting the UI sounds cleaner
```

There must be a concrete project constraint or measured benefit.

---

## Exit strategy

If GPUI is rejected later, the preferred migration direction is:

```text
shell-core
shell-hyprland
shell-linux
shell-config
shell-theme
      │
      │ unchanged
      ▼
new presentation/platform adapter
      │
      ├── wayland-client
      ├── Smithay Client Toolkit
      ├── wgpu
      ├── lyon or equivalent geometry
      └── dedicated text/rendering stack
```

Expected rewrite scope:

```text
GPUI components
GPUI rendering
GPUI event plumbing
GPUI-specific surface adapter
```

Expected reusable scope:

```text
domain state
application logic
commands
events
Hyprland adapter
Linux services
configuration
service ports
compositor ports
most semantic geometry/state models
```

---

## Final rationale

The project currently needs to prove that it can build a high-quality Linux shell, not prove that it can build a UI framework.

GPUI provides the shortest path toward a Rust-native shell while preserving a credible escape path.

Therefore:

```text
GPUI is accepted as the initial frontend,
but it does not own the architecture.
```
