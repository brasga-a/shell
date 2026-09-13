# AGENTS.md

This file defines the operating rules for coding agents working in the Luna repository.

Luna is a Rust/Wayland desktop shell focused initially on Hyprland. GPUI is the presentation frontend, GPUI Kit is the preferred UI foundation, and the architecture must keep compositor, Linux services, configuration, domain state, and presentation concerns separated.

---

## 1. Read before changing code

Before implementing a change, inspect the relevant architecture context under `context/`.

Primary sources of truth:

```text
context/README.md
context/milestones.md
context/metas/decisions.md
context/metas/invariants.md
context/metas/gates.md
context/adrs/
context/modules/
context/module-crate-migration.md
```

Use them in this order:

```text
invariants
  ↓ cannot be violated
ADRs / decisions
  ↓ current architectural choice
milestones / gates
  ↓ delivery and acceptance evidence
module docs
  ↓ feature-specific behavior and boundaries
```

If implementation evidence contradicts an ADR or decision, do not silently work around it. Document the evidence and update/supersede the architecture explicitly.

---

## 2. Repository architecture

Current workspace shape:

```text
crates/
├── shell-core/
├── shell-platform/
├── shell-hyprland/
├── shell-linux/
├── shell-config/
├── shell-theme/
├── shell-ui-gpui/
├── shell-app/
└── modules/
    ├── launcher/
    ├── calendar/
    ├── player/
    ├── theme/
    ├── resources/
    ├── clock/
    └── settings/
```

Core dependency direction:

```text
presentation / module crates
          ↓
application + core contracts
          ↓
platform / Linux / compositor adapters
```

Never reverse this by introducing presentation dependencies into domain or infrastructure crates.

---

## 3. Architectural invariants

Agents must preserve at minimum the following rules:

1. `shell-core` must not depend on GPUI, GPUI Kit, Hyprland, PipeWire, D-Bus clients, or other concrete adapters.
2. Hyprland is an adapter, not the domain model.
3. Widgets and module UI must not invoke `hyprctl`, `wpctl`, `nmcli`, `bluetoothctl`, or other system CLIs directly.
4. Widgets must not create raw D-Bus, PipeWire, Wayland, or compositor clients.
5. Persistent configuration and runtime state are separate concepts.
6. Wayland surface semantics live outside feature widgets.
7. Multi-monitor ownership must remain explicit.
8. Optional subsystem failure must not crash the shell.
9. Recoverable production I/O/protocol errors must not use `unwrap()`/`expect()` as control flow.
10. Prefer event-driven subscriptions over polling when an event source exists.
11. One authoritative state source should exist per external subsystem.
12. `shell-app` is the composition root.
13. GPUI remains a replaceable presentation adapter.
14. Architecture boundaries take priority over dependency purity or convenience.

---

## 4. Module architecture

Feature modules are independent crates under `crates/modules/*`.

The Notch is the dynamic host/casing; modules provide feature content.

```text
shell-app
   │ registers
   ▼
ModuleRegistry
   │
   ▼
shell-ui-gpui / Notch host
   │ hosts
   ▼
module crate UI
```

Rules:

- `shell-ui-gpui` must not depend on concrete module crates.
- `shell-app` depends on both the host and concrete modules and performs registration.
- A module must not own Wayland surfaces unless a future ADR explicitly allows it.
- A module may depend on presentation APIs and stable service/application contracts required for its feature.
- Module-specific logic belongs in the module crate; shared infrastructure belongs in the appropriate shared crate.
- Do not create new crates merely because a feature has multiple files; crate boundaries must represent architectural ownership.

Initial module documentation is under `context/modules/`.

---

## 5. Shell module vs full application

Use this product boundary:

> Shell modules are for glanceable state and quick actions. Full applications are for sustained work.

Examples:

```text
Calendar module  → today/upcoming/quick add
Calendar app     → week/day/month workspace, drag/drop, advanced editing

Resources module → CPU/RAM/top processes
Task Manager     → deep process management and hardware inspection

Settings module  → quick shell preferences
Settings app     → complete control center
```

If an interaction naturally becomes a multi-minute workspace, prefer a separate app backed by the same domain/services instead of expanding the Notch indefinitely.

---

## 6. GPUI and GPUI Kit

GPUI Kit is the preferred UI foundation for presentation code.

Allowed locations include:

```text
shell-ui-gpui
crates/modules/*
future GPUI applications
```

Do not add GPUI Kit to:

```text
shell-core
shell-platform domain types
shell-hyprland
shell-linux
shell-config
shell-theme domain/token model
```

Use GPUI Kit components directly when they fit the desired UX. Wrapping every component is not required.

Prefer GPUI Kit for common behavior and components such as:

```text
buttons
inputs
selects
lists
scrolling
overlays
popovers
dialogs
tooltips
focus helpers
motion / presence / transitions
```

Keep Luna-owned primitives for genuinely shell-specific behavior such as:

```text
Notch geometry
Wayland layer-shell integration
surface/input-region coordination
module hosting
calendar grids
resource charts
other domain-specific views
```

Do not introduce a second generic UI toolkit over GPUI Kit.

---

## 7. State and data flow

Prefer unidirectional state flow:

```text
Linux / compositor event
        ↓
adapter
        ↓
domain/application event
        ↓
state store
        ↓
GPUI/module render
```

Commands travel in the opposite direction:

```text
user interaction
       ↓
UI intent
       ↓
application command
       ↓
port/service
       ↓
Linux / compositor adapter
```

Presentation code renders state; it should not discover system state directly.

---

## 8. Configuration

Configuration is TOML-based and modular.

General rules:

- Only `shell-config` should own discovery, parsing, defaults, migrations, validation, file watching, and diagnostics.
- Modules consume typed validated configuration.
- Invalid hot reloads must preserve the previous valid configuration.
- Prefer directory watching + debounce + full candidate validation + atomic publish.
- Respect `$XDG_CONFIG_HOME`; use `~/.config` only as fallback behavior.
- Keybind configuration must map to semantic actions, not raw compositor commands.

---

## 9. Rust implementation style

Prefer clear ownership and explicit types over clever abstractions.

Guidelines:

- Use typed IDs instead of raw integers/strings where identity matters.
- Keep raw protocol payloads inside adapter crates.
- Use `thiserror`-style typed errors at library boundaries and richer contextual errors only at orchestration edges when appropriate.
- Add `tracing` context for subsystem/output/surface/module information where useful.
- Avoid global mutable state.
- Avoid long-lived locks on render paths.
- Keep filesystem, network, D-Bus, and process work off the GPUI render path.
- Do not prematurely introduce daemons, dynamic plugins, custom runtimes, or generic abstraction layers.
- Keep renderer-independent geometry/math free of GPUI-specific types where practical.

---

## 10. Testing and validation

Before declaring a change complete, run the checks relevant to the touched crates.

Repository baseline:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets
```

If full-workspace execution is impractical during iteration, run focused checks first, then the workspace checks before finalizing when possible.

For Wayland/Hyprland behavior, code compilation is not sufficient. Validate runtime semantics when the milestone/gate requires it, including:

```text
layer-shell behavior
focus
input regions
fullscreen interaction
multi-monitor
output hotplug
fractional scaling
Notch resize/animation
module switching
```

Do not claim performance improvements without measurements from release builds.

---

## 11. Documentation requirements

Update context documentation when a change modifies architectural behavior, module ownership, dependency direction, configuration schema, or milestone assumptions.

Do not leave code and `context/` describing different architectures.

When adding a substantial feature/module, document:

```text
purpose
ownership
state
ports/services
UI boundary
Notch behavior
persistence
MVP
non-goals
references when relevant
```

---

## 12. Commits

Commits should be small, coherent, and reviewable. A commit should represent one meaningful change, not an arbitrary checkpoint.

Use Conventional Commit-style prefixes:

```text
feat:     new user-visible behavior
fix:      bug fix
refactor: internal restructuring without intended behavior change
docs:     documentation only
test:     tests only
build:    dependencies, workspace, build system
ci:       CI/workflow changes
perf:     measured performance improvement
chore:    maintenance that does not fit another category
```

Examples:

```text
feat(launcher): add application provider registry
fix(notch): preserve focus during interrupted resize
refactor(calendar): move sync contracts into calendar core
docs: document module crate boundaries
build: add GPUI Kit workspace dependency
```

Commit rules:

- Use imperative, concise subjects.
- Keep unrelated changes in separate commits.
- Do not mix broad formatting churn with functional work.
- Do not commit generated artifacts, local credentials, editor state, temporary logs, or secrets.
- Never rewrite or force-push shared history unless explicitly requested.
- Preserve attribution/history when moving substantial code where practical.
- If architecture changes, include the corresponding context/ADR update in the same logical series.

---

## 13. Pull requests

PRs should explain the problem and architectural impact, not merely list changed files.

A good PR description should include:

```text
## Summary
What changed and why.

## Architecture
What boundaries, dependencies, state flow, or ownership changed.

## Validation
Commands/tests/runtime checks performed.

## Risks / limitations
Known gaps, platform constraints, follow-up work.
```

For UI changes, also describe the affected interaction/state transition. Screenshots or recordings are useful when the visual result is material.

PR rules:

- Prefer focused PRs over unrelated feature bundles.
- Keep architecture docs synchronized with implementation.
- Call out ADR/invariant changes explicitly.
- Link relevant milestone/gate/module documentation when applicable.
- Do not mark a PR ready if mandatory checks relevant to the change are knowingly failing without documenting why.
- Do not hide known regressions in the PR body.
- Avoid drive-by refactors unrelated to the PR objective.
- If a change crosses module/infrastructure boundaries, explain why the dependency direction remains valid.

Suggested title style follows commit conventions:

```text
feat(launcher): add provider-based search architecture
fix(config): preserve last valid snapshot on reload failure
docs: define calendar local-first architecture
```

---

## 14. What agents should not do

Do not:

```text
put GPUI/GPUI Kit types in shell-core
couple domain state to Hyprland payloads
call system CLIs directly from widgets
make modules own infrastructure clients
create a daemon by default
create a crate for every component
turn the Notch into a full application workspace
silently bypass gates or invariants
introduce polling when subscriptions exist
claim performance improvements without measurement
replace architecture documentation implicitly through code
```

When uncertain, preserve the existing boundary and document the unresolved decision rather than creating hidden coupling.
