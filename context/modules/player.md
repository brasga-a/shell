# Player Module

## Crate

`crates/modules/player` → package `shell-module-player`

## Purpose

Provide fast control over the **currently active media session** inside the Notch. The Player module is a shell controller, not a full music-library application.

It should work with any Linux application exposing MPRIS rather than owning playback itself.

---

# Reference applications and architecture

## MPRIS

MPRIS defines a common D-Bus interface for media players, including playback state, metadata, position, play/pause, next/previous, seek and player capabilities.

This is the primary architectural contract Luna should target.

Reference:

- https://specifications.freedesktop.org/mpris-spec/latest/

## Amberol

Amberol is intentionally focused on local music playback with a small, direct UI. The useful lesson for Luna is not to overload the shell controller with library-management concerns.

Reference:

- https://apps.gnome.org/Amberol/

## Elisa

KDE Elisa separates playback control and library/browsing concerns. Luna should follow the same conceptual boundary: the Notch exposes transport and session state; a future full media app may own collection browsing.

Reference:

- https://apps.kde.org/elisa/

---

# Shell vs app boundary

The Notch should own quick media actions:

```text
current track
artist/title/artwork
play/pause
previous/next
seek
volume shortcut if appropriate
switch active player
```

A standalone media application becomes appropriate for:

```text
music library
album/artist browsing
playlists
queue management
local file import
streaming-provider browsing
lyrics workspace
large queue editing
```

Rule:

> Player module controls an existing media session. A media app owns a media library/workspace.

---

# Target architecture

```text
MPRIS players
     │ D-Bus
     ▼
MediaService
├── player discovery
├── active-player policy
├── metadata normalization
├── capability mapping
└── command dispatch
     │
     ▼
MediaState / MediaPort
     │
     ▼
luna-module-player
     │
     ▼
Notch
```

The GPUI module must never open its own D-Bus connection.

---

# Active player policy

Multiple MPRIS players may exist simultaneously.

Luna needs deterministic selection rules, for example:

```text
1. currently Playing player
2. most recently interacted/changed player
3. user-selected pinned player
4. fallback to first available player
```

The exact policy belongs in `MediaService`, not inside the view.

The UI may expose a player switcher when multiple sessions exist.

---

# Normalized media model

Provider-specific MPRIS data should be converted into a stable internal model.

Conceptually:

```text
MediaPlayer
├── id
├── identity
├── desktop_entry
├── playback_status
├── capabilities
└── metadata

TrackMetadata
├── track_id
├── title
├── artists[]
├── album
├── artwork_uri
├── length
└── url
```

Capabilities should be explicit:

```text
can_play
can_pause
can_go_next
can_go_previous
can_seek
can_control
```

Disabled controls should follow capabilities rather than guessing based on player identity.

---

# Position and progress

MPRIS position updates should not force an IPC request every frame.

Recommended model:

```text
last_position
last_position_timestamp
playback_status
rate
        ↓
derive visible progress locally
        ↓
periodic/event resync
```

This keeps the UI smooth while preserving authoritative service state.

Seeking flow:

```text
Slider interaction
→ MediaCommand::Seek(...)
→ MediaPort
→ MPRIS adapter
→ service event/state confirmation
```

---

# Artwork

Artwork should be loaded asynchronously and cached outside the hot render path.

Required behavior:

```text
remote/file URI normalization
async image load
decode/cache
fallback artwork
cancel stale artwork loads when track changes
```

The module should not perform synchronous network/file reads during render.

---

# State

Presentation state:

```text
expanded/collapsed content state
player switcher open
seek-drag state
artwork loading state
```

Service state:

```text
available players
active player
track metadata
playback status
position/duration
capabilities
```

---

# Notch UX

Compact expanded view:

```text
┌────────────────────────────────┐
│ [art]  Song Title              │
│        Artist                  │
│                                │
│       ◀   ▶/❚❚   ▶             │
│  1:24 ━━━━━━━━━━━━━━━ 3:46     │
└────────────────────────────────┘
```

Optional richer mode:

```text
artwork
metadata
transport
seek
output/player switcher
```

The Notch owns final host geometry and module transitions.

---

# GPUI Kit usage

Useful GPUI Kit components:

```text
Button
IconButton
Slider
Popover
Dropdown / Select
Tooltip
Avatar/Image container
Presence
transition/spring
```

Luna-specific components:

```text
ArtworkView
TrackMetadataView
TransportControls
PlaybackProgress
PlayerSwitcher
```

---

# Future full media app

A future `luna-music` or `luna-media` should not be required for the Player module to work.

If created:

```text
media-core/library
├── Player module
└── full media app
```

The Player module remains a controller for system-wide active sessions even when the user plays media in Firefox, Spotify, VLC or another application.

---

# MVP

## MVP 1

```text
MPRIS player discovery
active-player selection
title/artist
play/pause
previous/next
no-player state
```

## MVP 2

```text
artwork
position/duration
seek
multiple-player switcher
```

## Later

```text
output/device shortcuts
volume integration
queue glimpse when exposed
full media app integration
```

---

# Completion criteria

- module owns no D-Bus connection;
- MediaService is authoritative for active-player state;
- control availability follows reported capabilities;
- progress remains smooth without polling MPRIS every frame;
- artwork loading does not block rendering;
- no-player and disappearing-player races are handled gracefully.

---

# Non-goals

```text
music library ownership
streaming service integration
playlist database
audio decoding
playback engine
speaker routing backend
```
