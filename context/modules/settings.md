# Settings Module

## Crate

`crates/modules/settings` → package `shell-module-settings`

## Purpose

Expose shell configuration inside the Notch without making the settings UI the owner of configuration storage.

## Responsibilities

```text
read current validated config
edit supported user-facing options
validate drafts
apply/revert changes
navigate settings sections
```

## Dependencies

Consumes configuration models and commands from `shell-config`/application ports. The module must not parse TOML files directly or mutate files from widgets.

## State

```text
active section
configuration draft
validation state
unsaved-change state
```

## Notch behavior

Settings may use a larger multi-section layout; the Notch owns resizing, focus, dismissal and surface semantics.

## MVP

```text
general settings
theme entry point
module enable/disable controls
key user preferences
apply/revert
```
