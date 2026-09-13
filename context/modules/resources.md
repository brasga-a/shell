# Task Manager / Resources Module

## Crate

`crates/modules/resources` → package `luna-module-resources`

## Purpose

Provide a lightweight task-manager/resource view inside the Notch for observing system usage and, later, managing processes through explicit service contracts.

## Responsibilities

```text
CPU usage
memory usage
GPU usage when available
disk/network activity
process list
sorting/filtering
future process actions through a dedicated port
```

## Dependencies

Consumes system-resource/process ports from the Linux service layer. Widgets must not scrape `/proc`, execute `ps`, or send process signals directly.

## State

```text
resource snapshots
process list
sort/filter state
selected process
loading/error state
```

## Notch behavior

This module may require one of the largest Notch layouts. The module owns content/layout intent; the Notch owns final geometry transitions and surface/input semantics.

## MVP

```text
CPU usage
memory usage
process list
sort by CPU/memory
periodic event-driven or bounded sampling
```
