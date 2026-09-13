# Clock Module

## Crate

`crates/modules/clock` → package `luna-module-clock`

## Purpose

Provide time/date presentation and clock-specific interaction inside the Notch while remaining independent from panel placement.

## Responsibilities

```text
current time
current date
12h/24h formatting
timezone display
optional timer/alarm expansion later
```

## Dependencies

Consumes time/configuration contracts only. The clock module should remain lightweight and must not own a dedicated background process.

## State

```text
current time snapshot
format preference
timezone preference
optional expanded clock state
```

## Notch behavior

The module may expose compact and expanded clock views; the Notch owns host resizing and animation.

## MVP

```text
show current time
show date
respect 12h/24h preference
update without blocking UI
```
