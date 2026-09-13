# Module Crate Migration Plan — GPUI Kit

## Status

Planned architecture change for the Luna workspace.

This document describes the code changes required to move Luna feature modules into **independent Rust crates** registered into the Notch host while adopting **GPUI Kit** as the canonical GPUI dependency and component foundation for presentation code.

Canonical module documentation lives in [modules/README.md](./modules/README.md).

---

# 1. Architectural decision

The UI stack becomes:

```text
GPUI Kit
├── GPUI                    rendering / windows / elements
├── gpui-base               behavior / focus / state / infrastructure
└── gpui-component          styled reusable components
        │
        ▼
Luna presentation layer
├── shell-ui-gpui           shell host + Notch + Luna-specific UI
└── module crates           feature UI rendered inside the Notch
```

Luna should use **`gpui-kit` as the facade dependency**, rather than independently selecting unrelated GPUI, `gpui-base` and `gpui-component` versions.

GPUI Kit pins and re-exports the matching GPUI family and exposes:

```text
gpui_kit::*            → GPUI API
gpui_kit::platform     → GPUI platform API
gpui_kit::base         → gpui-base
gpui_kit::component    → gpui-component
gpui_kit::assets       → default assets/icons
```

The styled component layer is intentionally allowed in Luna. Modules may directly use and customize GPUI Kit components. Luna should only introduce wrappers when there is a repeated Luna-specific semantic or visual requirement.

Do **not** create a parallel Luna implementation of generic controls merely to avoid using GPUI Kit.

---

# 2. Target workspace

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
│   ├── shell_components/
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

The modules remain statically linked into the Luna executable. A crate boundary does **not** imply one process per module, dynamic loading or a stable third-party plugin ABI.

---

# 3. Dependency policy

## 3.1 Presentation crates

The following crates are allowed to depend on GPUI Kit:

```text
shell-ui-gpui
luna-module-launcher
luna-module-calendar
luna-module-player
luna-module-theme
luna-module-resources
luna-module-clock
luna-module-settings
```

These crates render GPUI UI and may use:

```rust
use gpui_kit::*;
use gpui_kit::base::*;
use gpui_kit::component::*;
```

Import only the submodules/types needed by each file in production code rather than relying on broad wildcard imports everywhere.

## 3.2 Non-presentation crates

These crates must **not** depend on GPUI Kit:

```text
shell-core
shell-platform
shell-hyprland
shell-linux
shell-config
shell-theme
```

They remain renderer-independent.

`shell-theme` owns Luna's semantic theme/configuration model. It must not become a GPUI Kit crate. Translation from Luna theme tokens into the active GPUI Kit theme belongs in the presentation layer.

## 3.3 Composition root

`shell-app` should not require a direct GPUI Kit dependency while `shell-ui-gpui` owns GPUI application initialization.

If ownership of `gpui_kit::application()` / `gpui_kit::init()` later moves into `shell-app`, then adding the dependency there is acceptable and must be documented as presentation composition rather than domain coupling.

---

# 4. GPUI version unification

This migration must not mix incompatible GPUI type families.

Luna currently uses direct GPUI dependencies. GPUI Kit publishes and pins its own matching GPUI family and explicitly acts as the facade for applications.

Target workspace dependency:

```toml
[workspace.dependencies]
gpui-kit = "0.6"
```

Presentation crates then use:

```toml
[dependencies]
gpui-kit.workspace = true
```

After the GPUI Kit migration is validated, remove independent GPUI dependencies from presentation crates:

```text
gpui
gpui-platform
```

and use the facade instead:

```rust
use gpui_kit::{
    App,
    Context,
    Render,
    Window,
    div,
    prelude::*,
};

use gpui_kit::layer_shell::*;
```

Application startup becomes conceptually:

```rust
gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);

    // create Luna windows/surfaces after Kit initialization
});
```

`gpui_kit::init(cx)` must run once before Luna uses GPUI Kit components.

### Mandatory compatibility check

Because Luna depends on Wayland layer-shell semantics, replacing the existing GPUI pin is not a blind dependency update.

Before deleting the current direct GPUI dependency, revalidate:

```text
Layer::Top
Layer::Overlay
anchors
exclusive zone
keyboard interactivity
transparent surfaces
output selection
input regions
fractional scaling
```

GPUI Kit re-exports the GPUI `layer_shell` API, but Luna's existing G01/G02 viability requirements still apply.

If the GPUI Kit pinned GPUI snapshot fails a required layer-shell behavior, stop the migration and resolve the frontend dependency before moving module implementation onto it.

---

# 5. Workspace restructuring

## 5.1 Add module directory

Create:

```text
crates/modules/
```

## 5.2 Add initial crates

```text
crates/modules/launcher
crates/modules/calendar
crates/modules/player
crates/modules/theme
crates/modules/resources
crates/modules/clock
crates/modules/settings
```

Each starts with:

```text
Cargo.toml
src/lib.rs
```

## 5.3 Workspace members

Prefer:

```toml
[workspace]
members = [
    "crates/shell-core",
    "crates/shell-platform",
    "crates/shell-hyprland",
    "crates/shell-linux",
    "crates/shell-config",
    "crates/shell-theme",
    "crates/shell-ui-gpui",
    "crates/shell-app",
    "crates/modules/*",
]
```

## 5.4 Shared module dependencies

Add module crate aliases at workspace level when the crates are created:

```toml
[workspace.dependencies]

luna-module-launcher = { path = "crates/modules/launcher" }
luna-module-calendar = { path = "crates/modules/calendar" }
luna-module-player = { path = "crates/modules/player" }
luna-module-theme = { path = "crates/modules/theme" }
luna-module-resources = { path = "crates/modules/resources" }
luna-module-clock = { path = "crates/modules/clock" }
luna-module-settings = { path = "crates/modules/settings" }

gpui-kit = "0.6"
```

The lockfile must remain committed so GPUI Kit / GPUI updates happen intentionally.

---

# 6. Cargo dependencies by crate

## `shell-ui-gpui`

```toml
[dependencies]
shell-core.workspace = true
shell-config.workspace = true
shell-platform.workspace = true
shell-theme.workspace = true
gpui-kit.workspace = true
```

Responsibilities using GPUI Kit:

```text
GPUI application/window integration
Notch rendering
layer-shell window configuration
focus/input coordination
module host
shared Luna shell components
GPUI Kit theme adaptation
```

## Module crates

Every initial module renders UI, therefore every initial module imports GPUI Kit.

Common baseline:

```toml
[dependencies]
shell-core.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Then add only module-specific contracts.

### Launcher

```toml
[dependencies]
shell-core.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Uses GPUI Kit components for areas such as:

```text
input/search
scrolling / virtual list
buttons
keyboard focus
empty states
icons
```

### Calendar

```toml
[dependencies]
shell-core.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Uses component/layout primitives for navigation, buttons, grid cells and popovers when required.

### Player

```toml
[dependencies]
shell-core.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Consumes media state from application/service contracts and uses GPUI Kit for playback controls, sliders, tooltips and layout.

### Theme

```toml
[dependencies]
shell-core.workspace = true
shell-config.workspace = true
shell-theme.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Uses GPUI Kit controls to edit Luna's semantic theme model. It does not replace `shell-theme`.

### Resources

```toml
[dependencies]
shell-core.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Uses GPUI Kit data/layout components where useful for CPU, memory, disk and process/resource views.

### Clock

```toml
[dependencies]
shell-core.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

Keeps feature logic small while using shared typography/layout/components.

### Settings

```toml
[dependencies]
shell-core.workspace = true
shell-config.workspace = true
shell-ui-gpui.workspace = true
gpui-kit.workspace = true
```

GPUI Kit is especially useful here for:

```text
forms
switches
selects
sliders
tabs
navigation
scroll areas
dialogs
validation feedback
```

---

# 7. GPUI Kit component policy

GPUI Kit components are first-class dependencies of Luna's presentation layer.

Preferred:

```text
GPUI Kit Button
GPUI Kit Input
GPUI Kit Slider
GPUI Kit Switch
GPUI Kit Popover
GPUI Kit Tooltip
GPUI Kit Scroll/VirtualList
GPUI Kit Dialog
GPUI Kit Tabs
```

when they satisfy the feature requirement.

Luna may:

```text
compose them
restyle them
theme them
wrap them
copy/fork an individual component when necessary
```

Do not fork the entire GPUI Kit repository merely to alter Luna styling.

A Luna wrapper/component is justified when it introduces a stable product-level semantic, for example:

```text
NotchModuleHeader
ShellSettingRow
ShellIconButton
ResourceMetricCard
LauncherResultRow
```

A wrapper is **not** required merely to rename a generic GPUI Kit `Button` or `Slider`.

---

# 8. Theme integration

Luna keeps one canonical semantic theme model:

```text
shell-theme
        ↓
DesignTokens / ThemeConfig
        ↓
presentation adapter
        ↓
GPUI Kit theme
        ↓
Notch + modules
```

`theme.toml` remains the persistent source of Luna theme configuration.

The Theme module edits that model through `ConfigPort` / typed configuration APIs.

On successful configuration reload:

```text
theme.toml change
→ ConfigLoader
→ validated ThemeConfig
→ shell-theme tokens
→ GPUI Kit theme adapter
→ cx.notify / affected entities
→ modules rerender
```

Do not let individual module crates create independent color systems.

Modules may use GPUI Kit semantic theme APIs and Luna-specific tokens exposed through the presentation context.

---

# 9. Module host contract

Create the host boundary under:

```text
shell-ui-gpui/src/module_host/
├── mod.rs
├── module.rs
├── registry.rs
└── context.rs
```

The module interface is intentionally a **presentation boundary**, so GPUI Kit / GPUI types are allowed here.

Avoid pretending this interface is renderer-independent.

Conceptually:

```rust
pub trait NotchModule {
    fn id(&self) -> ModuleId;
    fn title(&self) -> &'static str;

    // Exact signature must use an object-safe GPUI representation.
    fn render(&mut self, ctx: &mut ModuleContext) -> ModuleElement;
}
```

`ModuleElement` should be implemented using a GPUI object-safe/type-erased element representation rather than `impl IntoElement` on a trait object.

The exact type is selected during implementation after validating the GPUI Kit API.

Required properties:

```text
stable module identity
GPUI-native render entry point
controlled application/service context
access to Luna/GPUI Kit theme
no direct Wayland ownership
no raw compositor commands
```

Do not turn this into a public plugin ABI yet.

---

# 10. Module identity and routing

Use:

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

Do not permanently hardcode module rendering branches inside the Notch implementation.

The route selects a registered module; the module supplies content; the Notch remains the visual host.

---

# 11. Module registry

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

Registration happens in `shell-app`:

```text
shell-app
├── create core/services
├── create shell-ui-gpui frontend
├── instantiate module crates
├── register modules
└── start shell
```

Dependency direction remains:

```text
shell-ui-gpui ───────────────┐
shell-core / services ───────┤
                             ▼
                       module crates
                             ▲
                             │
                          shell-app
```

`shell-ui-gpui` must never import concrete module crates.

---

# 12. Notch sizing contract

The Notch owns geometry and animation.

```text
active module
     ↓
GPUI Kit / module layout
     ↓
measured content size
     ↓
Notch target geometry
     ↓
transition / animation
     ↓
Wayland surface + input region
```

Required invariants:

```text
module does not set layer-shell anchors
module does not resize the Wayland surface directly
module does not own click-outside behavior
module does not own global focus policy
Notch owns transitions between module sizes
```

GPUI Kit layout/components determine content layout; shell-ui-gpui translates measured layout into Notch geometry.

---

# 13. Shared UI ownership

With GPUI Kit available, `shell-ui-gpui` should not recreate a generic widget library.

Keep only Luna-specific cross-module UI here:

```text
Notch shell
Notch header/container
module chrome
shell-specific surface treatments
focus/input integration
module host
Luna theme adapter
Luna-specific repeated components
```

Generic controls should normally come directly from GPUI Kit.

Example:

```text
luna-module-player
├── PlayerModule
├── TrackMetadata       Luna feature component
├── PlayerControls      Luna feature component
└── GPUI Kit Button / Slider / Tooltip
```

---

# 14. Service access

GPUI Kit does not change service boundaries.

Modules consume system state through core/application ports:

```text
Player
→ MediaPort / MPRIS adapter

Resources
→ resource metrics service

Launcher
→ application discovery/execution service

Theme
→ ConfigPort + shell-theme

Settings
→ ConfigPort + supported service capabilities
```

Forbidden inside module UI:

```text
Command::new("hyprctl")
Command::new("wpctl")
raw D-Bus connection creation
raw PipeWire client creation
raw Hyprland socket handling
```

A GPUI Kit component may emit an intent; infrastructure execution stays behind Luna ports.

Example:

```text
GPUI Kit Slider
      ↓
VolumeChanged intent
      ↓
AudioPort
      ↓
PipeWire adapter
```

---

# 15. Initial module extraction order

Recommended order:

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

- **Clock** validates GPUI Kit + crate + host integration with little infrastructure.
- **Launcher** validates GPUI Kit input, focus, lists, keyboard navigation and variable Notch sizing.
- **Player** validates event-driven Linux services plus reusable controls.
- **Calendar** validates richer layout/navigation.
- **Resources** validates streaming metrics and data visualization/layout.
- **Theme** validates live theme propagation into GPUI Kit and every module.
- **Settings** validates a component-heavy form/navigation surface after configuration contracts mature.

---

# 16. Module-specific preparation

## Clock

Required:

```text
module host API
GPUI Kit initialized
time update policy
theme access
```

Target: [modules/clock.md](./modules/clock.md)

## Launcher

Required:

```text
application discovery
application execution service
icon resolver
GPUI Kit input/focus/list behavior
```

Target: [modules/launcher.md](./modules/launcher.md)

## Player

Required:

```text
MediaPort
MPRIS adapter
player event stream
GPUI Kit controls
```

Target: [modules/player.md](./modules/player.md)

## Calendar

Required:

```text
time/date model
navigation state
GPUI Kit layout/buttons/popovers where useful
```

Target: [modules/calendar.md](./modules/calendar.md)

## Resources

Required:

```text
resource metrics service
sampling policy
bounded history
GPUI Kit data/layout primitives
```

Target: [modules/resources.md](./modules/resources.md)

## Theme

Required:

```text
typed Luna Theme model
ConfigPort
config hot reload
GPUI Kit theme adapter
validated theme snapshot
```

Target: [modules/theme.md](./modules/theme.md)

## Settings

Required:

```text
stable configuration schema
write/update API
validation errors
service capability model
GPUI Kit form/navigation components
```

Target: [modules/settings.md](./modules/settings.md)

---

# 17. Configuration integration

Module configuration remains modular TOML:

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

`theme.toml` remains canonical rather than introducing `modules/theme.toml` as a second theme store.

Hot reload remains:

```text
filesystem watch
→ debounce
→ parse complete candidate
→ validate
→ atomic snapshot replace
→ application event
→ update GPUI Kit/Luna theme when needed
→ module rerender
```

GPUI Kit must never read Luna config files directly.

---

# 18. Tests and validation

Each module crate owns feature logic tests.

Integration tests cover:

```text
GPUI Kit initialization
module registration
route activation
module replacement
Notch resizing
focus transfer
Escape / click-outside dismissal
theme propagation
config hot reload
service unavailable state
```

The GPUI Kit migration additionally requires re-running the existing frontend viability tests for:

```text
Wayland layer-shell
transparent windows
focus
keyboard interactivity
multi-monitor
fractional scaling
```

At least one test must demonstrate that adding a module does not require editing Notch rendering internals.

---

# 19. Migration sequence

Use this order to avoid mixing incompatible GPUI types:

```text
1. Add gpui-kit at workspace level
2. Validate GPUI Kit layer-shell support in an isolated PoC
3. Move shell-ui-gpui imports to gpui_kit::* / gpui_kit::platform
4. Initialize gpui_kit once in the frontend startup path
5. Remove direct GPUI / gpui-platform dependencies
6. Verify G01/G02 again
7. Add module host contract
8. Create module crates with gpui-kit.workspace = true
9. Extract Clock
10. Extract remaining modules in planned order
11. Integrate Luna theme → GPUI Kit theme
12. Complete module/config/service tests
```

Do not leave two unrelated GPUI families active in Luna presentation crates after the migration is complete.

---

# 20. Completion criteria

The migration is complete when:

- `gpui-kit` is the canonical GPUI dependency for Luna presentation code;
- direct GPUI dependencies are removed from presentation crates unless a documented compatibility exception exists;
- GPUI Kit is initialized exactly once before component use;
- all seven initial module crates exist and compile;
- all seven module crates depend on `gpui-kit` through the workspace;
- `shell-ui-gpui` depends on GPUI Kit but has no dependency on concrete module crates;
- `shell-core`, `shell-platform`, `shell-linux`, `shell-hyprland`, `shell-config` and `shell-theme` do not depend on GPUI Kit;
- `shell-app` registers modules;
- Notch navigation uses module identity rather than hardcoded feature branches;
- modules use GPUI Kit components directly where appropriate;
- Luna-specific wrappers exist only for repeated product semantics or styling;
- modules consume infrastructure through explicit Luna ports;
- changing active modules resizes the Notch through the common host path;
- Luna theme/config changes propagate into GPUI Kit and registered modules;
- G01/G02 still pass with the GPUI version supplied by GPUI Kit;
- `cargo check --workspace`, tests, Clippy and formatting pass.

---

# 21. Non-goals

This migration does not introduce:

```text
dynamic shared libraries
runtime crate loading
WASM plugins
JavaScript gpui-shell plugins
separate module processes
third-party stable plugin ABI
per-module Wayland surfaces
a second generic Luna widget toolkit over GPUI Kit
```

`gpui-shell` is not required for the initial Luna module architecture. Luna modules remain native Rust crates.
