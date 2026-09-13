# Configuration Decision — Modular TOML

- **Status:** Accepted
- **Date:** 2026-09-12
- **Scope:** Shell configuration architecture

---

## Decision

The shell will use **TOML as its primary configuration format** and will keep configuration **modularized into a small number of files with clear ownership boundaries**.

Configuration will not be stored in one monolithic file.

Initial layout:

```text
~/.config/atlantic/
├── config.toml
├── theme.toml
├── keybinds.toml
└── modules/
    ├── bar.toml
    ├── notch.toml
    ├── dock.toml
    ├── launcher.toml
    └── notifications.toml
```

The exact module files only exist when the corresponding module is implemented or configurable.

---

## Why TOML

TOML is selected because configuration is primarily declarative.

The shell needs:

```text
predictable parsing
typed schemas
clear defaults
validation
versioning
migration
human-readable files
good Rust integration
```

The intended Rust stack is:

```text
serde
toml
```

Lua is not used as the primary configuration format.

Lua may be introduced later for scripting or extension APIs if the project develops a real need for programmable configuration.

---

## File responsibilities

### `config.toml`

Contains global shell behavior.

Example:

```toml
version = 1

[general]
animations = true
startup_delay_ms = 0

[compositor]
backend = "hyprland"

[modules]
bar = true
notch = true
dock = false
launcher = true
notifications = true
```

Appropriate content:

```text
global behavior
feature enablement
compositor selection
general shell options
configuration schema version
```

It must not become a dump for module-specific styling or behavior.

---

### `theme.toml`

Contains design tokens and visual configuration shared across modules.

Example:

```toml
[colors]
background = "#090909"
foreground = "#EAE6E5"

[radius]
sm = 6
md = 10
lg = 16

[spacing]
sm = 4
md = 8
lg = 16

[motion]
fast_ms = 120
normal_ms = 220
slow_ms = 360
```

Possible domains:

```text
colors
typography
spacing
radius
shadows
motion
icons
```

Theme values must be consumed through the central theme system rather than independently parsed by each feature.

---

### `keybinds.toml`

Contains user-facing shell actions and their bindings.

Example:

```toml
[[binding]]
keys = ["SUPER", "SPACE"]
action = "launcher.toggle"

[[binding]]
keys = ["SUPER", "N"]
action = "notifications.toggle"

[[binding]]
keys = ["SUPER", "D"]
action = "dock.toggle"
```

Keybinds describe **semantic shell actions**, not compositor commands.

Correct:

```toml
action = "launcher.toggle"
```

Avoid:

```toml
command = "hyprctl dispatch exec ..."
```

The configuration layer maps strings into internal actions:

```text
"launcher.toggle"
        ↓
ShellAction::ToggleLauncher
```

The compositor/platform adapter is responsible for integrating those actions with Hyprland, KDE Plasma, Niri, Sway, or other supported environments.

This prevents `keybinds.toml` from becoming Hyprland-specific.

---

## `modules/`

Each configurable shell feature owns one file.

Example:

```text
modules/
├── bar.toml
├── notch.toml
├── dock.toml
├── launcher.toml
└── notifications.toml
```

A module file contains only configuration belonging to that module.

### Example — `modules/bar.toml`

```toml
enabled = true
position = "top"
height = 32

modules_left = [
    "workspaces"
]

modules_center = [
    "clock"
]

modules_right = [
    "network",
    "audio",
    "battery"
]
```

### Example — `modules/notch.toml`

```toml
enabled = true

collapsed_width = 144
width = 520
collapsed_height = 32
expanded_height = 360
corner_radius = 24
corner_size = 24
edge = "top"

[animation]
enabled = true
duration_ms = 220
```

### Example — `modules/dock.toml`

```toml
enabled = false
position = "bottom"
auto_hide = true
icon_size = 42
```

---

## Rust model

Configuration files are separate on disk but become a **single validated configuration model** for the application.

Conceptually:

```rust
struct ShellConfig {
    version: u32,
    general: GeneralConfig,
    compositor: CompositorConfig,
    modules: ModuleRegistry,
    theme: ThemeConfig,
    keybinds: KeybindConfig,
    bar: BarConfig,
    notch: NotchConfig,
    dock: DockConfig,
}
```

The rest of the application should not care which file a value came from.

Loading flow:

```text
config.toml
theme.toml
keybinds.toml
modules/*.toml
        ↓
ConfigLoader
        ↓
deserialize
        ↓
defaults
        ↓
validation
        ↓
ValidatedConfig
        ↓
ShellApplication
```

---

## Configuration loader

The configuration loader owns:

```text
file discovery
parsing
default values
schema validation
merging
version checks
migration
diagnostics
```

Feature modules must not read configuration files directly.

Forbidden:

```text
Bar widget
    ↓
read ~/.config/atlantic/modules/bar.toml
```

Required:

```text
ConfigLoader
    ↓
ValidatedConfig
    ↓
application state
    ↓
Bar UI
```

---

## Defaults

Every configuration domain must provide safe defaults.

The shell must be able to start when:

```text
the config directory does not exist
a module config file does not exist
an optional property is omitted
```

Missing configuration means:

```text
use defaults
```

not:

```text
fail startup
```

---

## Invalid configuration

Invalid user configuration must not brick the desktop session.

Expected behavior:

```text
load configuration
       ↓
parse/validate
       ↓
invalid field
       ↓
diagnostic
       ↓
fallback when safe
       ↓
shell remains usable
```

Critical parse errors should produce a clear diagnostic and use a known-safe fallback configuration when possible.

---

## Schema version

`config.toml` owns the global configuration schema version.

Example:

```toml
version = 1
```

This allows future migrations such as:

```text
v1
 ↓
migration
 ↓
v2
```

Individual module files should not introduce independent version systems unless they genuinely need separate migration lifecycles.

---

## Hardcoded values

The goal is **not** to remove every constant from source code.

Values belong in configuration when they are:

```text
user preferences
theme values
feature behavior
module composition
keybinds
reasonable runtime customization points
```

Values should remain hardcoded when they are:

```text
protocol constants
internal implementation details
security constraints
structural invariants
values users should not control
```

Configuration must not become an externalized source-code dump.

---

## Module discovery

The presence of a file does not automatically mean a module exists.

Modules are defined by the application.

For example:

```text
modules/foo.toml
```

must not dynamically create an arbitrary unknown shell module.

The loader should only load known schemas.

Conceptually:

```text
bar.toml
    ↓
BarConfig

notch.toml
    ↓
NotchConfig
```

Unknown files may be ignored with a diagnostic.

---

## Live reload

Live reload is desirable but is not required for the first implementation.

When added, reload should follow:

```text
filesystem change
      ↓
ConfigLoader
      ↓
parse + validate complete configuration
      ↓
atomic configuration update
      ↓
affected state/UI update
```

Do not partially mutate live state while parsing several files.

Invalid reloads should preserve the previous valid configuration.

---

## Separation from runtime state

Configuration is not runtime state.

Configuration examples:

```text
bar height
theme radius
notch animation duration
enabled modules
keybinds
```

Runtime state examples:

```text
focused workspace
launcher open
current volume
notification list
media playback
network connection
```

Do not persist transient runtime state into TOML unless explicitly introduced as a product feature.

---

## Directory growth rule

The configuration directory should remain intentionally small.

Do not create:

```text
one file per property
one file per widget
deep configuration directory trees
```

Preferred boundary:

```text
global
theme
keybinds
module
```

A module should generally own one configuration file.

Subdirectories inside `modules/` should only appear when a module becomes complex enough to justify them.

---

## Future scripting

If programmable customization becomes necessary, the preferred model is:

```text
TOML
→ configuration

Lua
→ optional scripting/extensions
```

Possible future structure:

```text
~/.config/atlantic/
├── config.toml
├── theme.toml
├── keybinds.toml
├── modules/
└── scripts/
    └── *.lua
```

Lua must not replace TOML merely to make static configuration programmable.

---

## Invariants

1. TOML is the primary configuration format.
2. Configuration files have clear ownership boundaries.
3. `ConfigLoader` is the only component that reads configuration files.
4. Application features consume typed configuration.
5. Persistent configuration and runtime state remain separate.
6. Module configs do not expose raw Hyprland commands.
7. Keybinds map to semantic `ShellAction` values.
8. Missing optional files fall back to defaults.
9. Invalid configuration must not make the shell unusable.
10. Configuration is validated before becoming active.
11. Modules cannot be created dynamically through unknown TOML files.
12. GPUI widgets never parse TOML directly.
13. Configuration must not externalize internal implementation constants without user value.

---

## Final structure

```text
~/.config/atlantic/
├── config.toml          # global shell behavior
├── theme.toml           # design system / appearance
├── keybinds.toml        # semantic shell shortcuts
└── modules/
    ├── bar.toml         # panel configuration
    ├── notch.toml       # notch behavior
    ├── dock.toml        # dock behavior
    ├── launcher.toml    # launcher configuration
    └── notifications.toml
```

The files are modular on disk while the application consumes one coherent, typed configuration model.

---

## Rationale

A single configuration file would become increasingly difficult to navigate as the shell grows.

A highly fragmented configuration tree would create unnecessary complexity.

The selected structure sits between those extremes:

```text
small number of files
+
clear domain ownership
+
typed Rust schemas
+
centralized loading
```

Therefore:

> Use modular TOML configuration with `config.toml`, `theme.toml`, `keybinds.toml`, and one file per configurable shell module.
