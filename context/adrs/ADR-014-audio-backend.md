# ADR-014 — Audio Backend

- **Status:** Accepted
- **Date:** 2026-09-11
- **Decision owners:** Project maintainers
- **Related gates:** `G10`, `G13`

---

## Context

This project is a Rust-first Linux desktop shell targeting Wayland and Hyprland initially. The architecture must support shell-specific behavior without allowing the UI toolkit, compositor, protocol libraries, or Linux service implementations to become the application architecture.

This ADR records one boundary that would be expensive to change implicitly later.

---

## Decision

Target PipeWire-native integration or a robust Rust binding/wrapper for production audio state and control. CLI tools are allowed only inside the adapter during early PoCs.

The implementation must follow the project's `invariants.md`. Any intentional exception requires a superseding ADR or an explicit amendment to this record.

---

## Decision drivers

- Event-driven audio state.
- Shared source for panel, OSD and settings.
- Avoid repeated subprocess spawning.

---

## Alternatives considered

- **Direct PipeWire integration**
- **PulseAudio compatibility APIs**
- **CLI-based wpctl/pactl adapter**

Alternatives are not permanently prohibited. They may be revisited when a gate or measurement invalidates the current assumptions.

---

## Consequences

- Direct PipeWire integration may increase implementation complexity.

---

## Implementation constraints

- Keep infrastructure-specific types at their owning adapter boundary.
- Preserve explicit ownership of state and lifecycle.
- Do not add abstractions solely for theoretical portability.
- Prefer event-driven integration where reliable signals/subscriptions exist.
- Keep failure of optional subsystems recoverable.
- Any performance claim that influences this ADR must be measured.

---

## Validation

This ADR is validated through: `G10`, `G13`.

A gate failure that directly contradicts the decision must reopen this ADR instead of being hidden behind permanent workarounds.

---

## Revisit when

Revisit this ADR when one of the following is true:

- a mandatory gate fails because of this decision;
- production behavior exposes a correctness, security, or lifecycle problem;
- the maintenance cost becomes materially higher than an available alternative;
- measured performance shows the decision is a bottleneck;
- the project's supported platform scope changes.

Do not revisit it merely because another implementation is aesthetically cleaner.

---

## Rationale

Target PipeWire-native integration or a robust Rust binding/wrapper for production audio state and control. CLI tools are allowed only inside the adapter during early PoCs.

The goal is to keep the shell architecture explicit, testable, and replaceable at its real boundaries without turning the project into a generic framework.
