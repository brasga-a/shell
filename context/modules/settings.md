# Settings Module

## Crate

`crates/modules/settings` → package `luna-module-settings`

## Purpose

Provide **quick access to the most important Luna preferences inside the Notch** without making the Notch the complete desktop settings application.

The Settings module is a controller over validated configuration and service capabilities. It does not own TOML parsing, persistence, D-Bus clients, compositor IPC or system configuration backends.

---

# Reference applications and architecture

## KDE System Settings / KCM

KDE System Settings is built around independent configuration modules (KCMs). Each settings area has a focused responsibility while the host application handles navigation, loading and presentation.

Useful lesson for Luna:

```text
Settings host
├── Appearance
├── Input
├── Network
├── Bluetooth
├── Power
└── ...
```

should be composed from capability-specific settings sections rather than one giant controller with direct system calls.

Reference:

- https://develop.kde.org/docs/features/configuration/kcm/

## GNOME Control Center

GNOME Control Center similarly separates settings into panels backed by system services and configuration APIs. The UI is not the source of truth; it reflects and changes external settings through defined backends.

Useful lesson:

```text
settings presentation
→ typed settings/service API
→ authoritative backend
```

rather than:

```text
settings widget
→ arbitrary filesystem/CLI mutation
```

---

# Shell vs full app boundary

The Notch module should expose settings that are frequently toggled or adjusted quickly:

```text
appearance shortcut
module enable/disable
animation toggle
compact theme options
clock format
basic panel/notch behavior
key preference shortcuts
quick network/bluetooth/power links
```

A future `luna-settings` app should own deeper configuration:

```text
all modules
all keybindings
accounts
network configuration
Bluetooth pairing details
power profiles
outputs/displays
input devices
advanced compositor options
accessibility
privacy
updates
system information
```

Rule:

> The Notch Settings module is a quick-control surface. The full Settings app is the configuration workspace.

---

# Target architecture

```text
Config files / system services
        │
        ▼
 shell-config + service adapters
        │
        ▼
ValidatedConfig / CapabilityState
        │
        ▼
 Settings application commands
        │
   ┌────┴────────────┐
   ▼                 ▼
Settings module   future Settings app
```

The module must never read or write TOML directly.

---

# Settings section model

The module should not hardcode a monolithic form. Use a typed section model that can grow safely.

Conceptually:

```text
SettingsSection
├── General
├── Appearance
├── Modules
├── Input
└── AdvancedShortcuts
```

Future full-app-only sections may include:

```text
Network
Bluetooth
Power
Displays
Accounts
Privacy
Accessibility
```

A section should depend only on the capability/contracts it needs.

---

# Configuration flow

Read path:

```text
ConfigLoader
→ ValidatedConfig
→ Config snapshot
→ SettingsModule
```

Write path:

```text
user edits draft
→ typed SettingsCommand
→ validation
→ persistent config update
→ filesystem write
→ ConfigWatcher
→ full candidate reload
→ validation
→ atomic snapshot swap
→ ConfigChanged
→ UI rerender
```

This preserves a single configuration pipeline.

The Settings module must not bypass hot reload by mutating live application state and separately writing disk later.

---

# Draft / apply model

Not all settings need the same commit behavior.

Classify settings by application mode:

```text
Immediate
→ safe visual preference; applies as user changes it

ApplyRequired
→ multiple related values should validate together

RecreateSurface
→ requires surface geometry/recreation

RestartService
→ backend must restart/reconnect

RestartShell
→ architecture/backend selection changed
```

Examples:

```text
theme color         → Immediate
clock format        → Immediate
notch dimensions    → RecreateSurface or validated live update
compositor backend  → RestartShell
audio backend       → RestartService initially
```

The UI should clearly communicate restart/recreate requirements rather than silently pretending every setting is live.

---

# Capability-driven UI

Settings should appear only when the system/backend supports them.

Conceptually:

```text
CompositorCapabilities
LinuxServiceCapabilities
ModuleCapabilities
```

Examples:

```text
Hyprland-only option
→ shown only when supported by active compositor adapter

battery settings
→ hidden/disabled on systems without battery/power capability
```

Do not expose unavailable settings and then fail after interaction.

---

# Module enablement

The Settings module should manage the module registry configuration:

```text
Launcher   enabled
Calendar   enabled
Player     enabled
Resources  enabled
Clock      enabled
Theme      enabled
```

Enabling/disabling a module changes configuration, while `shell-app` / application composition decides how the runtime registry reacts.

The Settings module itself must not instantiate or destroy concrete module crates directly.

---

# State

Presentation state:

```text
active_section
search_query
configuration_draft
validation_errors
dirty_fields
restart_requirements
```

External state:

```text
validated config snapshot
module registry metadata
compositor capabilities
Linux service capabilities
```

---

# Notch UX

Recommended quick settings layout:

```text
┌────────────────────────────────┐
│ Settings                       │
│                                │
│ Appearance                     │
│ Theme              Luna Dark > │
│ Animations                 [✓] │
│                                │
│ Modules                        │
│ Calendar                   [✓] │
│ Player                     [✓] │
│ Resources                  [✓] │
│                                │
│ Clock                          │
│ 24-hour format             [✓] │
│                                │
│                 Open Settings │
└────────────────────────────────┘
```

The Notch should avoid deep navigation trees and very large forms.

---

# Search

A full Settings application should eventually support search by indexing setting metadata:

```text
id
section
label
description
keywords
```

The Notch module may expose a lightweight search later, but this is not required for the first MVP.

---

# GPUI Kit usage

Useful components:

```text
Tabs / navigation
Switch
Checkbox
Select
Slider
Input
SearchInput
Button
Dialog
Popover
Tooltip
Badge
ScrollView
```

Luna-specific components:

```text
SettingsRow
SettingsSectionHeader
RestartRequirementBadge
ModuleToggleRow
CapabilityUnavailableState
```

Use GPUI Kit components directly where they fit; do not wrap every component merely to rename it.

---

# Future full Settings app

A future `luna-settings` app should share:

```text
configuration models
validation
settings commands
capability models
settings metadata
```

with the Notch module.

It should not fork into a second settings persistence mechanism.

Architecture:

```text
Settings domain/contracts
       │
  ┌────┴──────────────┐
  ▼                   ▼
Notch quick module   luna-settings app
```

---

# MVP

## MVP 1

```text
general shell preferences
module enable/disable
clock preference
animation preference
link to Theme module
apply/revert where needed
validation feedback
```

## MVP 2

```text
keybind management
panel/notch behavior
capability-driven compositor settings
restart requirement handling
```

## Later / full app

```text
network
Bluetooth
power
displays
input devices
accounts
privacy
accessibility
system info
```

---

# Completion criteria

- GPUI widgets never parse/write TOML;
- all mutations pass through typed configuration/application commands;
- invalid drafts cannot replace the active valid config;
- unavailable capabilities are represented explicitly;
- module enablement is configuration-driven;
- Settings module does not directly instantiate other modules;
- quick settings remain usable without becoming a full control-center workspace.

---

# Non-goals for the Notch module

```text
complete GNOME/KDE control-center replacement
raw NetworkManager UI
raw BlueZ UI
Hyprland config-file editor
system package/update manager
user/account administration
arbitrary text editing of TOML
```
