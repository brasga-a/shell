# Clock Module

## Crate

`crates/modules/clock` → package `luna-module-clock`

## Purpose

Provide fast access to **current time, date, world clocks, timers and alarms** inside the Notch without turning the shell into a permanently running clock application.

The Clock module is a shell quick-surface. A future standalone Clock app is optional and only justified if alarm/timer workflows become deep enough to need their own workspace.

---

# Reference applications and architecture

## GNOME Clocks

GNOME Clocks separates four user-facing capabilities:

```text
World
Alarm
Stopwatch
Timer
```

That separation is useful for Luna because these are different state machines rather than one generic "clock" feature.

Useful lesson:

```text
Clock shell surface
├── time/date presentation
├── world clocks
├── alarm state
├── timer state
└── stopwatch state
```

but each capability should own its own state and lifecycle.

Reference:

- https://apps.gnome.org/Clocks/

---

# Shell vs app boundary

The Notch is appropriate for:

```text
current time/date
secondary time zones
start/pause/reset timer
start/pause/reset stopwatch
see next alarms
quick alarm creation
```

A future standalone app becomes justified only for:

```text
large alarm management
many world clocks
complex recurring alarms
rich timer presets
long-running activity history
```

Unlike Calendar, a Clock app is **not required by default**. Most clock workflows fit naturally inside the shell.

---

# Proposed architecture

```text
System clock / monotonic clock
             │
             ▼
        ClockService
       ┌─────┼─────────┐
       │     │         │
       ▼     ▼         ▼
    Timer   Alarm   Stopwatch
       │     │         │
       └─────┴────┬────┘
                  ▼
          luna-module-clock
                  │
                  ▼
                Notch
```

The module must distinguish wall-clock time from elapsed-duration timing.

Use:

```text
wall clock
→ local date/time/timezone display

monotonic clock
→ timer/stopwatch elapsed duration
```

Timer correctness must not depend on frame rate or repeated `sleep(1s)` increments.

---

# Responsibilities

The module owns:

```text
current time/date presentation
12h/24h formatting
timezone presentation
world-clock list presentation
timer interaction state
stopwatch interaction state
alarm presentation/quick editing
```

The underlying clock/alarm service owns:

```text
reliable timer deadlines
alarm scheduling
persistence
resume after UI close
suspend/resume reconciliation
system notification trigger
```

The module must not keep a long-running timer alive merely because a GPUI view remains mounted.

---

# State model

Presentation state:

```text
active_tab
selected_world_clock
alarm_editor_state
```

Service/domain state:

```text
current_datetime
configured_timezones
active_timer
stopwatch_state
alarms
next_alarm
```

Conceptual timer state:

```text
Idle
Running { deadline }
Paused { remaining }
Finished
```

Conceptual stopwatch state:

```text
Idle
Running { started_at, accumulated }
Paused { accumulated }
```

These should be derived from timestamps, not mutable counters updated every second.

---

# Persistence

Simple preferences belong in configuration:

```text
12h/24h
show_seconds
world clock timezones
```

Persistent alarm/timer state should use a dedicated storage abstraction if/when alarms become real system features.

Do not make `clock.toml` a live database of elapsed milliseconds.

---

# Notifications and background behavior

Timers and alarms must continue to function after the Notch closes.

Flow:

```text
ClockModule
   ↓ command
ClockService
   ↓ stores deadline
background scheduling
   ↓
deadline reached
   ↓
Notification / alarm event
   ↓
Shell notification + Clock state update
```

The UI should subscribe to state changes and periodically repaint display text while visible, but timer truth remains in the service.

---

# Notch UX

Recommended default expanded layout:

```text
┌──────────────────────────────┐
│          06:42               │
│   Sunday, September 13       │
│                              │
│ São Paulo              06:42 │
│ Tokyo                   18:42 │
│                              │
│ [ Timer ] [ Alarm ] [ ... ]  │
└──────────────────────────────┘
```

Secondary views may use tabs or a segmented control:

```text
World | Alarm | Stopwatch | Timer
```

The Notch owns final size and transitions between these layouts.

---

# GPUI Kit usage

Useful components:

```text
Tabs / segmented control
Button
IconButton
Input
Select
Popover
ScrollView
Switch
Tooltip
Presence / transition / spring
```

Clock-specific UI remains Luna-owned:

```text
DigitalClock
WorldClockRow
TimerDial / TimerReadout
StopwatchReadout
AlarmRow
```

Animation should never become the source of timing truth.

---

# MVP

## MVP 1

```text
current time
current date
12h/24h preference
optional seconds
correct timezone display
```

## MVP 2

```text
world clocks
timer
stopwatch
```

## MVP 3

```text
alarms
persistence
shell notifications
suspend/resume correctness
```

---

# Completion criteria

- Clock is a standalone module crate.
- timer/stopwatch math uses timestamps rather than frame counters.
- closing the Notch does not cancel background timer/alarm state.
- GPUI widgets do not own background threads solely for timekeeping.
- module remains functional with shell theme/config hot reload.

---

# Non-goals

```text
NTP implementation
system timezone configuration
calendar event management
weather
full scheduling suite
```
