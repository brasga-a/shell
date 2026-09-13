# report.md

# Configuration Hot Reload Report

## Context

The shell uses modular TOML configuration files under:

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

Even though Rust is a compiled language, the shell can read and monitor configuration files dynamically at runtime.

No recompilation is required when a TOML file changes.

---

## Decision

The shell should monitor the entire configuration directory in real time and hot-reload configuration when files change.

Preferred stack:

```text
notify
notify-debouncer-mini
serde
toml
ArcSwap
```

On Linux, `notify` uses native filesystem event mechanisms such as `inotify`.

---

## Architecture

```text
~/.config/atlantic/
        │
        ▼
ConfigWatcher
        │
 filesystem events
        ▼
Debouncer
        │
        ▼
ConfigLoader
        │
 parse all config
        ▼
ConfigValidator
        │
        ├── valid
        │     ↓
        │  ConfigSnapshot
        │     ↓
        │  atomic swap
        │     ↓
        │  ConfigChanged
        │     ↓
        │  ShellState / UI
        │
        └── invalid
              ↓
           diagnostic
              ↓
      keep previous config
```

---

## Watch the directory, not individual files

The shell should watch:

```text
~/.config/atlantic/
```

recursively.

This automatically covers:

```text
config.toml
theme.toml
keybinds.toml
modules/*.toml
```

and any future known module configuration files.

Conceptually:

```rust
watcher.watch(
    config_dir,
    RecursiveMode::Recursive,
)?;
```

The watcher should not need to be restarted every time a supported module config is added.

---

## Why debounce is required

Editors do not always save a file with one simple write.

A single save can generate sequences such as:

```text
modify
modify
close
```

or:

```text
create temporary file
write temporary file
rename temporary file
remove old file
```

Reloading immediately on every event can therefore cause:

```text
duplicate reloads
partial reads
temporary parse errors
unnecessary UI updates
```

The watcher should debounce filesystem events before reloading.

Recommended initial debounce interval:

```text
~200 ms
```

A practical range is:

```text
100–300 ms
```

The exact value may later be tuned through measurement.

---

## Reload strategy

The initial implementation should use a **full configuration reload**.

```text
any supported config file changes
        ↓
reload complete configuration tree
```

This is preferable to prematurely implementing partial dependency tracking because TOML configuration files are expected to remain small.

Conceptually:

```rust
let candidate = ConfigLoader::load_all(config_dir)?;
let validated = ConfigValidator::validate(candidate)?;
config_manager.replace(validated);
```

---

## Atomic replacement

The active configuration must never be mutated incrementally while files are being parsed.

Recommended model:

```rust
struct ConfigManager {
    current: ArcSwap<ValidatedConfig>,
}
```

Flow:

```text
old valid configuration
        │
        ▼
parse candidate configuration
        │
        ▼
validate complete snapshot
        │
        ├── success
        │      ↓
        │   atomic replace
        │
        └── failure
               ↓
        preserve old snapshot
```

This prevents the shell from observing a half-updated configuration.

---

## Invalid configuration handling

A live configuration editor frequently passes through invalid intermediate states.

For example:

```toml
height =
```

may exist briefly while the user is typing.

That must not break the running shell.

Required behavior:

```text
new config valid
→ activate it

new config invalid
→ reject it
→ log/report error
→ continue using previous valid config
```

The shell must remain usable at all times.

---

## Configuration snapshots

The application should consume a validated configuration snapshot rather than reading TOML files directly.

Conceptually:

```rust
struct ValidatedConfig {
    general: GeneralConfig,
    theme: ThemeConfig,
    keybinds: KeybindConfig,
    modules: ModuleConfig,
}
```

The config directory remains modular on disk, while runtime configuration appears as one coherent object.

---

## Event propagation

After a successful reload:

```text
ConfigWatcher
      ↓
ConfigLoader
      ↓
ValidatedConfig
      ↓
ConfigManager
      ↓
ConfigChanged
      ↓
application state
      ↓
affected UI rerender
```

Widgets must not read files directly.

Forbidden:

```text
Bar widget
    ↓
read modules/bar.toml
```

Required:

```text
ConfigManager
    ↓
BarConfig
    ↓
Bar UI
```

---

## Initial implementation

The first version should remain intentionally simple:

```text
watch whole directory
debounce
reload all TOML files
validate complete configuration
atomic replace
emit one ConfigChanged event
```

Do not initially implement:

```text
per-property filesystem subscriptions
dependency graphs between config files
incremental TOML patching
custom file watcher
```

Those add complexity without meaningful benefit at the expected configuration size.

---

## Future optimization

If configuration grows significantly, reload events may later become more granular.

Example:

```text
theme.toml
    ↓
ThemeChanged

modules/bar.toml
    ↓
BarConfigChanged

keybinds.toml
    ↓
KeybindsChanged
```

This optimization should only be introduced if full reload becomes measurably problematic.

---

## Threading and runtime

Filesystem events should be processed outside the GPUI render path.

Recommended flow:

```text
notify watcher thread/task
        ↓
debouncer
        ↓
config parsing/validation
        ↓
application message
        ↓
UI thread/state update
```

Background filesystem tasks must not mutate GPUI state directly.

---

## Failure handling

The watcher must tolerate:

```text
config directory temporarily missing
file renamed during save
partial writes
permission changes
invalid TOML
unknown module files
temporary filesystem errors
```

Expected policy:

```text
recover when possible
keep last known-good config
emit structured diagnostics
never crash the shell for a config edit
```

---

## Logging

Configuration reload events should use structured logging.

Example context:

```text
event = config_reload
path = ~/.config/atlantic/theme.toml
result = success
duration_ms = ...
```

On failure:

```text
event = config_reload
result = rejected
error = parse_error
path = ...
```

Do not spam logs for every low-level inotify event after debounce.

---

## Invariants

1. Configuration changes do not require recompilation.
2. The shell watches the configuration directory at runtime.
3. File events are debounced.
4. Configuration is parsed and validated before activation.
5. Runtime configuration is replaced atomically.
6. Invalid reloads preserve the previous valid configuration.
7. Widgets never read TOML files directly.
8. Filesystem watcher tasks do not mutate UI state directly.
9. Missing or malformed config must not crash the shell.
10. Full reload is preferred until incremental reload is proven necessary.

---

## Recommended implementation stack

```text
notify
    ↓
filesystem watching / inotify backend

notify-debouncer-mini
    ↓
event coalescing

serde
    ↓
typed deserialization

toml
    ↓
TOML parsing

ArcSwap
    ↓
atomic configuration snapshot replacement

tracing
    ↓
diagnostics
```

---

## Final decision

The shell will support **real-time configuration hot reload**.

The initial implementation strategy is:

```text
watch directory
→ debounce filesystem events
→ reload complete modular TOML configuration
→ validate
→ atomically replace current snapshot
→ emit configuration change
→ update affected shell UI
```

If the new configuration is invalid, the shell keeps the previous valid configuration and reports the error without interrupting the desktop session.
