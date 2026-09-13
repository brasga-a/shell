# gates.md

# Project Gates — Linux Shell UI

This file defines the validation gates that must be passed before the project advances to the next architectural or product stage.

A gate is not a milestone checklist. It is a **go / no-go decision point**.

If a gate fails, the project must either:

1. fix the implementation;
2. revise an architectural decision in `decisions.md`; or
3. reduce scope explicitly.

Do not proceed by ignoring failed criteria.

---

# Gate rules

Every gate must produce:

```text
result: PASS | FAIL | CONDITIONAL PASS
evidence:
- tests
- measurements
- screenshots/video when relevant
- known limitations
decision impact:
- decisions confirmed
- decisions reopened
- invariants affected
```

A gate only passes when all **mandatory criteria** are satisfied.

Optional criteria may remain open if they are explicitly documented.

---

# G00 — Repository foundation

## Purpose

Validate that the codebase can evolve without immediately collapsing into cyclic dependencies or framework coupling.

## Mandatory criteria

```text
[ ] Cargo workspace builds successfully
[ ] shell-core compiles without GPUI
[ ] shell-core compiles without Hyprland-specific crates
[ ] shell-ui-gpui depends on shell-core, not the inverse
[ ] shell-hyprland is isolated behind compositor-facing interfaces
[ ] shell-linux owns Linux service integrations
[ ] shell-app is the composition root
[ ] structured logging is available
[ ] basic error types exist
[ ] CI runs cargo check
[ ] CI runs cargo test
[ ] CI runs cargo fmt --check
[ ] CI runs cargo clippy
```

## Exit condition

The dependency graph reflects the intended architecture before feature development begins.

## Confirms

```text
D001
D003
D004
D005
D023
```

---

# G01 — GPUI window viability

## Purpose

Prove that GPUI is viable on the target Linux environment before building shell components around it.

## Target environment

Initial validation target:

```text
Wayland
Hyprland
Linux
```

## Mandatory criteria

```text
[ ] GPUI application starts natively on Wayland
[ ] transparent window/background works
[ ] window can render custom content
[ ] pointer input works
[ ] keyboard input works
[ ] resizing does not crash
[ ] window can be recreated cleanly
[ ] application exits without leaking/hanging runtime tasks
```

## Evidence

Capture:

```text
startup logs
window screenshot
basic interaction recording
known backend limitations
```

## Failure policy

If native GPUI behavior is unstable enough to block shell development:

```text
reopen D002
```

and evaluate a lower-level frontend earlier.

---

# G02 — Layer Shell viability

## Purpose

This is the first hard architectural gate.

A desktop shell requires behavior that normal application windows do not provide.

## Mandatory criteria

The implementation must prove:

```text
[ ] wlr-layer-shell surface can be created
[ ] Layer::Top works
[ ] Layer::Overlay works
[ ] anchor TOP works
[ ] anchor TOP | LEFT | RIGHT works
[ ] exclusive zone works
[ ] exclusive zone can be removed/changed
[ ] transparent background works
[ ] keyboard interactivity can be None
[ ] keyboard interactivity can become interactive when needed
[ ] pointer input works only where intended
[ ] surface can target a specific output
[ ] surface survives workspace changes
[ ] surface stays above normal application windows
```

## Hyprland validation

Test with:

```text
normal tiled window
floating window
fullscreen window
special workspace if applicable
multiple workspaces
```

## Hard failure conditions

Any of these blocks progression:

```text
cannot use layer-shell
cannot target an output
cannot control keyboard interactivity
cannot control input behavior
cannot remain correctly layered
```

## Decision impact

If failed:

```text
reopen D002
revisit GPUI backend strategy
consider GPUI patch/fork
consider direct Wayland/SCTK frontend
```

---

# G03 — Surface topology

## Purpose

Resolve whether the shell should use one unified fullscreen surface per output or multiple independent layer surfaces.

## Candidates

### A — Unified surface

```text
Output
└── fullscreen transparent shell surface
    ├── panel
    ├── notch
    ├── dock
    └── overlays
```

### B — Independent surfaces

```text
Output
├── panel surface
├── notch surface
├── dock surface
└── overlay surface
```

## Mandatory experiments

For both approaches test:

```text
[ ] pointer click-through
[ ] dynamic input regions
[ ] click-outside behavior
[ ] keyboard focus
[ ] animation during resize
[ ] exclusive zones
[ ] fullscreen application behavior
[ ] popup layering
[ ] multi-monitor behavior
[ ] output removal
[ ] output addition
```

## Evaluation criteria

Score each option on:

```text
correctness
Wayland compatibility
implementation complexity
focus behavior
input control
animation quality
surface lifecycle complexity
debuggability
```

## Exit condition

Choose one architecture and update:

```text
D007
```

from `Proposed` to `Accepted`.

---

# G04 — Multi-monitor correctness

## Purpose

Prove that per-output state and surface ownership are real architectural properties, not theoretical abstractions.

## Mandatory criteria

Test with at least two outputs.

```text
[ ] each output has a unique OutputId
[ ] panel appears on correct output
[ ] notch opens on intended output
[ ] scale is tracked per output
[ ] geometry is tracked per output
[ ] focused output changes correctly
[ ] output hotplug creates state/surfaces
[ ] output removal destroys state/surfaces
[ ] remaining output continues working
[ ] no restart is required
```

## Fractional scaling

At least one test must use a non-1.0 scale when environment permits.

Validate:

```text
[ ] logical dimensions remain correct
[ ] pointer hit testing remains correct
[ ] text remains aligned
[ ] custom geometry remains aligned
```

## Hard failure conditions

```text
single-monitor assumptions in shared state
orphan surfaces
crash on hotplug
incorrect scale-dependent input geometry
```

---

# G05 — Hyprland adapter

## Purpose

Validate that compositor integration can remain outside UI code.

## Mandatory state coverage

The adapter must provide:

```text
[ ] monitors
[ ] workspaces
[ ] focused workspace
[ ] focused monitor
[ ] windows
[ ] focused window
[ ] fullscreen state
```

## Mandatory event coverage

The implementation must react without UI polling to:

```text
[ ] workspace change
[ ] focused window change
[ ] window open
[ ] window close
[ ] monitor change where exposed
[ ] fullscreen change
```

## Mandatory command coverage

At minimum:

```text
[ ] focus workspace
[ ] focus window or equivalent useful command
```

## Architecture validation

```text
[ ] raw Hyprland JSON does not reach shell-core state
[ ] GPUI code does not call Hyprland IPC directly
[ ] event translation occurs inside shell-hyprland
[ ] temporary malformed/disappearing entities do not crash the shell
```

## Exit condition

The panel can display real Hyprland state using only application-level types.

---

# G06 — Basic panel

## Purpose

Validate the complete path from system event to UI rendering.

## Required UI

At minimum:

```text
[ workspaces ]                     [ clock ]
```

Then add:

```text
volume
network status
battery
```

## Mandatory criteria

```text
[ ] panel is correctly anchored
[ ] panel reserves space when configured to do so
[ ] workspace state updates from Hyprland events
[ ] focused workspace is visually distinct
[ ] clock updates without blocking UI
[ ] panel remains responsive during service failures
[ ] no widget invokes system CLI directly
```

## Performance baseline

Record:

```text
cold startup time
idle RAM
idle CPU
```

This is a baseline, not an optimization target yet.

---

# G07 — Notch geometry

## Purpose

Prove that the visual primitive central to the shell can be rendered without compromising input or animation.

## Required states

```text
Idle
Expanded
```

## Mandatory criteria

```text
[ ] concave left corner renders correctly
[ ] concave right corner renders correctly
[ ] center body scales dynamically
[ ] geometry remains correct at different widths
[ ] geometry remains correct at different heights
[ ] geometry remains correct under fractional scaling
[ ] transparent area does not incorrectly consume pointer input
```

## Implementation freedom

The renderer may use:

```text
GPUI path
custom paint
tessellation
shader
```

The gate evaluates behavior, not implementation style.

---

# G08 — Notch interaction and navigation

## Purpose

Validate that the notch is a real interactive shell container rather than only a custom shape.

## Required routes

```text
Idle
Launcher
```

Optional at this gate:

```text
Dashboard
Notifications
PowerMenu
Tools
```

## Mandatory criteria

```text
[ ] click expands notch
[ ] expansion animates smoothly
[ ] width can animate
[ ] height can animate
[ ] keyboard focus is acquired when needed
[ ] keyboard focus is released on dismiss
[ ] click outside dismisses reliably
[ ] Escape dismisses where applicable
[ ] input region matches interactive area
[ ] interrupted animation does not corrupt state
[ ] repeated rapid open/close does not crash
```

## Hard failure conditions

```text
focus remains stolen after close
transparent areas block applications
animation causes surface corruption
notch state and rendered state diverge permanently
```

---

# G09 — Launcher MVP

## Purpose

Validate a non-trivial feature inside the notch architecture.

## Mandatory criteria

```text
[ ] launcher opens through application state
[ ] text input works
[ ] application list is available
[ ] filtering works
[ ] keyboard navigation works
[ ] Enter launches selected application
[ ] Escape returns to Idle
[ ] launcher logic does not depend on notch rendering internals
[ ] launcher can be rendered elsewhere in principle
```

## Performance

With a realistic local app list:

```text
[ ] typing remains responsive
[ ] filtering does not visibly stall rendering
```

---

# G10 — Audio service

## Purpose

Validate the Linux service adapter pattern with a real system integration.

## Mandatory criteria

```text
[ ] current output volume can be read
[ ] mute state can be read
[ ] volume changes are observed
[ ] mute changes are observed
[ ] set volume works
[ ] toggle mute works
[ ] panel consumes AudioState only
[ ] OSD can consume the same service
[ ] service failure does not crash shell
```

## Preferred implementation

Production target:

```text
PipeWire / proper system API
```

Temporary CLI-based PoC is acceptable only if isolated inside the adapter.

---

# G11 — System services baseline

## Purpose

Prove that the service architecture scales beyond one subsystem.

## Required services

```text
Audio
Battery
Network
MPRIS
```

## Mandatory criteria

For every service:

```text
[ ] one authoritative adapter owns external synchronization
[ ] state is represented with application-level types
[ ] UI does not instantiate infrastructure clients
[ ] service can become unavailable without crashing shell
[ ] state changes are event-driven where practical
```

## Network minimum

```text
connected/disconnected
current connection
basic status
```

## Battery minimum

```text
percentage
charging/discharging
availability
```

## MPRIS minimum

```text
player detection
playback state
metadata
play/pause command
```

---

# G12 — Notifications

## Purpose

Validate asynchronous external events, persistent UI state, and transient shell surfaces together.

## Before implementation

Resolve:

```text
O006 — notification daemon ownership
```

## Mandatory criteria

If the shell owns notifications:

```text
[ ] implements required notification D-Bus interface
[ ] receives notification
[ ] assigns stable NotificationId
[ ] displays popup
[ ] popup timeout works
[ ] dismiss works
[ ] notification history works
[ ] malformed notification does not crash shell
```

If consuming another daemon, document the integration contract instead.

---

# G13 — OSD

## Purpose

Validate short-lived overlay surfaces and shared system state.

## Required OSDs

```text
Volume
Brightness or Microphone
```

## Mandatory criteria

```text
[ ] OSD appears above applications
[ ] does not steal keyboard focus
[ ] disappears automatically
[ ] repeated updates extend/reset visibility correctly
[ ] rapid volume changes do not create multiple conflicting OSDs
[ ] OSD consumes shared service state
```

---

# G14 — Visual system

## Purpose

Prevent feature development from degenerating into duplicated ad-hoc styling.

## Mandatory criteria

Central tokens exist for:

```text
[ ] colors
[ ] typography
[ ] spacing
[ ] radius
[ ] shadows
[ ] motion
[ ] icons
```

At least three independent features must consume the shared system.

Example:

```text
panel
notch
launcher
```

## Failure condition

If every feature still carries its own arbitrary constants, this gate fails even if the UI looks correct.

---

# G15 — Failure resilience

## Purpose

A shell is infrastructure. Optional subsystem failure must not take down the desktop UI.

## Mandatory failure tests

Simulate or induce:

```text
[ ] Hyprland IPC reconnect
[ ] NetworkManager unavailable
[ ] PipeWire unavailable
[ ] no battery device
[ ] no MPRIS players
[ ] invalid configuration
[ ] output removed
[ ] service task panic/error boundary
```

## Mandatory behavior

```text
[ ] process remains alive where recovery is possible
[ ] unaffected features remain usable
[ ] failure is logged
[ ] UI shows neutral unavailable state where appropriate
[ ] adapters can reconnect when applicable
```

---

# G16 — Performance baseline

## Purpose

Establish whether the shell is operationally reasonable before expanding feature scope.

## Required measurements

Measure release build:

```text
cold startup time
idle RAM
idle CPU
frame time during notch animation
input response during launcher filtering
CPU during workspace churn
```

## Test scenario

At minimum:

```text
panel visible
Hyprland adapter active
audio active
network active
battery active if available
MPRIS active
notch idle
```

Then measure again during:

```text
notch animation
launcher typing
workspace switching
volume changes
```

## Rule

No target number is hardcoded before measurement.

This gate passes when:

```text
no pathological idle CPU
no persistent frame stalls
no obvious event-loop starvation
no runaway memory growth during normal interaction
```

Record numbers for future regression comparison.

---

# G17 — Shell MVP

## Purpose

Define the point where the project stops being a technical prototype and becomes a minimally usable shell.

## Required features

```text
[ ] panel
[ ] workspaces
[ ] clock
[ ] notch
[ ] launcher
[ ] audio
[ ] network status
[ ] battery status where available
[ ] MPRIS
[ ] notifications
[ ] OSD
[ ] multi-monitor
[ ] configuration
[ ] centralized theme
```

## Architectural requirements

```text
[ ] G00 through G16 mandatory requirements are satisfied
[ ] decisions.md reflects the implementation
[ ] invariants.md review checklist passes
[ ] no major feature bypasses ports/adapters
[ ] no known input/focus bug can lock the user out of applications
```

## Exit condition

The shell can be used for a normal Hyprland session without relying on another bar/launcher/notification frontend for the features above.

---

# G18 — GPUI continuation decision

## Purpose

After the MVP, decide whether GPUI remains the long-term frontend.

## Evaluate

```text
rendering control
custom geometry complexity
Wayland integration maintenance
memory usage
CPU usage
animation quality
input handling
multi-monitor stability
developer velocity
upstream GPUI direction
amount of local patching/forking required
```

## Outcomes

### PASS — Keep GPUI

Use when GPUI meets shell requirements without disproportionate maintenance.

### CONDITIONAL PASS — Keep GPUI with platform extensions

Use when a small isolated Linux backend layer solves remaining issues.

### FAIL — Begin custom frontend

Potential direction:

```text
wayland-client
+ Smithay Client Toolkit
+ wgpu
+ lyon
+ dedicated text stack
```

The core architecture must remain unchanged.

---

# G19 — Dock

## Purpose

Add dock only after the core shell architecture is stable.

## Mandatory criteria

```text
[ ] dock surface behavior is defined
[ ] auto-hide does not break input
[ ] active applications update from authoritative state
[ ] launching does not bypass application services
[ ] multi-monitor ownership is explicit
[ ] fullscreen behavior is correct
```

Dock is not required for MVP.

---

# G20 — Overview

## Purpose

Validate high-volume window/workspace visualization.

## Mandatory criteria

```text
[ ] window list is event-driven
[ ] rendering many windows remains responsive
[ ] selecting a window focuses it through CompositorPort
[ ] workspace selection uses internal domain commands
[ ] no raw Hyprland payload reaches overview widgets
```

Not required for MVP.

---

# G21 — Lockscreen security gate

## Purpose

Prevent a visually convincing but insecure lockscreen from being shipped.

## This gate is mandatory before any lockscreen is called production-ready.

## Mandatory criteria

```text
[ ] correct session-lock protocol is used
[ ] all outputs are covered
[ ] newly connected outputs are locked
[ ] keyboard/pointer ownership is correct
[ ] authentication boundary is understood
[ ] shell crash behavior is understood
[ ] compositor/session behavior on lock-process death is understood
[ ] bypass attempts have been tested
```

## Hard rule

A normal `Overlay` surface is not sufficient proof of a secure lockscreen.

---

# G22 — Backend replacement readiness

## Purpose

Prove the architecture actually preserves the option to replace GPUI rather than merely claiming it.

## Test

Build a minimal alternative frontend or headless adapter that consumes:

```text
ShellState
application commands
surface-independent geometry/state
```

It does not need feature parity.

## Mandatory criteria

```text
[ ] shell-core compiles without GPUI
[ ] Hyprland adapter runs without GPUI
[ ] Linux services run without GPUI
[ ] state can be observed by another consumer
[ ] application commands can be issued without GPUI
```

## Exit condition

The architecture demonstrates real frontend separation.

---

# Release gates

## Alpha

Requires:

```text
G00–G11
G14–G16
```

Expected state:

```text
functional shell prototype
Hyprland-first
developer-oriented
known rough edges acceptable
```

---

## MVP

Requires:

```text
G00–G17
```

Expected state:

```text
daily-usable core shell
```

---

## Beta

Requires:

```text
MVP
+
G18 completed
+
recovery behavior hardened
+
configuration stabilized
+
basic migration/versioning strategy
```

Optional features such as Dock and Overview may enter here.

---

## Stable

Requires:

```text
no unresolved critical input/focus issues
no known shell-crashing optional service failures
multi-monitor proven
fractional scaling proven
performance regression baselines in CI or release process
security-sensitive features independently validated
architecture documentation matches implementation
```

---

# Gate dependency map

```text
G00 Repository Foundation
 ↓
G01 GPUI Window
 ↓
G02 Layer Shell
 ↓
G03 Surface Topology
 ↓
G04 Multi-monitor
 ↓
G05 Hyprland Adapter
 ↓
G06 Panel
 ├──────────────┐
 ↓              ↓
G07 Notch       G10 Audio
 ↓              ↓
G08 Interaction G11 Services
 ↓              │
G09 Launcher    │
 └──────┬───────┘
        ↓
   G12 Notifications
        ↓
      G13 OSD
        ↓
   G14 Visual System
        ↓
   G15 Resilience
        ↓
   G16 Performance
        ↓
     G17 MVP
        ↓
 G18 GPUI Decision
```

Optional post-MVP:

```text
G19 Dock
G20 Overview
G21 Lockscreen
G22 Backend Replacement Readiness
```

---

# Core gate

The most important go/no-go decision is:

```text
G02 — Layer Shell viability
```

If GPUI cannot reliably support the Wayland semantics required by a desktop shell, do not build the rest of the architecture around workarounds.

Resolve the platform layer first.
