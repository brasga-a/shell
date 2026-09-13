# Module Architecture

This directory is the canonical context for Luna feature modules.

## Decision

Luna modules are **individual Rust crates**. The Notch remains the dynamic visual shell/container; modules are independently owned crates that render their content inside that host when activated.

```text
Notch / shell-ui-gpui
        ↓ hosts
Module crate
        ↓ renders
module content
```

Initial module crates:

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

Recommended package names:

```text
luna-module-launcher
luna-module-calendar
luna-module-player
luna-module-theme
luna-module-resources
luna-module-clock
luna-module-settings
```

## Dependency model

`crates/shell-ui-gpui` owns the Notch host, shared primitives, animation, focus/input coordination and GPUI-specific surface integration. It must not depend on concrete module crates.

Each module crate may depend on the GPUI presentation API/primitives and on stable core/service contracts required by that module. `shell-app` is the composition root and registers concrete modules with the Notch host.

```text
shell-core / shell-linux / shell-config / shell-theme
                    ↑
                    │
             module crates
                    ↑
                    │ registered by
                 shell-app
                    │
                    ▼
             shell-ui-gpui
                 Notch
```

The important constraint is that the host does not import concrete modules, avoiding a dependency cycle.

## Notch contract

The Notch is responsible for:

```text
shell shape
concave geometry
background
surface/input region
focus ownership
routing
module mount/unmount
size transitions
animations
```

A module is responsible for:

```text
its own state
its own commands/intents
its own layout/content
service consumption through ports
its preferred content constraints
```

The Notch adapts its target size to the active module's layout and animates between states. Modules must not directly control Wayland layer-shell semantics.

A conceptual registration API may look like:

```rust
trait NotchModule {
    fn id(&self) -> ModuleId;
    fn title(&self) -> &'static str;
    fn render(&mut self, cx: &mut ModuleContext) -> ModuleView;
}
```

The concrete GPUI API may differ; the architectural boundary is the important part.

## Routing

Prefer a scalable route model:

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

This avoids coupling the Notch state machine to a permanently fixed list of views.

## Initial modules

- [Launcher](./launcher.md)
- [Calendar](./calendar.md)
- [Player](./player.md)
- [Theme](./theme.md)
- [Task Manager / Resources](./resources.md)
- [Clock](./clock.md)
- [Settings](./settings.md)

## Invariants

1. One feature module equals one crate boundary.
2. Modules render inside the Notch when their route is active.
3. `shell-ui-gpui` owns the Notch and shared GPUI infrastructure, not feature behavior.
4. `shell-app` composes/registers modules; the Notch host does not import them directly.
5. Modules do not execute raw Hyprland/Linux commands from widgets.
6. Modules consume system state through core/service ports.
7. Modules do not own Wayland surfaces unless a future ADR explicitly grants one.
8. Shared infrastructure remains outside module crates.
9. A module crate may be lazy-initialized even though it is statically linked into the shell binary.
10. Individual crates do not imply separate processes or dynamic plugins.
