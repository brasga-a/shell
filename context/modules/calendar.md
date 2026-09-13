# Calendar Module

## Crate

`crates/modules/calendar` → package `shell-module-calendar`

## Purpose


Provide a **fast calendar surface inside the Notch** for checking dates, seeing the next events and creating simple events without turning the Notch into a full productivity application.


The Calendar domain is expected to eventually have both a shell surface and a full application:

```text
Calendar domain
├── luna-module-calendar
│   └── glance + quick actions
└── luna-calendar
    └── deep calendar workflow
```

The module and future app must share the same calendar data/domain layer rather than maintain separate databases or synchronization logic.

---

## Shell vs full app boundary

The Notch module is optimized for interactions that should take seconds:

```text
check today
check this month
see upcoming events
jump between dates
quick-create an event
open an existing event summary
```

The following belong to a future `luna-calendar` application:

```text
full day/week/month workspace
hour-by-hour week grid
large multi-calendar sidebar
drag and drop
resize events
advanced recurrence editing
attendee management
meeting scheduling
large event editor
account management
sync diagnostics
```

Rule:

> The module answers “what is happening and what can I do quickly?”. The app owns sustained calendar work.

---

# Reference applications and architecture

## GNOME Calendar + Evolution Data Server

GNOME's calendar ecosystem separates the presentation application from Evolution Data Server (EDS), which provides shared calendar/task data access through client libraries such as `libecal` and backend implementations.

Useful lesson for Luna:

```text
UI must not own remote calendar protocols
UI talks to calendar service/domain
backend owns storage/sync/provider details
```

EDS also exposes client-side views/change notifications, reinforcing the event-driven model Luna should use instead of making the calendar UI repeatedly query remote services.

References:

- https://gnome.pages.gitlab.gnome.org/evolution-data-server/libecal/
- https://wiki.gnome.org/Apps%282f%29Evolution%282f%29EDS_Architecture.html

## KDE Merkuro + Akonadi

Merkuro uses Akonadi as shared PIM infrastructure. Akonadi sits between applications and local/remote resources and provides the core operations required to fetch, create, edit and delete events while keeping remote resources synchronized.

Merkuro supports local calendars and providers such as Google Calendar, Outlook, Nextcloud and CalDAV without embedding each provider directly into its presentation layer.

Useful lesson for Luna:

```text
Calendar UI
      ↓
shared calendar model/cache
      ↓
synchronization resources/adapters
      ↓
remote services
```

Luna should adopt the separation, but with a smaller Rust-native local-first implementation rather than reproducing Akonadi as a general PIM server.

References:

- https://apps.kde.org/merkuro/
- https://community.kde.org/KDE_PIM/Akonadi/Architecture
- https://github.com/KDE/merkuro

---

# Target Luna architecture

The long-term Calendar architecture should be local-first:

```text
Google Calendar ─────┐
Microsoft Outlook ───┼── provider adapters
CalDAV / iCloud ─────┘
          │
          ▼
      SyncEngine
          │
          ▼
   CalendarRepository
          │
          ▼
       SQLite
          │
          ▼
     CalendarCore
       ├── event queries
       ├── mutations
       ├── recurrence
       └── notifications
          │
     ┌────┴───────────┐
     ▼                ▼
Notch module      future app
```

The UI should always read from the local repository. Remote APIs update the local store through synchronization.

This gives Luna:

```text
instant local reads
offline operation
one source of truth
shared data between shell and app
provider-independent UI
```

---

# Proposed shared calendar crates

The calendar module itself should not eventually contain all calendar infrastructure.

A likely split is:

```text
crates/
├── calendar-core/
│   ├── models
│   ├── recurrence
│   ├── repository contracts
│   └── sync contracts
│
├── calendar-sqlite/
│   └── local repository
│
├── calendar-google/
├── calendar-microsoft/
├── calendar-caldav/
│
├── modules/calendar/
│   └── Notch presentation
│
└── apps/calendar/            # future
    └── full calendar UI
```

These names are architectural direction, not a requirement to create every crate during the MVP.

---

# Data model

The domain should use an internal Luna identifier even for remote events.

Conceptual entities:

```text
Account
Calendar
Event
Attendee
Reminder
RecurrenceRule
SyncState
```

Conceptual `Event` fields:

```text
id                    # Luna-owned ID
calendar_id
remote_id             # optional provider ID

title
description
location

start_at
end_at
timezone
all_day

recurrence_rule
recurrence_parent

status
created_at
updated_at

sync_status
remote_etag
```

Never make a Google/Microsoft/CalDAV remote ID the primary domain identity.

A local-only event must be a first-class event, not a special provider hack.

---

# SQLite persistence

SQLite is the intended local source of truth.

Initial conceptual tables:

```text
accounts
calendars
events
event_attendees
event_reminders
recurrence_rules
sync_state
```

The repository layer owns SQL. GPUI widgets must never execute SQL directly.

Possible flow:

```text
CalendarModule
   ↓ CalendarQuery / CalendarCommand
CalendarCore
   ↓
CalendarRepository
   ↓
SQLite
```

---

# Provider / synchronization model

Provider-specific APIs stay behind a common interface.

Conceptually:

```rust
trait CalendarProvider {
    async fn calendars(&self) -> Result<Vec<RemoteCalendar>>;
    async fn initial_sync(&self, calendar: CalendarId) -> Result<SyncBatch>;
    async fn sync(&self, cursor: SyncCursor) -> Result<SyncBatch>;

    async fn create_event(&self, event: &Event) -> Result<RemoteEvent>;
    async fn update_event(&self, event: &Event) -> Result<RemoteEvent>;
    async fn delete_event(&self, event: &Event) -> Result<()>;
}
```

Initial target adapters:

```text
GoogleCalendarProvider
MicrosoftGraphProvider
CalDavProvider
```

`CalDavProvider` is strategically important because it can cover iCloud and other standards-based services such as Nextcloud, Fastmail, Radicale and similar servers.

Synchronization is not UI behavior. A `SyncEngine` owns:

```text
initial sync
incremental sync
provider cursor/token persistence
offline mutation queue
remote deletion handling
conflict policy
retry/backoff
sync diagnostics
```

The UI observes synchronization state through application events.

---

# Difficult calendar semantics

The architecture must explicitly prepare for:

```text
recurring events
recurrence exceptions
timezones
DST transitions
all-day events
remote deletions
offline edits
conflicts
provider token invalidation
multiple accounts
multiple calendars
```

Do not encode recurrence as “duplicate rows forever”. Preserve recurrence rules and materialize occurrences only where useful for querying/rendering.

---

# Calendar module responsibilities

`luna-module-calendar` owns only the Notch presentation and local interaction state:

```text
visible month
selected date
month navigation
today agenda
upcoming event cards
calendar visibility filters
quick-create UI
quick edit/delete intent
sync-state indicator
Open Calendar action
```

It does not own:

```text
SQLite connection
OAuth tokens
Google API client
Microsoft Graph client
CalDAV client
recurrence engine
background synchronization
```

---

# State

Presentation state:

```text
visible_month
selected_date
selected_event
quick_editor_state
visible_calendar_ids
loading/error state
```

Application/domain state is supplied through calendar contracts:

```text
today events
selected-date events
upcoming events
calendar metadata
sync status
```

---

# Notch UX

Recommended expanded surface:

```text
┌────────────────────────────────┐
│ September 2026           ‹  ›  │
│ M  T  W  T  F  S  S            │
│ 31  1  2  3  4  5  6          │
│     •     ••                    │
│                                │
│ Today                          │
│ 09:00  Daily                   │
│ 13:30  Class                   │
│ 18:00  Meeting                 │
│                                │
│ + New event      Open Calendar │
└────────────────────────────────┘
```

The module should not reproduce a full seven-column week workspace inside the Notch.

The Notch owns final geometry, transition animation, input region and dismissal.

---

# GPUI Kit usage

Use GPUI Kit where appropriate for:

```text
Button
IconButton
Popover
Dialog / quick editor surface
Input
Select
Checkbox
ScrollView
Tooltip
Presence / motion
```

Calendar-specific visual primitives remain Luna-owned:

```text
MiniMonthGrid
DayCell
AgendaList
EventCard
CalendarDot
```

A future full calendar app will add custom structures such as:

```text
WeekGrid
TimeRuler
EventBlock
MonthGrid
DragResizeController
```

---

# App integration

The module should expose an explicit action:

```text
Open Calendar
```

Future behavior:

```text
CalendarModule
    ↓ open_app(Calendar, optional_date/event)
luna-calendar
```

The app should accept context from the shell so opening an event or selected date does not lose the user's current intent.

---

# MVP

## MVP 1 — local Notch calendar

```text
current month
month navigation
today highlight
selected date
agenda for selected date
SQLite-backed local events
quick-create simple event
quick delete/edit basic fields
```

## MVP 2 — synchronization

Recommended order:

```text
Google Calendar
Microsoft Outlook / Microsoft 365
CalDAV / iCloud
```

## Later — full Luna Calendar app

```text
Day view
Week view
Month view
Schedule view
multi-calendar sidebar
drag/drop
resize
advanced recurrence
account management
```

---

# Completion criteria

The module is architecturally healthy when:

- it renders entirely from local/application state;
- it remains usable without a network connection;
- remote provider code is absent from the module crate;
- SQLite access is absent from GPUI widgets;
- the Notch module stays useful without becoming a full calendar workspace;
- the same domain/repository can later serve `luna-calendar` without migration to a second storage model.

---

# Non-goals for the module

```text
full calendar desktop workspace
provider OAuth implementation
raw CalDAV protocol implementation
advanced meeting scheduling
full task manager
complete recurrence editor
per-provider UI logic
```
