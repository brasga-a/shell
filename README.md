# Shell

A modern Linux/Wayland desktop shell written primarily in Rust, built around a modular Notch interface, GPUI and GPUI Kit.

> **Status:** early development. Shell is not ready to replace a production desktop environment yet.

[![CI](https://github.com/brasga-a/shell/actions/workflows/ci.yml/badge.svg)](https://github.com/brasga-a/shell/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Wayland](https://img.shields.io/badge/Wayland-native-blue)
![License](https://img.shields.io/badge/license-Apache--2.0-green)

## What is Shell?

Shell is an experimental desktop shell for Linux/Wayland, initially targeting **Hyprland**.

The project explores a shell where common desktop interactions are exposed through a compact, dynamic **Notch** instead of turning every system feature into a traditional application window.

```text
                         Shell
                          │
            ┌─────────────┴─────────────┐
            │                           │
        Shell UI                    Full Apps
            │                           │
     Panel + Notch               deep workflows
            │
      module surfaces
```

The core rule is simple:

> **Shell modules are for glanceable information and quick actions. Full applications are for sustained work.**

For example:

```text
Calendar module  → upcoming events + quick create
Calendar app     → day/week/month workspace

Resources module → CPU/RAM + top processes
Task Manager     → deep process/system inspection

Settings module  → quick controls
Settings app     → full system configuration
```

## Design goals

- **Wayland-native** desktop shell.
- **Rust-first** implementation.
- **Hyprland-first**, without making Hyprland part of the domain model.
- **Modular architecture** with individual crates for shell modules.
- **Fast interaction** through the Notch and panel.
- **Event-driven state** instead of unnecessary polling.
- **Local-first** where appropriate.
- **Replaceable frontend architecture**: GPUI is the initial presentation adapter, not the core architecture.
- Strong separation between domain, application, presentation and Linux/compositor integrations.

## Stack

```text
Rust
├── GPUI
├── GPUI Kit
├── Wayland / layer-shell
├── Hyprland IPC
├── Tokio where asynchronous runtime is required
├── D-Bus / native Linux APIs
├── PipeWire where applicable
├── serde + TOML
└── SQLite for local-first domains where appropriate
```

GPUI and GPUI Kit are presentation dependencies. Core crates must remain independent from them.

## Architecture

Shell follows a hexagonal architecture.

```text
                   ┌─────────────────────┐
                   │       Modules       │
                   │  individual crates  │
                   └──────────┬──────────┘
                              │
                              ▼
                   ┌─────────────────────┐
                   │  Notch / UI Host    │
                   │   shell-ui-gpui     │
                   └──────────┬──────────┘
                              │
                              ▼
                   ┌─────────────────────┐
                   │ Application / Core  │
                   │ state + use cases   │
                   └──────────┬──────────┘
                              │
           ┌──────────────────┼──────────────────┐
           ▼                  ▼                  ▼
     CompositorPort     SystemServicePort     ConfigPort
           │                  │                  │
           ▼                  ▼                  ▼
      Hyprland IPC       Linux services         TOML
```

The project is deliberately **not** structured as “a GPUI application that happens to behave like a shell”. GPUI is only the first presentation adapter.

## Workspace

```text
crates/
├── shell-app/           # composition root
├── shell-core/          # renderer/compositor-independent domain
├── shell-platform/      # shell surface/platform contracts
├── shell-hyprland/      # Hyprland adapter
├── shell-linux/         # Linux service adapters
├── shell-config/        # typed configuration + hot reload
├── shell-theme/         # semantic design tokens
├── shell-ui-gpui/       # GPUI/GPUI Kit presentation + Notch host
└── modules/
    ├── launcher/
    ├── calendar/
    ├── player/
    ├── theme/
    ├── resources/
    ├── clock/
    └── settings/
```

`shell-app` is the composition root. It creates services, the UI host and concrete modules, then registers those modules into the Notch.

`shell-ui-gpui` must not depend on concrete module crates.

## The Notch

The Notch is Shell's shared quick-interaction surface.

It owns:

```text
shape / concave geometry
background
routing
module hosting
focus and dismissal
surface/input region
width and height targets
resize animation
```

Modules own their feature-specific content and state, but they do **not** own Wayland surfaces.

Conceptually:

```text
user intent
    ↓
NotchRoute::Module(ModuleId)
    ↓
module content
    ↓
preferred/measured size
    ↓
Notch target geometry
    ↓
animation
    ↓
Wayland surface/input update
```

## Initial modules

### Launcher

Application discovery and fuzzy search, designed around a provider model that can later support windows, files, calculator, commands, clipboard and web search.

References include zlaunch, Anyrun, Walker and Vicinae.

### Calendar

Mini calendar, daily agenda, upcoming events and quick event creation.

The long-term domain is local-first with SQLite and provider adapters for Google Calendar, Microsoft/Outlook and CalDAV/iCloud. A future full Calendar application can reuse the same domain and storage layer.

### Player

MPRIS-backed active media controls with metadata, artwork, playback state and supported transport controls.

### Resources

Glanceable CPU, memory, disk/network and process information. Deep process management belongs to a future Task Manager application.

### Clock

Time/date, timezone and quick timer/alarm functionality. It is intentionally lightweight and should not require a dedicated daemon for normal clock presentation.

### Theme

Theme selection and quick appearance controls. `shell-theme` remains the semantic theme model; the module is only an editor/controller for it.

### Settings

Quick shell settings and module preferences. A future full Settings application can provide deeper configuration workflows.

See [`context/modules/`](context/modules/) for module-specific architecture.

## Configuration

Shell uses typed, modular TOML configuration.

Target layout:

```text
$XDG_CONFIG_HOME/shell/
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

Configuration reload is designed around validated snapshots:

```text
filesystem event
→ debounce
→ load complete candidate
→ validate
→ atomic snapshot replace
→ ConfigChanged
→ UI/services update
```

Invalid live configuration must not replace the last valid snapshot.

## Development status

Current focus:

- module crate architecture;
- GPUI Kit integration;
- Notch module host and routing;
- dynamic Notch sizing/animation;
- initial module implementations;
- Linux service adapters;
- configuration and hot reload.

The practical task list lives in [`TODO.md`](TODO.md). The architectural roadmap lives in [`context/milestones.md`](context/milestones.md).

## Building

Shell currently targets Linux/Wayland.

Prerequisites include a Rust toolchain and the native development libraries required by GPUI/Wayland.

On Debian/Ubuntu-based systems, the CI environment currently installs:

```bash
sudo apt-get install \
  libfontconfig1-dev \
  libfreetype6-dev \
  libwayland-dev \
  libxkbcommon-dev
```

Then:

```bash
git clone https://github.com/brasga-a/shell.git
cd shell
cargo check --workspace
cargo test --workspace
```

The project is still under active development, so running the full shell may require a Wayland session and milestone-specific flags/configuration.

## Quality gates

Pull requests are expected to pass:

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

GitHub Actions runs these checks through the `rust` CI job.

## Contributing

Read [`AGENTS.md`](AGENTS.md) before making changes. It defines architecture boundaries, validation requirements and commit/PR conventions.

General expectations:

- keep changes scoped;
- preserve dependency direction;
- do not move infrastructure access into widgets;
- update architecture documentation when boundaries change;
- add or update tests for changed behavior;
- use Conventional Commit-style messages where practical.

Example:

```text
feat(launcher): add application provider registry
fix(notch): preserve focus during interrupted resize
docs: document module boundaries
```

## Documentation

Start here:

- [`context/README.md`](context/README.md) — canonical project architecture
- [`context/modules/README.md`](context/modules/README.md) — module architecture
- [`context/module-crate-migration.md`](context/module-crate-migration.md) — module crate migration
- [`context/milestones.md`](context/milestones.md) — roadmap and gates
- [`context/adrs/`](context/adrs/) — architecture decisions
- [`TODO.md`](TODO.md) — practical execution checklist
- [`AGENTS.md`](AGENTS.md) — contributor/agent instructions

## License

Licensed under the [Apache License 2.0](LICENSE).
