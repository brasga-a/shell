# Launcher Module

## Crate

`crates/modules/launcher` → package `shell-module-launcher`

The Launcher is Luna's primary command/search surface.

It renders inside the Notch and should evolve beyond a simple application picker into a provider-based command interface capable of searching and executing multiple categories of actions.

The first implementation must remain small enough to validate the module-host architecture before expanding into a full Raycast-like experience.

Conceptually:

```text
Notch
  ↓ hosts
Launcher Module
  ↓
Search Controller
  ↓
Provider Registry
  ├── Applications
  ├── Windows
  ├── Calculator
  ├── Files
  ├── Commands
  ├── Clipboard
  └── Web Search
```

Only `ApplicationsProvider` is mandatory for the first MVP.

---

# External references

The Launcher should use existing Linux launchers as architectural and UX references rather than reimplementing every problem without prior art.

## zlaunch — primary implementation reference

Repository:

```text
https://github.com/zortax/zlaunch
```

zlaunch is the most directly relevant implementation reference because it combines:

```text
Rust
GPUI
Wayland
.desktop application discovery
icons
fuzzy search
window switching
calculator
web search
clipboard integration
persistent/daemon-oriented startup optimization
```

For Luna, study zlaunch primarily for:

```text
GPUI composition
keyboard/focus handling
application discovery
.desktop parsing
icon resolution
search flow
result rendering
caching/startup behavior
Wayland-specific launcher behavior
```

Do not copy zlaunch's architecture blindly. Luna must preserve its own module/service boundaries and render inside the Notch rather than own an independent launcher surface.

---

## Anyrun — provider architecture reference

Repository:

```text
https://github.com/anyrun-org/anyrun
```

Anyrun is useful as a conceptual reference for separating query sources into independent providers/plugins.

The useful idea for Luna is:

```text
query
  ↓
provider registry
  ↓
providers search independently
  ↓
normalized results
  ↓
ranking/merge
  ↓
UI
```

Luna should initially implement this using ordinary statically linked Rust types rather than Anyrun-style dynamic libraries.

Preferred initial model:

```rust
Vec<Box<dyn LauncherProvider>>
```

Do not introduce a stable plugin ABI, `cdylib`, WASM plugins or runtime crate loading during the initial launcher implementation.

---

## Walker — provider capability reference

Repository:

```text
https://github.com/abenz1267/walker
```

Walker is useful as a reference for the breadth of providers a Linux launcher can support, including:

```text
applications
calculator
files
commands
web search
clipboard
symbols
windows
Bluetooth
audio/system actions
```

Walker uses a different UI stack, so it should be treated as a feature/architecture reference rather than a rendering implementation reference.

---

## Vicinae — UX/product reference

Repository:

```text
https://github.com/vicinaehq/vicinae
```

Vicinae should be treated primarily as a UX/product reference for a Raycast-like command launcher.

Relevant concepts:

```text
search-first interaction
results + actions
secondary action menus
nested commands/views
keyboard-first navigation
rich result metadata
extensions/providers
```

Luna should borrow interaction patterns where useful without coupling itself to Vicinae's C++/Qt implementation or licensing model.

---

# Architectural direction

The Launcher should be implemented as a provider-oriented search system.

```text
LauncherModule
    │
    ├── QueryController
    │
    ├── ProviderRegistry
    │    ├── ApplicationsProvider
    │    ├── WindowsProvider
    │    ├── CalculatorProvider
    │    ├── FilesProvider
    │    ├── CommandsProvider
    │    ├── ClipboardProvider
    │    └── WebProvider
    │
    ├── Ranking
    │
    └── Launcher UI
```

This allows Luna to add launcher capabilities without turning `LauncherModule` into a large `match` statement containing unrelated search logic.

---

# Provider contract

The exact Rust API may evolve, but the conceptual contract should remain close to:

```rust
pub trait LauncherProvider {
    fn id(&self) -> ProviderId;

    fn metadata(&self) -> ProviderMetadata;

    fn search(
        &self,
        query: &LauncherQuery,
        cx: &LauncherContext,
    ) -> ProviderSearch;

    fn execute(
        &self,
        result: &LauncherResult,
        action: ResultAction,
        cx: &LauncherContext,
    ) -> Result<(), LauncherError>;
}
```

`ProviderSearch` may be synchronous or asynchronous depending on the provider.

The abstraction should support:

```text
fast local providers
async providers
partial results
provider-specific actions
cancellation/stale-query rejection
```

Do not force every provider into async if it is unnecessary.

---

# Core launcher types

Recommended renderer-independent types:

```rust
struct LauncherQuery {
    text: String,
    generation: u64,
}

struct LauncherResult {
    id: ResultId,
    provider: ProviderId,
    title: String,
    subtitle: Option<String>,
    icon: Option<IconRef>,
    score: f32,
    actions: Vec<ResultAction>,
}

enum ResultAction {
    Primary,
    Secondary(ActionId),
}
```

These types should not contain GPUI-specific elements.

The provider owns feature-specific payloads behind typed IDs/internal state rather than stuffing arbitrary infrastructure objects into `LauncherResult`.

---

# Initial providers

## ApplicationsProvider — MVP

Responsible for:

```text
freedesktop .desktop discovery
application metadata normalization
visibility rules
search/indexing
icon references
launch intent
```

Normalize entries into an internal model such as:

```rust
struct ApplicationEntry {
    id: ApplicationId,
    name: String,
    generic_name: Option<String>,
    comment: Option<String>,
    keywords: Vec<String>,
    icon: Option<IconRef>,
    executable: ApplicationCommand,
}
```

The UI must not execute the raw `Exec=` field directly.

Execution flows through an application execution service:

```text
Launcher UI
   ↓
LauncherAction::Execute(ApplicationId)
   ↓
ApplicationService
   ↓
validated desktop-entry execution
```

---

## WindowsProvider — later

Consumes compositor-neutral window state from the core/compositor port.

Responsibilities:

```text
search open windows
show application/window title
focus selected window
optionally expose close/move actions later
```

It must not invoke Hyprland IPC directly.

---

## CalculatorProvider — later

Pure/local provider when possible.

Responsibilities:

```text
recognize mathematical expressions
evaluate safely
return copyable result
```

Do not shell out to arbitrary interpreters.

---

## FilesProvider — later

Responsibilities may include:

```text
filename search
recent files
open containing directory
open selected file
```

Filesystem indexing strategy requires a separate decision before implementing broad full-disk indexing.

---

## CommandsProvider — later

This provider must distinguish safe shell actions from arbitrary command execution.

Initial scope should favor typed Luna actions:

```text
open settings
reload config
lock session
power actions
module navigation
```

A generic arbitrary-shell-command mode should not be part of the initial launcher.

---

## ClipboardProvider — later

Consumes a clipboard/history service rather than owning compositor/clipboard protocol clients.

---

## WebProvider — later

Produces browser/search actions from configured search engines.

Network fetching should not be required merely to construct a search URL.

---

# Search pipeline

Recommended flow:

```text
keyboard input
    ↓
LauncherQuery generation N
    ↓
ProviderRegistry
    ↓
providers
    ↓
normalized LauncherResult stream
    ↓
ranking + deduplication
    ↓
visible result list
```

Every new query increments a generation/revision.

Results from older async searches must be discarded:

```text
query generation 10
query generation 11
provider returns generation 10
→ discard
```

This prevents stale results from replacing newer search state.

---

# Ranking

Ranking should remain independent from rendering.

Initial factors may include:

```text
fuzzy textual score
exact prefix/name match
provider priority
usage frequency
recency
pinned/favorite state
```

The first MVP does not need sophisticated learning-to-rank.

Begin with deterministic fuzzy scoring and add usage/recency only after the base behavior is measured.

Searchable application fields should include, where available:

```text
Name
GenericName
Keywords
Comment
```

The primary application name should receive greater weight than secondary metadata.

---

# Indexing and caching

Application discovery should not parse every desktop entry during every keystroke.

Expected flow:

```text
startup / service initialization
        ↓
discover desktop entries
        ↓
parse + normalize
        ↓
build application index
        ↓
cache in memory
        ↓
query index repeatedly
```

Watch relevant application directories or refresh the index through controlled events where practical.

Potential sources include the standard XDG application directories rather than hardcoded distro-specific paths.

---

# Icons

Icon resolution is part of infrastructure/shared asset handling, not arbitrary widget code.

Flow:

```text
.desktop Icon value
     ↓
IconRef
     ↓
icon resolver/cache
     ↓
renderable GPUI asset
```

Support:

```text
freedesktop icon themes
absolute icon paths where valid
fallback icon
cache
```

The Launcher should not repeatedly walk icon directories during rendering.

---

# UI stack

The launcher crate is a presentation module and may use:

```text
GPUI Kit
├── gpui-base behavior
└── gpui-component controls
```

Prefer GPUI Kit components directly when they satisfy Luna's requirements.

Likely useful components/primitives include:

```text
text input
scroll/list container
virtualized list where necessary
buttons/icon buttons
keyboard/focus helpers
tooltips
overlays/action menus
Presence / motion primitives
```

Luna-specific UI should be created only where the launcher requires behavior/appearance not provided cleanly by GPUI Kit.

---

# Suggested crate layout

```text
crates/modules/launcher/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── module.rs
    ├── state.rs
    ├── query.rs
    ├── result.rs
    ├── ranking.rs
    ├── registry.rs
    │
    ├── providers/
    │   ├── mod.rs
    │   ├── applications.rs
    │   ├── windows.rs
    │   ├── calculator.rs
    │   ├── files.rs
    │   ├── commands.rs
    │   ├── clipboard.rs
    │   └── web.rs
    │
    └── ui/
        ├── mod.rs
        ├── search_input.rs
        ├── results.rs
        ├── result_row.rs
        └── action_menu.rs
```

Do not create all provider files before they are implemented. The structure above is the target shape, not a requirement to scaffold unused code.

---

# State

Initial module state:

```rust
struct LauncherState {
    query: String,
    query_generation: u64,
    results: Vec<LauncherResult>,
    selected_index: Option<usize>,
    status: LauncherStatus,
}
```

Possible status:

```rust
enum LauncherStatus {
    Idle,
    Searching,
    Ready,
    Error,
}
```

Provider/service state should not be duplicated unnecessarily inside the view state.

---

# Keyboard interaction

The Launcher is keyboard-first.

Minimum behavior:

```text
type          → update query
ArrowUp       → previous result
ArrowDown     → next result
Enter         → primary action
Escape        → dismiss launcher
Tab / shortcut → action menu later
```

Focus ownership remains coordinated through the Notch/FocusManager.

The Launcher must not independently manipulate Wayland keyboard interactivity.

---

# Notch behavior

The Launcher renders content inside the Notch.

The Launcher controls its internal layout but does not own the outer shell geometry.

```text
Launcher content
      ↓
layout measurement / preferred constraints
      ↓
Notch target size
      ↓
GPUI Kit motion / spring
      ↓
rendered Notch geometry
```

The Notch owns:

```text
outer shape
concave corners
surface size
input region
focus boundary
open/close transition
click-outside dismissal
```

The Launcher owns:

```text
search box
result list
selection
provider results
action UI
```

---

# Animation

Use GPUI Kit motion primitives for launcher transitions where appropriate.

Examples:

```text
Presence  → result/action view enter/exit
transition → opacity
spring     → internal layout movement where useful
stagger    → optional result appearance, only if it remains performant
```

Do not encode semantic launcher state as animation dimensions.

Example:

```text
state = SearchResults
```

not:

```text
state.height = 480
```

---

# Service boundaries

The Launcher may consume:

```text
ApplicationService
CompositorPort
ClipboardService
ConfigPort
Browser/OpenUrl service
future file index service
```

Forbidden inside launcher UI/provider rendering code:

```text
Command::new("hyprctl")
raw Hyprland socket access
raw D-Bus connections
walking system directories during render
executing raw desktop-entry Exec strings from widgets
```

Providers should operate through typed services and application actions.

---

# Performance requirements

The launcher must feel immediate.

Measure rather than invent hard performance numbers.

At minimum benchmark:

```text
first open latency
subsequent open latency
application index construction
query-to-results latency
large application-list behavior
rapid typing / stale async result handling
icon cache behavior
memory stability after repeated opening/closing
```

Consider lazy initialization and persistent in-memory indexing where useful.

A separate daemon is not required initially; Luna itself is already a persistent shell process and can keep launcher indexes warm.

This is an important difference from standalone launchers that need a daemon merely to achieve fast cold activation.

---

# MVP

The first Launcher milestone includes only:

```text
ApplicationsProvider
.desktop discovery
normalized application index
fuzzy search
icons
keyboard navigation
primary application launch action
Escape / click-outside dismissal
Notch resize/animation
```

Explicitly defer:

```text
WindowsProvider
CalculatorProvider
FilesProvider
CommandsProvider
ClipboardProvider
WebProvider
extensions/plugins
provider marketplace
runtime dynamic loading
```

The provider architecture should exist from the beginning, but only the application provider needs to be production-ready for the first milestone.

---

# Future UX direction

Long term, the Launcher may evolve toward a Raycast/Vicinae-style model:

```text
query
↓
results
↓
primary action
↓
secondary actions
↓
subviews / commands
```

Possible examples:

```text
Firefox
├── Open
├── Open new window
├── Pin
└── Show application info

Window result
├── Focus
├── Move to workspace
└── Close

System command
├── Lock
├── Suspend
└── Power off
```

These actions must remain typed application intents, not arbitrary shell command strings.

---

# Invariants

1. `luna-module-launcher` is an independent module crate.
2. The Notch hosts the Launcher; the Launcher does not own a Wayland surface.
3. The Launcher uses GPUI Kit as its primary UI/component foundation.
4. Search providers are separate from visual rendering.
5. Application execution does not occur directly in widgets.
6. Hyprland-specific operations remain behind compositor contracts.
7. Async provider results from stale queries are discarded.
8. Application metadata is indexed/cached rather than reparsed per keystroke.
9. The first implementation uses statically linked providers, not dynamic plugins.
10. zlaunch is the primary GPUI implementation reference; Anyrun/Walker inform provider architecture; Vicinae informs UX.
11. External projects are references, not architectural authorities.
12. Performance decisions are based on measurements in Luna's persistent-shell environment.
