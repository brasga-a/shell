# Launcher Module

## Crate

`crates/modules/launcher` → package `shell-module-launcher`

## Purpose

Application discovery, search, ranking, selection and launch intent. The module renders inside the Notch and should be the first module used to validate module hosting.

## Responsibilities

```text
discover desktop applications
normalize application metadata
index/search applications
keyboard navigation
selection
launch intent
icon resolution requests
```

## Dependencies

Consumes application discovery/execution contracts and shared UI primitives. It must not invoke `hyprctl`, shell commands or desktop entry executables directly from widgets.

## State

```text
query
results
selected index
loading/error state
```

## Notch behavior

The Launcher reports/render its content layout; the Notch owns the resulting resize and transition.

## MVP

```text
open launcher
type query
navigate results
launch selected application
Escape / click outside dismiss
```
