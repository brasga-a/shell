# Player Module

## Crate

`crates/modules/player` → package `shell-module-player`

## Purpose

Expose active media playback inside the Notch using MPRIS-backed state from the Linux service layer.

## Responsibilities

```text
active player selection
track metadata presentation
play/pause
previous/next
playback state
optional progress/seek when supported
```

## Dependencies

Consumes a media/MPRIS port from the service layer. The module must not talk directly to D-Bus from GPUI widgets.

## State

```text
active player
track metadata
playback status
progress/duration when available
```

## Notch behavior

Compact and expanded layouts are owned by the module; the Notch owns geometry transitions and surface behavior.

## MVP

```text
show active track
show artist/title
play/pause
previous/next
handle no active player gracefully
```
