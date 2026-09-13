# Calendar Module

## Crate

`crates/modules/calendar` → package `shell-module-calendar`

## Purpose

Provide date/calendar information inside the Notch, with room for future agenda integration (SQLite) without coupling the shell to a specific calendar provider.

## Responsibilities

```text
current date
month/week navigation
calendar grid
date selection
future agenda/event presentation through ports
```

## Dependencies

Consumes time/date services and, if introduced later, a calendar/agenda port. Provider-specific APIs must remain outside the module UI.

## State

```text
visible month
selected date
optional agenda data
loading/error state
```

## Notch behavior

The Calendar renders its own content; the Notch owns size, animation, focus and dismissal.

## MVP

```text
show current month
navigate months
select date
return to current date
```
