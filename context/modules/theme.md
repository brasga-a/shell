# Theme Module

## Crate

`crates/modules/theme` → package `luna-module-theme`

## Purpose

Provide the user-facing **theme selector/editor inside the Notch** while keeping the actual design-system model centralized in `shell-theme`.

The Theme module edits and previews shared theme tokens. It does not own a second theme engine, a private color system, or per-module styling rules.

---

# Reference applications and architecture

## KDE Plasma Global Themes / Look-and-Feel

KDE Plasma separates appearance into reusable theme/configuration layers such as color schemes, icons, cursors and global themes. A global theme can coordinate multiple visual resources without requiring every application/widget to define its own styling independently.

Useful lesson for Luna:

```text
shared visual model
├── colors
├── typography
├── spacing
├── radius
├── motion
├── shadows
└── icons
```

and a Theme UI edits/selects that shared model.

Reference:

- https://develop.kde.org/docs/plasma/theme/
- https://develop.kde.org/docs/plasma/look-and-feel/

## GNOME appearance / Tweaks model

GNOME tooling reinforces the distinction between:

```text
settings UI
→ writes configuration/preferences
→ desktop/toolkit consumes those preferences
```

rather than styling each feature directly from the settings widget.

Useful lesson:

> Theme editing belongs in a controller surface; rendering consumes the centralized theme state elsewhere.

---

# Shell vs full app boundary

The Notch Theme module should be optimized for quick changes:

```text
select installed theme
light/dark preference
accent/color preset
radius preset
motion preference
preview current theme
reset to defaults
```

A future full Appearance/Theme app becomes appropriate for:

```text
full token editor
palette construction
advanced typography
icon/cursor packs
wallpaper integration
import/export theme packs
per-display wallpaper/theme behavior
community themes
full preview workspace
```

Rule:

> The module chooses and tunes. A full appearance app authors and manages theme systems deeply.

---

# Core boundary

`luna-module-theme` is **not** `shell-theme`.

```text
shell-theme
├── Theme model
├── semantic tokens
├── validation
├── defaults
└── conversion helpers
      │
      ▼
luna-module-theme
├── selection UI
├── editing UI
├── preview state
└── apply/reset commands
```

The module must never become the only place where token definitions exist.

---

# Target architecture

```text
theme.toml
    │
    ▼
shell-config
    │ validated config
    ▼
shell-theme
├── semantic Theme model
├── defaults
└── validation
    │
    ├───────────────┐
    ▼               ▼
shell-ui-gpui   Theme module
    │               │
    │ translate     │ edit/select
    ▼               ▼
GPUI Kit Theme  SettingsCommand
    │               │
    └───────┬───────┘
            ▼
      application config flow
```

This keeps GPUI Kit as presentation infrastructure while `shell-theme` remains renderer-independent.

---

# Semantic tokens

Luna should use semantic tokens instead of component-specific colors.

Example:

```text
background
surface
surface_raised
foreground
foreground_muted
border
accent
accent_foreground
danger
warning
success
```

Plus structural tokens:

```text
spacing
radius
typography
shadows
motion
icons
```

Avoid tokens such as:

```text
launcher_search_bar_gray
calendar_event_blue
player_button_hover
```

unless they represent a real reusable semantic role.

---

# GPUI Kit integration

GPUI Kit is the presentation foundation.

The preferred dependency direction is:

```text
shell-theme
   ↓ semantic values
shell-ui-gpui
   ↓ conversion
GPUI Kit theme/components
```

`luna-module-theme` may use GPUI Kit components directly for its interface, but edits `shell-theme`/configuration values rather than mutating GPUI Kit internals as the persistent source of truth.

Conceptually:

```rust
fn to_gpui_kit_theme(theme: &Theme) -> GpuiKitTheme {
    // presentation mapping only
}
```

Exact API depends on the pinned GPUI Kit version.

---

# Theme presets

Themes should have stable metadata:

```text
id
name
author
version
base_mode
```

A preset resolves to validated semantic tokens.

Initial built-ins might include:

```text
Luna Dark
Luna Light
System
```

The architecture should allow additional presets later without hardcoding each preset into module rendering logic.

---

# Draft and preview model

Theme editing benefits from temporary preview before persistence.

Conceptual flow:

```text
active Theme
   ↓ clone
ThemeDraft
   ↓ user edits
validated preview
   ↓
live UI preview
   ├── Apply → persist through ConfigPort
   └── Cancel → restore active Theme
```

The preview must still pass validation. Do not let malformed values enter global rendering state.

If live preview is enabled, every module should update from the same preview snapshot rather than the Theme module locally faking its appearance.

---

# Hot reload

External edits to `theme.toml` remain supported:

```text
filesystem event
→ debounce
→ load complete config
→ validate
→ atomic config/theme snapshot
→ ThemeChanged
→ GPUI Kit theme mapping updates
→ shell/modules rerender
```

Theme module writes should use the same pipeline instead of introducing a parallel write path.

---

# System integration

Luna may eventually expose compatibility options for external desktop appearance:

```text
GTK theme
Qt theme
icon theme
cursor theme
wallpaper
```

These are separate system-integration concerns and should not be conflated with Luna's internal design tokens.

Possible future layering:

```text
Luna Theme
├── internal shell theme
└── optional system appearance adapters
    ├── GTK
    ├── Qt
    ├── icons
    └── cursors
```

The internal Luna shell must remain visually coherent even when external app theming cannot be perfectly synchronized.

---

# State

Presentation state:

```text
selected_theme_id
ThemeDraft
preview_enabled
active_editor_section
validation_errors
```

Application state:

```text
active validated Theme
available presets
config persistence state
```

---

# Notch UX

Recommended quick editor:

```text
┌────────────────────────────────┐
│ Appearance                     │
│                                │
│ Theme       [ Luna Dark      ▾]│
│ Mode        [ Dark / Light ]   │
│ Accent      ● ● ● ● ●          │
│ Radius      [ Compact — Round ]│
│ Motion                       ✓ │
│                                │
│ [Reset]            [Apply]     │
│                  Open Appearance│
└────────────────────────────────┘
```

The Notch should prioritize presets and high-value controls over exposing every token.

---

# GPUI Kit components

Useful directly:

```text
Select
Tabs
Button
Switch
Slider
Popover
Dialog
Tooltip
ScrollView
Color-related input if available
Presence / transition
```

Luna-specific components may include:

```text
ThemePreview
PaletteSwatch
TokenPreviewRow
ThemePresetCard
MotionPreview
```

Do not create wrappers unless Luna needs behavior or styling that cannot be expressed through normal GPUI Kit composition/theming.

---

# Future full Appearance app

A future `luna-appearance` or expanded `luna-settings` app can provide advanced theme authoring.

It must reuse:

```text
shell-theme models
same presets
same validation
same persistence
same preview mechanism
```

rather than inventing a separate theme format.

---

# MVP

## MVP 1

```text
Luna Dark / Light selection
accent selection
radius preference
motion enable/disable
live preview
apply/reset
```

## MVP 2

```text
additional semantic token editing
custom presets
import/export Luna theme file
```

## Later

```text
external GTK/Qt/icon/cursor integration
wallpaper coordination
community/theme gallery
full appearance editor
```

---

# Completion criteria

- `shell-theme` remains renderer-independent;
- Theme module is only a presentation/controller crate;
- GPUI Kit receives theme values through an adapter/mapping layer;
- all modules consume the same active theme snapshot;
- preview can be cancelled without corrupting persistent config;
- external `theme.toml` hot reload and Theme module edits converge on the same configuration pipeline;
- no module owns private duplicated design tokens without architectural justification.

---

# Non-goals

```text
forking GPUI Kit's complete styling system
per-module independent themes
hardcoding colors in every module
full GTK/Qt theming in MVP
wallpaper manager inside the Theme Notch module
CSS-like arbitrary runtime styling language
```
