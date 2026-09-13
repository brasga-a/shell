# ADR-002 — Hexagonal Architecture for Shell Boundaries

- **Status:** Accepted
- **Date:** 2026-09-11
- **Decision owners:** Project maintainers
- **Related decisions:** D003, D004, D005, D006, D009, D010, D011, D012, D023
- **Related gates:** G00, G03, G05, G06, G10, G11, G22

---

## Context

The project is a Linux desktop shell written primarily in Rust.

It must integrate with infrastructure that is inherently platform-specific:

```text
GPUI
Wayland
Hyprland
D-Bus
PipeWire
NetworkManager
BlueZ
UPower
MPRIS
filesystem/configuration
```

At the same time, application concepts such as:

```text
workspaces
outputs
windows
notch state
launcher state
audio state
notifications
commands
user preferences
```

should not be structurally coupled to any single UI toolkit, compositor, protocol library, or Linux subsystem implementation.

Without explicit boundaries, the easiest implementation path would gradually produce code such as:

```text
GPUI widget
   ↓
hyprctl/wpctl/nmcli
   ↓
parse output
   ↓
mutate UI state
```

That architecture is initially fast but creates several problems:

```text
UI becomes coupled to Linux implementation details
Hyprland-specific data spreads through the application
services are duplicated by features
testing requires a real desktop environment
frontend replacement becomes expensive
state ownership becomes unclear
failure handling becomes inconsistent
```

The project also intends to preserve the option of replacing GPUI with a lower-level Wayland + custom renderer implementation later.

A clear dependency model is therefore required from the beginning.

---

## Decision

Use a **hexagonal architecture** as the primary architectural model.

The shell is organized around an application/core layer that defines:

```text
domain types
runtime state
commands
events
use cases
ports
```

External technologies are implemented as adapters.

Conceptually:

```text
                    ┌──────────────────────┐
                    │   Presentation       │
                    │      GPUI            │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │   Application/Core   │
                    │ state + commands     │
                    │ events + ports       │
                    └──────────┬───────────┘
                               │
         ┌─────────────────────┼─────────────────────┐
         ▼                     ▼                     ▼
  CompositorPort          AudioPort             ConfigPort
         │                     │                     │
         ▼                     ▼                     ▼
 Hyprland Adapter        PipeWire Adapter        TOML Adapter
```

The architecture must preserve inward-facing dependency direction.

Adapters depend on contracts exposed by the core.

The core must not depend on adapter implementations.

---

## Architectural layers

### Core

Owns application semantics.

Expected responsibilities:

```text
domain models
ShellState
commands
events
use cases
ports
capability models
state transitions
validation
```

Example:

```rust
pub trait CompositorPort {
    fn outputs(&self) -> Vec<Output>;
    fn workspaces(&self) -> Vec<Workspace>;
    fn windows(&self) -> Vec<Window>;

    async fn focus_workspace(&self, id: WorkspaceId)
        -> Result<(), CompositorError>;
}
```

The core does not know whether this is implemented through:

```text
Hyprland IPC
Sway IPC
Niri IPC
mock adapter
```

---

### Presentation adapter

Initial implementation:

```text
shell-ui-gpui
```

Responsibilities:

```text
render ShellState
translate input into application intents
manage GPUI-specific component state
perform GPUI-specific drawing
bridge application state changes into rerendering
```

It must not own system integration.

Forbidden:

```text
GPUI button
   ↓
hyprctl
```

Required:

```text
GPUI button
   ↓
ApplicationCommand::FocusWorkspace
   ↓
CompositorPort
```

---

### Compositor adapter

Initial implementation:

```text
shell-hyprland
```

Responsibilities:

```text
connect to Hyprland IPC
subscribe to events
parse Hyprland-specific payloads
translate them into internal types/events
translate core commands into Hyprland dispatches
reconnect when possible
```

Hyprland-specific types terminate at this boundary.

---

### Linux service adapters

Expected integrations include:

```text
audio
network
bluetooth
battery/power
notifications
media
system tray
brightness
clipboard
```

Possible implementation technologies:

```text
zbus
PipeWire bindings
Wayland protocols
filesystem APIs
native Rust crates
C library bindings where appropriate
```

Each service exposes an application-facing contract.

---

### Configuration adapter

Persistent configuration is also infrastructure.

Expected flow:

```text
TOML
 ↓
serde
 ↓
validation
 ↓
ShellConfig
```

The application consumes validated configuration, not raw configuration files.

---

## Dependency rule

The intended dependency direction is:

```text
                shell-app
              /     |      \
             /      |       \
            ▼       ▼        ▼
       ui-gpui   hyprland   linux
            \       |        /
             \      |       /
              ▼     ▼      ▼
                shell-core
```

More precisely:

```text
shell-app
    depends on all implementations required for composition

shell-ui-gpui
    depends on shell-core

shell-hyprland
    depends on shell-core

shell-linux
    depends on shell-core

shell-config
    depends on core/config contracts where required

shell-core
    depends on none of the concrete adapters
```

---

## Composition root

Concrete implementations are wired together only at the application boundary.

Expected location:

```text
shell-app/src/main.rs
```

Conceptually:

```rust
let compositor = HyprlandAdapter::new(...);
let audio = PipeWireAudioAdapter::new(...);
let network = NetworkManagerAdapter::new(...);
let config = TomlConfigAdapter::new(...);

let application = ShellApplication::new(
    compositor,
    audio,
    network,
    config,
);

GpuiFrontend::run(application);
```

The exact ownership model may evolve, but implementation selection belongs at the composition root.

---

## Ports

Ports should represent meaningful application capabilities.

Candidate ports:

```text
CompositorPort
AudioPort
NetworkPort
BluetoothPort
PowerPort
MediaPort
NotificationPort
TrayPort
BrightnessPort
ClipboardPort
ConfigPort
```

Ports should not blindly mirror external APIs.

Bad abstraction:

```rust
trait HyprctlPort {
    fn dispatch_raw(&self, command: &str);
}
```

Better abstraction:

```rust
trait CompositorPort {
    async fn focus_workspace(
        &self,
        workspace: WorkspaceId,
    ) -> Result<(), CompositorError>;
}
```

The contract describes what the shell needs, not how Hyprland happens to expose it.

---

## Port granularity

Hexagonal architecture must not become interface proliferation.

Do not create a trait for every function.

A port is justified when it creates a meaningful boundary around:

```text
external infrastructure
replaceable implementation
independent lifecycle
test seam
platform-specific behavior
```

Small internal pure modules generally do not need ports.

---

## Events

Adapters translate infrastructure events into application events.

Example:

```text
Hyprland:
workspace>>2
       ↓
Hyprland adapter
       ↓
WorkspaceFocused {
    output,
    workspace,
}
       ↓
application
       ↓
ShellState
```

Application code must not need to parse:

```text
socket strings
JSON payloads
D-Bus variants
PipeWire callbacks
```

---

## Commands

UI interactions are translated into application commands.

Example:

```text
WorkspaceButton clicked
        ↓
FocusWorkspace(WorkspaceId)
        ↓
application
        ↓
CompositorPort
        ↓
HyprlandAdapter
        ↓
Hyprland IPC
```

Another example:

```text
VolumeSlider changed
        ↓
SetVolume(0.65)
        ↓
AudioPort
        ↓
PipeWireAdapter
```

---

## State ownership

The core owns semantic application state.

Examples:

```text
ShellState
CompositorState
AudioState
NetworkState
MediaState
NotificationState
```

Adapters own infrastructure state required to communicate with external systems.

GPUI owns transient presentation state required only for rendering.

Example:

```text
Core:
NotchRoute::Launcher

GPUI:
current animation progress
hover state
paint cache
```

Presentation-only state must not migrate into the domain model merely because GPUI needs it.

---

## Testing strategy

Hexagonal boundaries must enable testing without a live Hyprland session.

Example:

```text
FakeCompositor
FakeAudioService
FakeNetworkService
```

Core tests can validate:

```text
state transitions
commands
error handling
feature behavior
cross-service coordination
```

without:

```text
Wayland
Hyprland
PipeWire
D-Bus
GPU
```

Adapters receive their own integration tests.

---

## Alternatives considered

### Framework-centric architecture

Example:

```text
GPUI application
├── components
├── services
└── utility modules
```

where GPUI becomes the root abstraction.

#### Rejected because

It would make renderer replacement significantly harder and encourage infrastructure access from UI code.

---

### Layered architecture without ports

Example:

```text
UI
 ↓
Services
 ↓
Linux
```

#### Advantages

Simpler initially.

#### Rejected because

The system has several independently replaceable boundaries:

```text
frontend
compositor
audio
network
notifications
configuration
```

Explicit ports provide valuable seams in this project.

---

### Microservices / daemon-oriented architecture

#### Rejected because

Process boundaries do not solve dependency boundaries automatically.

The initial shell does not require distributed components.

A modular Rust process is simpler.

---

## Consequences

### Positive

The project gains:

```text
replaceable frontend
replaceable compositor adapter
isolated Linux integrations
testable application logic
clear ownership
controlled dependency graph
failure isolation by subsystem
```

This directly supports the long-term possibility of:

```text
GPUI
 ↓ replacement
custom Wayland + renderer frontend
```

without replacing system integrations or application logic.

---

### Negative

The architecture introduces additional types and translation boundaries.

For example:

```text
HyprlandWorkspace
    ↓ conversion
Workspace
```

and:

```text
D-Bus signal
    ↓ conversion
NetworkEvent
```

Small features may require more code than a direct widget-to-command implementation.

Poorly designed ports can also become artificial abstractions with no practical value.

---

## Risks

### R1 — Over-abstraction

Developers may create traits and layers for trivial internal functionality.

**Mitigation:**

Ports exist primarily around external or genuinely replaceable boundaries.

---

### R2 — Generic domain model becomes too weak

Trying to support every compositor may reduce useful Hyprland functionality to a lowest common denominator.

**Mitigation:**

Support capability detection and extension-specific features without contaminating the common model.

The project is Hyprland-first, not portability-first.

---

### R3 — Event duplication

Adapters and application code may accidentally maintain competing copies of external state.

**Mitigation:**

Each external subsystem has one authoritative adapter/service owner.

---

### R4 — Excessive message plumbing

Every interaction could become a chain of trivial wrappers.

**Mitigation:**

Use direct Rust calls inside appropriate boundaries.

Hexagonal architecture defines ownership and dependencies; it does not require an event bus for every function.

---

## Constraints

The following are mandatory:

```text
GPUI types do not enter shell-core

Hyprland payloads do not enter shell-core

widgets do not execute Linux commands

features do not create infrastructure clients

persistent configuration is separate from runtime state

service adapters are authoritative for their subsystem

main.rs/composition root selects concrete implementations
```

---

## Workspace implication

Initial logical structure:

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

These boundaries do not imply that every future subsystem requires a separate crate.

Crates are used where dependency enforcement provides real value.

---

## Interaction with ADR-001

ADR-001 selects GPUI as the initial frontend.

ADR-002 ensures that this decision does not turn GPUI into the architecture itself.

The relationship is:

```text
ADR-001:
GPUI is the first frontend.

ADR-002:
the frontend is outside the application core.
```

Together they allow:

```text
today:

GPUI
  ↓
Core
  ↓
Linux/Hyprland adapters
```

and potentially later:

```text
Custom renderer
  ↓
same Core
  ↓
same Linux/Hyprland adapters
```

---

## Validation

### G00 — Repository foundation

Must prove dependency direction.

### G05 — Hyprland adapter

Must prove Hyprland payloads terminate at the adapter boundary.

### G06 — Panel

Must prove real compositor state reaches GPUI through application-level models.

### G10/G11 — Linux services

Must prove UI consumes service contracts rather than infrastructure clients.

### G22 — Backend replacement readiness

Must prove core and adapters can operate without GPUI.

---

## Conditions that reopen this ADR

Reopen only if:

```text
the port boundaries consistently obstruct required performance
core/adapters cannot express required compositor capabilities cleanly
a fundamentally different state architecture becomes necessary
the project intentionally becomes tightly Hyprland + GPUI specific
```

Do not reopen because:

```text
direct calls are shorter
a feature would require fewer files without boundaries
a prototype can be written faster by calling commands from UI
```

Those are implementation conveniences, not architectural reasons.

---

## Final rationale

A Linux shell sits between presentation, compositor protocols, and operating-system services.

Those boundaries are real regardless of programming style.

The architecture should make them explicit.

Therefore:

```text
The shell core defines behavior and contracts.

GPUI, Hyprland and Linux APIs implement adapters around that core.
```

This is the primary dependency model for the project.
