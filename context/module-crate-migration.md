# Module Crate Migration Plan

## Status

Planned architecture change for the Luna workspace.

This document describes the code changes required to move from feature modules living inside the UI crate to **independent Rust crates** registered into the Notch host.

Canonical module documentation lives in [modules/README.md](./modules/README.md).

---

# Target architecture

```text
crates/
├── shell-app/
├── shell-core/
├── shell-platform/
├── shell-hyprland/
├── shell-linux/
├── shell-config/
├── shell-theme/
├── shell-ui-gpui/
│   ├── notch/
│   ├── primitives/
│   ├── animation/
│   └── module_host/
│
└── modules/
    ├── launcher/
    ├── calendar/
    ├── player/
    ├── theme/
    ├── resources/
    ├── clock/
    └── settings/
```

Package names:

```text
luna-module-launcher
luna-module-calendar
luna-module-player
luna-module-theme
luna-module-resources
luna-module-clock
luna-module-settings
```

The modules remain statically linked into the Luna binary. A crate boundary does **not** imply one process per module or runtime dynamic loading.

---

# Dependency direction

Required dependency direction:

```text
shell-core ───────────────┐
shell-linux ──────────────┤
shell-config ─────────────┤
shell-theme ──────────────┤
                          ▼
                    module crates
                          │
                          │ implement host contract
                          ▼
                    shell-ui-gpui
                          ▲
                          │ composition
                          │
                       shell-app
```

In practical Cargo terms, modules may depend on `shell-ui-gpui` for the GPUI module-host API and shared primitives. `shell-ui-gpui` must **not** depend on concrete module crates. `shell-app` depends on both and performs registration.

This prevents:

```text
shell-ui-gpui -> launcher -> shell-ui-gpui
```

from becoming a dependency cycle.

---

# Phase 1 — Workspace restructuring

## 1.1 Add module directory

Create:

```text
crates/modules/
```

## 1.2 Add initial crates

Create:

```text
crates/modules/launcher
crates/modules/calendar
crates/modules/player
crates/modules/theme
crates/modules/resources
crates/modules/clock
crates/modules/settings
```

Each crate starts with:

```text
Cargo.toml
src/lib.rs
```

## 1.3 Update workspace members

The root workspace must include module crates, preferably through a stable workspace pattern such as:

```toml
[workspace]
members = [
    "crates/*",
    "crates/modules/*",
]
```

If `crates/*` would incorrectly include the `modules` directory as a crate, use explicit infrastructure members plus `crates/modules/*`.

## 1.4 Workspace dependency policy

Shared dependency versions should stay at workspace level where appropriate:

```text
gpui
serde
tracing
thiserror
tokio
```

Module crates should use workspace dependencies instead of introducing conflicting versions.

---

# Phase 2 — Module host contract

Create a module host boundary under `shell-ui-gpui`:

```text
shell-ui-gpui/src/module_host/
├── mod.rs
├── module.rs
├── registry.rs
└── context.rs
```

Conceptually:

```rust
pub trait NotchModule {
    fn id(&self) -> ModuleId;
    fn title(&self) -> &'static str;
    fn render(&mut self, cx: &mut ModuleContext) -> ModuleView;
}
```

The exact GPUI types should be selected during implementation; the architectural requirements are:

```text
stable module identity
render entry point
controlled access to shell/application context
no direct Wayland ownership
no direct raw compositor commands
```

Do not make the first version into a general plugin ABI. This is an internal Rust interface.

---

# Phase 3 — Module identity and routing

Replace a permanently enumerated set of Notch-specific routes such as:

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

with a module-oriented route model:

```rust
enum ModuleId {
    Launcher,
    Calendar,
    Player,
    Theme,
    Resources,
    Clock,
    Settings,
}

enum NotchRoute {
    Idle,
    Module(ModuleId),
}
```

A later implementation may replace the enum with a validated stable identifier if external modules become a real requirement. Do not introduce that complexity now.

---

# Phase 4 — Module registry

`ModuleRegistry` belongs to the UI composition layer.

Conceptually:

```rust
pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn NotchModule>>,
}
```

Required operations:

```text
register
lookup
activate
iterate metadata
```

Registration happens from `shell-app`:

```text
shell-app
├── create shell services
├── create GPUI host
├── instantiate modules
├── register modules
└── start shell
```

The Notch host knows only the module contract and registry, not concrete types such as `LauncherModule`.

---

# Phase 5 — Notch sizing contract

The Notch remains responsible for its own geometry and animation.

A module may provide layout constraints or a preferred content size, but must not directly manipulate the Wayland surface.

Flow:

```text
active module
     ↓
module content/layout
     ↓
measured/preferred size
     ↓
Notch target geometry
     ↓
transition/animation
     ↓
Wayland input region + rendered shell
```

Required invariants:

```text
module does not set layer-shell anchors
module does not resize Wayland surfaces directly
module does not own click-outside behavior
Notch owns transition between module sizes
```

---

# Phase 6 — Shared primitives

Keep cross-module GPUI primitives in `shell-ui-gpui`:

```text
button
icon button
text
surface
search input
slider
toggle
scroll container
list item
tooltip
```

Do not copy these into every module crate.

A module crate owns feature-specific components only.

Example:

```text
luna-module-player
├── PlayerModule
├── TrackMetadata
└── PlayerControls
```

while generic `IconButton` stays in `shell-ui-gpui`.

---

# Phase 7 — Service access

Modules consume system state through existing core/service contracts.

Examples:

```text
Player
→ MediaPort / MPRIS adapter

Resources
→ resource metrics service

Launcher
→ application discovery/execution service

Theme
→ configuration/theme service

Settings
→ ConfigPort + supported system service ports
```

Forbidden inside module widgets:

```text
Command::new("hyprctl")
Command::new("wpctl")
raw D-Bus connection creation
raw PipeWire client creation
raw Hyprland socket handling
```

Shared infrastructure clients are owned by adapters/services and injected through application context.

---

# Phase 8 — Initial module extraction order

Recommended extraction order:

```text
1. Clock
2. Launcher
3. Player
4. Calendar
5. Resources
6. Theme
7. Settings
```

Rationale:

- **Clock** validates the crate/host boundary with almost no infrastructure dependency.
- **Launcher** validates keyboard input, search, variable Notch sizing and application actions.
- **Player** validates event-driven system services through MPRIS.
- **Calendar** validates richer content/layout without heavy system integration.
- **Resources** validates streaming metrics and bounded refresh policy.
- **Theme** validates live configuration propagation across crate boundaries.
- **Settings** comes last because it depends on the configuration and service contracts being mature.

---

# Phase 9 — Module-specific preparation

## Clock

Required before extraction:

```text
module host API
time update policy
theme primitives
```

Target document: [modules/clock.md](./modules/clock.md)

## Launcher

Required before extraction:

```text
application discovery
application execution service
icon resolver
keyboard/focus integration
```

Target document: [modules/launcher.md](./modules/launcher.md)

## Player

Required before extraction:

```text
MediaPort
MPRIS adapter
player event stream
```

Target document: [modules/player.md](./modules/player.md)

## Calendar

Required before extraction:

```text
time/date model
navigation state
shared grid/list primitives
```

Target document: [modules/calendar.md](./modules/calendar.md)

## Resources

Required before extraction:

```text
resource metrics service
sampling policy
bounded history
```

Target document: [modules/resources.md](./modules/resources.md)

## Theme

Required before extraction:

```text
typed Theme model
ConfigPort
config hot reload
validated theme snapshot
```

Target document: [modules/theme.md](./modules/theme.md)

## Settings

Required before extraction:

```text
stable configuration schema
write/update API
validation errors
service capability model
```

Target document: [modules/settings.md](./modules/settings.md)

---

# Phase 10 — Configuration integration

Each module can have a corresponding modular TOML section/file when needed:

```text
~/.config/luna/
├── config.toml
├── theme.toml
├── keybinds.toml
└── modules/
    ├── launcher.toml
    ├── calendar.toml
    ├── player.toml
    ├── resources.toml
    ├── clock.toml
    └── settings.toml
```

`theme.toml` remains the canonical theme configuration rather than duplicating theme state under `modules/theme.toml`.

The Theme module is the UI/controller for the shared theme system, not the owner of an independent theme configuration format.

Config reload flow remains:

```text
filesystem watch
→ debounce
→ parse complete candidate
→ validate
→ atomic snapshot replace
→ application event
→ modules rerender
```

---

# Phase 11 — Tests

Each module crate should have its own unit tests for feature logic.

The shell should additionally test module integration:

```text
registration
route activation
module replacement
Notch resizing
focus transfer
Escape/click-outside dismissal
theme propagation
service unavailable state
```

At least one test must demonstrate that adding a module does not require modifying Notch rendering internals.

---

# Phase 12 — Migration completion criteria

The architecture migration is complete when:

- all seven initial module crates exist and compile;
- `shell-ui-gpui` has no dependency on concrete module crates;
- `shell-app` registers the modules;
- Notch navigation uses module identity rather than hardcoded feature rendering branches;
- each module consumes shared services through explicit contracts;
- module-specific UI logic no longer lives under a generic `shell-ui-gpui/features/` directory;
- modules can be initialized lazily where appropriate;
- changing the active module causes the Notch to resize through the host sizing path;
- `cargo check --workspace`, tests, clippy and formatting pass.

---

# Non-goals

This migration does not introduce:

```text
dynamic shared libraries
runtime crate loading
WASM plugins
separate module processes
third-party stable plugin ABI
per-module Wayland surfaces
```

Those require separate architectural decisions if needed later.
