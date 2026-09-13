# Theme Module

## Crate

`crates/modules/theme` → package `shell-module-theme`

## Purpose

Provide the user-facing theme editor and theme controls inside the Notch. This crate is distinct from `shell-theme`: `shell-theme` defines shared theme models/tokens; `shell-module-theme` is the interactive feature that edits/selects them.

## Responsibilities

```text
theme selection
color/token editing
preview state
apply/reset actions
persist theme preferences through configuration ports
```

## Dependencies

Consumes `shell-theme` models and configuration/application ports. It must not own the global design-system implementation.

## State

```text
selected theme
editable token draft
preview state
validation errors
```

## Notch behavior

The module owns its editor layout; the Notch owns host geometry, resize animation, focus and dismissal.

## MVP

```text
list/select themes
edit primary visual tokens
preview changes
apply/reset changes
```
